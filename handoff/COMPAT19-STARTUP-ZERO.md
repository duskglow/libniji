# COMPAT19-STARTUP-ZERO handoff

State: branch `task/compat19-startup-zero` (cut from origin/next df83365cd), code head 1a51abd35 (later commits:
handoff/plants only). Role mei:~/w/s4. Brief: ~/state/briefs/COMPAT19-STARTUP-ZERO.md. Decisions: D-27.66 (amended),
COMPAT-19 record (`cost`, #6944 note: a failed `dlopen` set leaves stacks already made executable as they are).

## Design (in force)
* A normal start reads nothing for COMPAT-19. ld.so's `run_init` reads `EXEC_STACK` only in the `else` of
  `if let Some(f) = early` (a start without libniji's C library). With libniji's libc, `early_init` checks
  `t.exec_stack == Some(true)` only inside the `GLIBC_TUNABLES`-is-set closure, then the cold, argument-less
  `exec_stacks_at_start` walks the start objects through the loader's walk (`LOADER_ITERATE`, not the exported
  `dl_iterate_phdr`) and, when one asks, makes the main stack executable (`thread::exec_main_stack`, shared with
  ld.so) and calls `thread::stacks_executable()`. dlopen path unchanged.
* Why not in the loader's PT_GNU_STACK branch (brief step 1): at start that branch runs before libc is mapped and
  relocated, so libc's thread flag cannot be set there; any deferred action needs a check on the normal path. The
  tunables closure is the existing branch nearest to "only when opted in".
* Two traps found by the count (leg 2), both fixed: (1) calling the exported `dl_iterate_phdr` from libc adds a
  GLOB_DAT relocation every dynamic start resolves (+1,409 instr empty main, +1,736 three libs); (2) passing the
  borrowed `Auxv` into the cold function made early_init spill it on every start (+~4, cancelling ld.so's -4).

## Results (mei, Skylake-SP, ASLR off, median of 101; base = origin/next df83365cd, both shipped profile)
* count4 (head 1a51abd35): pie0 740,749 -> 740,745; pie3 996,291 -> 996,287; clock_gettime 774,759 -> 774,755;
  static empty main 2,048 = 2,048; faults 52/67/56/7 unchanged. No row worse. Zen 1 class not counted (budget).
* check4: clippy -D warnings, xtask static, shipped-profile so, `exec_stack` tests 5/5: green.
* plants3 (`compat19-startup-zero-plants3`, head 0834d6584): 5 CAUGHT, 0 WARN; the count plant SURVIVED for a
  tooling reason: in the plants copy the products are built layout ungrouped (empty main 736,960 instr, 59 faults),
  so its fixed 740,745 threshold was meaningless. `~/work/compat19-startup-zero/plantcount.sh` is now
  self-relative (counts the copy as given, then with the role's unplanted initfini.rs, fails when higher);
  `~/work/compat19-startup-zero/count-plant.txt` holds that one plant line. NOT yet rerun.
* Superseded jobs: compat19-startup-zero-prep/-check/-count1 (leg 1), -plants (file not synced, rc 3), -plants2
  (killed: old head), -check2/-count2 (head bbc88c3c3: +1,409, the GLOB_DAT), -check3/-count3 (f63ad8738: = base,
  the spill), -prof/-prof2 (perf throttled, no usable split).
* `compat19-startup-zero-startup` (started 04:35Z on the role, ~/work/compat19-startup-zero/startup.sh):
  affected --list said `tier: Direct`; then cargo xtask test, shipped so + hot_text_products, layout-bins diff vs
  base df83365 (~/wl/src/df83.../target/product/fast/layout-bins.tsv). Still running at 05:05Z.

## 1120C bisect (#7046): attributed to COMPAT19-TUNABLE (efe95309f), not removed here
* bisect (same kit, 1120C start-up progs, median of 31): 3f2a84520 785,881 / 1,037,330 / 819,821; efe95309f
  788,413 / 1,043,753 / 822,416 (+2,532 / +6,423 / +2,595, exactly the 1120C figures); 17eda37a8 (LD-SPLIT-3),
  c47f3d49c (LAYOUT-GOT-ORDER) and b732d9b46 equal efe95309f. Relocations (2065) and faults unchanged: the cost is
  in ld.so, about 1.3k per object searched plus about 1.2k fixed.
* Not split per function: perf sampling at this rate is throttled (per-symbol diffs +-50 samples vs ~6,900
  expected) and mei has no valgrind. Unknown whether origin/next still pays it.
* Candidate (unverified): each start dependency is fully checked twice, `niji_ld::dlopen::check` in
  `image::open_candidate` (the search) and `map::load`'s plan in `link.rs`.

## Next step (leg 3, in order; deadline 07:18Z)
1. `~/bin/jobwait compat19-startup-zero-startup`; check `grep -E "^== |layout-bins|startup: done|FAILED"` and
   `~/bin/test-summary` of its log. Red: fix, rerun only that part.
2. Then (role idle) the count plant: `setsid nohup ~/bin/jobrun compat19-startup-zero-plants4 plants -- sh -c ". ~/.cargo/env;
   cd ~/w/s4 && cargo xtask plants ~/work/compat19-startup-zero/count-plant.txt --dir
   /mnt/zram/claude/compat19-startup-zero/plants-4 --timeout 1800" >/dev/null 2>&1 </dev/null &` (two builds per
   run). Expect CAUGHT with "grew by 3" or so (ld.so disassembly: movzbl/test/jne on the path).
3. After plants4 has copied the role: the direct gate, `setsid nohup ~/bin/jobrun compat19-startup-zero-gate gate --
   sh -c '. ~/.cargo/env; ~/bin/gate 16G sh -c "cd ~/w/s4 && cargo xtask affected --run --base origin/next"'
   >/dev/null 2>&1 </dev/null &`.
4. `cargo xtask precheck --jobs <labels> --head <sha>` on the role; report json handoff/reports/COMPAT19-STARTUP-ZERO.json;
   `ops handback-check`; hand back. JOBS: check4, count4, plants3, plants4, startup, gate, bisect (all mei).
   RULING/INFORMATIONAL to report: the bisect attribution below; Zen 1 class not counted.

## Follow-up (not this story)
* A follow-up story: per-function split of ld.so's start (gdb single-step trace of `pie3` from the efe95309f and
  3f2a84520 products in mei `/mnt/zram/claude/compat19-startup-zero/bisect/`), then remove the per-object cost;
  measure origin/next first by replacing the candidate check in a scratch copy.
* Jobs and scripts: mei `~/work/compat19-startup-zero/` (check.sh, count3.sh, bisect.sh, plantcount.sh, startup.sh).
