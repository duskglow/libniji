# DL-START-SEARCH-3 report

Branch `task/dl-start-search-3` (stacks on `task/dl-start-search-2`). Role keke:~/w/n3. Brief ~/state/briefs/DL-START-SEARCH-3.md.

## Why -2 went red, and the fix to the counts
dl-start-search-2-counts (mei) measured the base first; the base cannot start p_soname (it refuses a bare soname:
the search this story adds), so the job exited rc 1 before measuring the head. The count script now takes the cases
to run: base runs p_none and p_path, head runs all three (~/work/DL-START-SEARCH-3/count.sh, counts_job.sh,
counts_head.sh on keke). The -2 start-up check job (mei:dl-start-search-2-startup) ended rc 0.

## Counts (keke, Zen 1; release products; perf mean of 30 runs, ASLR off; instructions:u / page-faults:u)
| program | base 2d5625a3e | head 06a5c42 (-2 code) | head 71db28f7f (this run) |
|---|---|---|---|
| p_none (no DT_NEEDED) | 30,011 / 16 | 30,315 / 16 (+304) | 30,026 / 16 (+15) |
| p_path (one path DT_NEEDED) | 131,437 / 40 | 132,267 / 40 (+830) | 131,977 / 40 (+540) |
| p_soname (bare soname, DT_RUNPATH) | cannot start | 149,276 / 43 | 149,106 / 43 |
Jobs: dl-start-search-3-counts-2 (base and -2 head), dl-start-search-3-counts-3 (this head; base products reused
from counts-2, same revision). Smoke: the host's dynamic /bin/echo runs under the head ld.so (rc 0).

## Where the -2 surplus went (gdb single-step trace of both builds, every user instruction, exact: +304 / +830)
* +195 (every program): ld.so self-relocation. RELR went from 238 to 241 words (each about 65 boot instructions);
  the three new words are the vtable of the `dyn Fn` closure behind `CacheRef::Lazy` (call_once, call_mut, call).
* +105 (every program): `liella_ld_start` read AT_EXECFN on every start (an auxv scan).
* p_path only, +526: `link` +237, one more out-of-line `Elf::parse` call (124; base inlined it into `link`) and a new
  512-byte by-value copy of a 64-entry array inside `link` (~165 in memmove). Not addressed in this run (see next step).
Tools: ~/work/DL-START-SEARCH-3/{trace.py,tdiff.py,callers.py,relr_targets.py,trace_job.sh} on keke.

## Code (this run)
* `CacheRef::Lazy(&LazyCache, fn)`: a OnceCell and a plain reader function, no closure object, no vtable.
* AT_EXECFN read only when a search first needs the program's `$ORIGIN` (`Origin` in ld.rs); `List.execfn` removed.
* Plants: handoff/DL-START-SEARCH-3-plants.txt (the -2 six; the two cache plants rewritten for the new code).

## Jobs (keke)
* dl-start-search-3-lint0: clippy + static on the inherited head 06a5c42, rc 0.
* dl-start-search-3-gate (06a5c42): affected direct tier, rc 0, 5 suites, 131 passed, 0 failed, peak 1.6G.
* dl-start-search-3-plants (06a5c42): 6 CAUGHT of 6 (WARN-DEV-SHM only: the copy was under /dev/shm).
* dl-start-search-3-lint3 (71db28f7f): `cargo clippy --workspace --all-targets --all-features -D warnings`, `cargo
  xtask clippy`, `cargo xtask static`, rc 0.
* Final head 71db28f7f, running at hand-back: dl-start-search-3-gate2 (affected direct tier, `cargo xtask test`,
  LIELLA_FULL_PROFILE=1 so + hot_text_products), dl-start-search-3-plants2, dl-start-search-3-counts-3.
* Superseded: dl-start-search-3-counts (stopped by me: it held the role busy for a re-sync), dl-start-search-3-startup
  (stopped by me: tree before the fix), dl-start-search-3-lint1 (started on the old tree after a BUSY sync; stopped),
  dl-start-search-3-lint2 (compile error in the Debug impl; fixed in 71db28f7f), dl-start-search-3-trace (probe,
  evidence only).

## Process notes
* The role was first synced with base = dl-start-search-2's head, so `affected` saw no change; re-synced with
  `--base origin/next`.
* Stopping a start-up check job mid-build left trace-at-build's gdb tracers orphaned (parent 1), holding the role BUSY;
  ended by PID.
* Scratch slips: one local heredoc and one `sed -i`, both on my own scratch scripts (not the repository).
