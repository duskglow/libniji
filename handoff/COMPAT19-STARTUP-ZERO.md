# COMPAT19-STARTUP-ZERO handoff

State: COMPLETE. Branch `task/compat19-startup-zero` (cut from origin/next df83365cd), code head 1a51abd35 (later
commits: handoff/plants/report only). Role mei:~/w/s4. Brief: ~/state/briefs/COMPAT19-STARTUP-ZERO.md. Decisions:
D-27.66 (amended), COMPAT-19 record (`cost`, #6944 note: a failed `dlopen` set leaves stacks already made
executable as they are).

## Design (in force)
* A normal start reads nothing for COMPAT-19. ld.so's `run_init` reads `EXEC_STACK` only in the `else` of
  `if let Some(f) = early` (a start without libniji's C library). With libniji's libc, `early_init` checks
  `t.exec_stack == Some(true)` only inside the `GLIBC_TUNABLES`-is-set closure. The cold, argument-less
  `exec_stacks_at_start` then walks the start objects through the loader's walk (`LOADER_ITERATE`, not the exported
  `dl_iterate_phdr`). When one of them asks, it makes the main stack executable (`thread::exec_main_stack`, shared
  with ld.so) and calls `thread::stacks_executable()`. The dlopen path is unchanged.
* Why not in the loader's PT_GNU_STACK branch (brief step 1): at start that branch runs before libc is mapped and
  relocated, so libc's thread flag cannot be set there, and any deferred action needs a check on the normal path. The
  tunables closure is the existing branch nearest to "only when opted in".
* The count found two traps (leg 2), both fixed: (1) calling the exported `dl_iterate_phdr` from libc adds a
  GLOB_DAT relocation that every dynamic start resolves (+1,409 instructions empty main, +1,736 three libs); (2)
  passing the borrowed `Auxv` into the cold function made early_init spill it on every start (+~4).

## Results (mei, Skylake-SP, ASLR off, median of 101; base = origin/next df83365cd, both shipped profile)
* count4 (head 1a51abd35), instructions: pie0 740,749 -> 740,745; pie3 996,291 -> 996,287; clock_gettime
  774,759 -> 774,755; static empty main 2,048 = 2,048; faults 52/67/56/7 unchanged. No row is worse. The Zen 1
  class was not counted (budget).
* check4: clippy -D warnings, xtask static, shipped-profile so, `exec_stack` tests 5/5: green.
* startup (cargo xtask test 6,953 passed / 0 failed; NIJI_FULL_PROFILE so + hot_text_products 32/32): green.
  The shipped-profile `target/product/release/layout-bins.tsv` is identical to base df83365's.
* plants3: 5 test plants CAUGHT, 0 WARN (job rc 1 only for the count plant, which survived because its fixed
  threshold did not fit the copy's ungrouped layout). plants4 (self-relative count script): CAUGHT, "pie0 grew by 4"
  (736,960 vs 736,956; baseline equal). plants5 (the five test plants on the final code): 5 CAUGHT, 0 WARN.
  plants3 is SUPERSEDED by plants4 + plants5. Results are in handoff/COMPAT19-STARTUP-ZERO-plants.txt.
* gate (direct, --base origin/next): 14 suites, 10,721 passed, 0 failed; conformance 32/32; static ok; peak 13G.
* Superseded jobs: -prep/-check/-count1, -plants (rc 3), -plants2 (killed), -check2/-count2 (the GLOB_DAT),
  -check3/-count3 (the spill), -prof/-prof2 (perf throttled).

## 1120C bisect (#7046): attributed to COMPAT19-TUNABLE (efe95309f), not removed here (RULING NEEDED)
* Bisect (1120C start-up kit, median of 31): 3f2a84520 785,881 / 1,037,330 / 819,821; efe95309f 788,413 /
  1,043,753 / 822,416 (+2,532 / +6,423 / +2,595, the 1120C figures exactly). 17eda37a8 (LD-SPLIT-3), c47f3d49c
  and b732d9b46 equal efe95309f. Relocations (2065) and faults are unchanged. The cost is in ld.so: about 1.3k
  per object searched plus about 1.2k fixed. Not split per function: perf is throttled at this rate and mei has
  no valgrind. Whether origin/next still pays it is unknown.
* Candidate (unverified): every start dependency is fully checked twice, by `niji_ld::dlopen::check` in
  `image::open_candidate` and by `map::load`'s plan in `link.rs`.

## Next step
* None for this story; the coordinator lands it. Follow-up story recommended: a per-function split of ld.so's start
  (gdb single-step trace of `pie3` from the efe95309f and 3f2a84520 products in mei
  `/mnt/zram/claude/compat19-startup-zero/bisect/`). Then remove the per-object cost, measuring origin/next first
  with the candidate check replaced in a scratch copy.
* Scripts: mei `~/work/compat19-startup-zero/` (check.sh, count3.sh, bisect.sh, plantcount.sh, startup.sh).
