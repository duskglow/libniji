# UNWIND-EXIT-COST (handoff; leg 3 = UNWIND-EXIT-COST-2: merge of next, re-gated, re-measured)

State: branch `task/unwind-exit-cost-2` (continues `task/unwind-exit-cost`; origin/next b35f1f363 merged in),
role mei:~/w/s2 (Skylake-SP). Code: `crates/rt/src/unwind.rs` (walker), one test case in `crates/rt/tests/unwind.rs`,
record D-22.28 (`records/decisions/D-22/D-22.28.toml`). Plants: `handoff/UNWIND-EXIT-COST-plants.txt` (with results).
Scratch: `~/w/.scratch/UNWIND-EXIT-COST{,-2}/`; probes on mei in `~/work/unwind-exit-cost-2/`.

## Leg 3 (this run)
* CI's merge of `task/unwind-exit-cost` stopped on a conflict in `crates/rt/src/unwind.rs`: next made `run` spill
  remembered rows past REMEMBER_DEPTH through `mem` (`run`/`row_at` take `mem`); this branch made the inline remembered
  rows lazy and `row_at`/`step` write in place. Resolved keeping both (merge commit 6bdff5b49): `run<M>(..., mem)`
  with `saved: [Option<Row>; REMEMBER_DEPTH]` plus next's spill and `checked_add`; `row_at<M>(fde, target, mem, &mut row)`.
* Two plants (`remember-wrong-slot`, `remember-not-stored`) re-pointed at the merged text (same mutations).
* Inherited head checked first (clippy + static, job chk0): clean.

## What the branch changes (D-22.28 body has the detail)
1. `.eh_frame_hdr` table in datarel|sdata4 read straight from a slice that must fit its segment.
2. `run`: remembered rows stored lazily, not 4 row copies per call; `restore` builds no row eagerly.
3. `walk_with` generic over `O: Borrow<Object>`; `row_at` and `step` write in place; register buffers swapped.
4. `LastCie`: the CIE parsed last is kept across the walk (keyed by address).

## Measurements (mei, Skylake-SP; release-dyn products of origin/next b35f1f363 and head a22a9ecd2 built by
`validation/wl_prep.sh`; user instructions per operation minus a returning thread; two runs each, spread <= 12)
| row | reference | origin/next | head |
|---|---|---|---|
| pthread_exit depth 0 | 8,802 | 59,558 | 32,074 |
| pthread_exit depth 10 | 21,262 | 101,326 | 54,290 |
| pthread_exit depth 10 + cleanup_push | 79,002 | 127,365 | 80,329 |
| pthread_cancel depth 3 | 14,025 | 72,860 | 39,627 |
| C++ exit 10 dtor frames (libgcc unwinder) | 76,796 | 95,899 | 95,826 |
| C++ throw 10 frames (libgcc unwinder) | 86,270 | 99,079 | 99,079 |
Leg 2's Zen 1 numbers (keke, fast profile): depth 0 57,115 -> 30,980; same shape.

## Jobs (mei) on the final code (a22a9ecd2; later commits touch handoff/ only)
unwind-exit-cost-2-chk1 (workspace clippy -D warnings, static, unwind lib+integration tests, so, probes),
unwind-exit-cost-2-plants1 (7/7 CAUGHT), unwind-exit-cost-2-gate1 (direct tier: 13 suites, 916 passed, 0 failed,
peak 2.9G), unwind-exit-cost-2-counts1 (the table above). chk0 = inherited head 06a312407 (clippy + static).

## Next step
Land. Follow-ups (not started): (b) skip the walk when no frame can have a handler (needs a per-object
personality-CIE answer kept across dlopen/dlclose; libc's own .eh_frame has personality CIEs, so a build-time answer;
estimated ~25k per plain-C exit, unmeasured); a percall bound for dynamic pthread_exit/cancel at the new level (none
exists to tighten); `thread::finish` ~19.5k (THREAD-CREATE-COST's area); FDE/CIE instruction interpretation per frame.

## Decisions / rulings in force
D-22.19, D-22.21, D-22.23 (the walk), ruling #6904 (lookup stays uncached), D-22.28 (this story).
