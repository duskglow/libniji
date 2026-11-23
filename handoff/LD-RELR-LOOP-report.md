# LD-RELR-LOOP report

**Result:** a dynamic start-up is about 25,100 user instructions shorter (Zen 1, release, median of 101: empty
main 268,837 -> 243,688, three libraries 378,132 -> 352,952, `/bin/true` 329,222 -> 304,186), one page fault
fewer, and libc.so.6's `.rela.dyn` is 34,584 bytes smaller (35,808 -> 1,224, plus a 504-byte `.relr.dyn`). The
story's target (RELATIVE work + self-relocation <= 15k) is not reached by estimate (~17k; not measured).

## Commits (branch task/ld-relr-loop, base origin/next fd973fc44)
* bdf1d6fa0 Apply RELR entries a run at a time and pack libc.so.6's relative relocations
* bdbdf3c85 Tighten the dynamic start-up bounds after packed relative relocations (D-27.87, plants file)
* handoff and report commits

## Jobs (keke, role n1, Zen 1)
* ld-relr-loop-gate2: direct gate on the final code, 11 suites ok, 950 passed, 0 failed; peak 1.9G.
* ld-relr-loop-plants1: handoff/LD-RELR-LOOP-plants.txt (4 plants), results below.
* ld-relr-loop-startup1: `cargo xtask test`, then release `cargo xtask so` + `hot_text_products`.
* ld-relr-loop-meas-base / ld-relr-loop-meas-new: the counts (STARTUP-REPROFILE kit, products in keke
  `~/work/ld-relr-loop/{base,new}`).
* Superseded: ld-relr-loop-gate1 (only failure: `init_dynamic_empty_main_stays_within_its_instruction_bound`,
  "more than 10% under: tighten", fixed in bdbdf3c85); ld-relr-loop-relmeas (release level for that bound,
  failed the same way by design); ld-relr-loop-prof-base / -prof2 (sampled perf on Zen 1, throttled to ~7% of
  instructions with unreliable symbols: not used).

## Where the remaining cost is (estimate from disassembly and the decoded tables, NOT measured)
* libc.so.6 RELR in `pass1`: 1,441 words at 8 instructions (tzcnt, shift, add, a null-address branch from
  `conv::bytes_at_mut`, the in-place add, clear-lowest-bit, loop) + 63 entries at about 30 = ~13.4k.
* ld.so self-relocation (`add_bits`): 233 words at 13 (the safe slice view's per-word bounds check stays) + 11
  entries at about 40 = ~3.5k.
* Next step: `Image` word-add without the null branch, self loop without the per-word check: ~3-4k.

## Probe-settled / checked
* `readelf -V` libc.so.6: no version needs (the RELR link adds no `GLIBC_ABI_DT_RELR` need); `readelf -d`: RELR,
  RELRSZ 504, RELRENT 8.

## REFUSED
* `. ~/src/liella-ops/tools/vm_ssh.sh; L=~/w/.scratch/LD-RELR-LOOP/xfer; mkdir -p $L; vm_ssh_cmd keke;
  rsync -a -e "$VSSH_E" --exclude libc.a "${VSSH[-1]}:work/ld-relr-loop/base" "${VSSH[-1]}:work/ld-relr-loop/new"
  $L/ && echo pulled; du -sh $L; vm_ssh_cmd mei; "${VSSH[@]}" 'mkdir -p ~/work/ld-relr-loop' && rsync -a -e
  "$VSSH_E" $L/base $L/new ~/w/.scratch/LD-RELR-LOOP/kit/ "${VSSH[-1]}:work/ld-relr-loop/" && echo
  pushed; ...` (copy the release products keke -> local -> mei for Skylake-SP counts and an Intel PT phase
  profile; refused by the auto-mode classifier as unauthorized access). Step dropped: no Skylake-SP counts, no
  PT profile.

## INFORMATIONAL
* `layout-bins.tsv` not diffed against the base: no base copy on the host; it covers libc's hot group only.
* STARTUP-REPROFILE's 47.6k "RELATIVE and table walk" phase included pass1 work that remains, so the brief's
  -50k expectation overstated what RELR alone removes.

# LD-RELR-LOOP-2 (continuation, branch task/ld-relr-loop-2)

**Result:** RELR bitmap words are added in place (one unaligned read-add-write per word, 6 instructions per set
bit in both loops, from 8 in `pass1` and 13 in the self-relocation). A dynamic start is about 5,100 user
instructions shorter than leg 2 (Zen 1, release, median of 101): empty main 243,688 -> 238,655, three libraries
352,952 -> 347,868, `/bin/true` 304,186 -> 299,000; about 30,200 shorter than next fd973fc44 in all. Faults and
system calls unchanged. Story target (RELATIVE + self-relocation <= 15k): about 12k by estimate from the
disassembly (both loops read in `~/work/ld-relr-loop-2/new2-ld.dis` on keke) and the decoded tables; NOT measured
(no Intel PT or valgrind on the Zen 1 host; Skylake-SP not built this run).

## Commits
* 9a96824e1 Add RELR bitmap words in place, one add per word (`Image::add_bits` + default `add_bits_by_word`,
  `ProcImage` override; `boot::Memory::add_bits` + default view loop, `bootstrap::Mapped` override)
* bb4b03995 Tighten the dynamic start-up bounds after in-place RELR adds (D-27.87 extended, plants file)
* c58063588 Keep D-27.87's body on one line (the records check: a body is one table cell)

## Jobs (keke, role n1, Zen 1)
* ld-relr-loop-2-lint0: clippy (workspace, all targets, all features) + static on the inherited head: rc 0.
* ld-relr-loop-2-lint1: the same after the code commit: rc 0.
* ld-relr-loop-2-meas: release build + counts, previous head's products against this commit's (kit above).
* ld-relr-loop-2-bound: `hot_text_products` bound test, release then fast: 238,792 / 248,446 (bounds tightened).
* ld-relr-loop-2-plants1: handoff/LD-RELR-LOOP-2-plants.txt, 4 plants, 4 CAUGHT, no WARN (run on bb4b03995;
  the planted files are identical at the final head).
* ld-relr-loop-2-gate2: direct gate, final code: 13 suites ok, 1,358 passed, 0 failed, peak 2.3G.
* ld-relr-loop-2-lint2: clippy + static on 98c31e490: rc 0.
* ld-relr-loop-2-startup: `cargo xtask test` (7,120 + 4,875 + 128 + 46 passed), release `so`, `hot_text_products`
  (33 passed): rc 0.
* precheck on acd1d6840 with those four jobs: exit 0 (job notes: later commits are prose only).
* meas and bound are measurements on earlier heads with the same code (precheck: SUPERSEDED, not JOBS).
* Superseded: ld-relr-loop-2-gate1 (bb4b03995): 13 suites ok, 1,358 passed, 0 failed, but `cargo xtask static`
  red on a newline in D-27.87's body, which my edit added; fixed in c58063588.

## INFORMATIONAL
* The big-endian branch of `add_bits_by_word` (as of the earlier per-word code) has no test: no big-endian
  target is relocated by any test image.

# LD-RELR-LOOP-3 (continuation, branch task/ld-relr-loop-3)

**Result:** origin/next 8e1837914 merged into the LD-RELR-LOOP-2 head (clean, no conflict); no code change. The
eleven loader tests that failed in CI train 932 pass on the merged head. They reproduce on a merge with next as of
d258cd270 even with libc.so.6 left unpacked, so the RELR change is not their cause; cause not found (below).

## Commits
* 245779614 merge of origin/next (8e1837914) into the LD-RELR-LOOP-2 head
* the handoff/report commits after it (prose only)

## Diagnosis of the CI red (mei, role s2, fast profile)
| build | atexit_owner `net` | job |
|---|---|---|
| this branch + next 8e1837914 | pass (and dlclose, dl_find_object_index, dlopen_concurrent, dlopen_process_objects, runpath_search: 27 passed; `s27_dladdr` ok) | ld-relr-loop-3-repro2, -repro3 |
| loop-2 + next d258cd270 (scratch merge, the next just before 22:17Z) | FAIL `got 'dA aX', want 'dA mainH aX mainC'` | ld-relr-loop-3-repro4 |
| the same, libc.so.6 not packed (so.rs flag removed in the scratch copy) | FAIL, same message | ld-relr-loop-3-repro5 |
* In the failing build, a probe compared each of ld.so's RELR-named words with its file value + load bias at
  run time: 415 words, 0 mismatches. The test program has no DT_RELR (readelf); the helper libraries are built
  by the system compiler without it. So no RELR code but ld.so's (correct) self-relocation runs in that test.
* A probe that looks up `main` with dladdr and `_dl_find_object` after dlopens, also with libc named by path
  like the tests, finds the program in the failing build: not reduced further (budget).
* Reading: what the branch changes on the failing path is ld.so's code layout. The defect is layout-sensitive,
  in the loader's program entry or address tables (LD-OBJECT-ONCE-2 trips it the same way), and was masked by
  next's 22:17Z merge. It may still be latent on next.
* Recipe to rebuild the failing build: a detached worktree at next d258cd270, `git merge
  origin/task/ld-relr-loop-2`, sync to a role, `cargo xtask so && cargo test -p niji-ld --test atexit_owner`.

## Jobs (mei, role s2)
* ld-relr-loop-3-lint0: clippy (workspace, all targets, all features) + static on the inherited head: rc 0.
* ld-relr-loop-3-repro, -repro2, -repro3: the CI-failing suites on the merged tree (first run skipped: no products).
* ld-relr-loop-3-repro4, -repro5, -probe1..7: the diagnosis above (scratch builds, not this head).
* ld-relr-loop-3-lint1: clippy + static green; rc 134 only from a `| head` that broke the affected listing's pipe
  (superseded by lint2). The affected listing: FULL (xtask/src/so.rs), so the gate is the train's.
* ld-relr-loop-3-startup: `cargo xtask test` (every suite, the changed crates included), then
  `NIJI_FULL_PROFILE=1 cargo xtask so` and `hot_text_products`: rc 0 (nextest: 7,153 + 4,877 + 129 + 46
  passed, 0 failed, the ten CI-failing niji-ld tests among them; `hot_text_products` full profile 33 passed).
* ld-relr-loop-3-lint2: clippy + static on the final head (after the prose commits): see the hand-back.
* Plants: none new (no code change this run; LD-RELR-LOOP-2's 4 plants, all CAUGHT, cover the code unchanged).
* Start-up counts: not re-measured this run (the STARTUP-REPROFILE kit is on the Zen 1 host); LD-RELR-LOOP-2's
  numbers stand (the merge changes no RELR code); `hot_text_products`' bound holds (startup job).
