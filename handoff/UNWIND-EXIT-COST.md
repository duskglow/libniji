# UNWIND-EXIT-COST (handoff; leg 2 = code, measured, gated)

State: branch `task/unwind-exit-cost` (from origin/next 0495c4684), role keke:~/w/n2 (Zen 1 host, 60 vCPU).
Code: `crates/rt/src/unwind.rs` (walker), one test case in `crates/rt/tests/unwind.rs`, pending record
`records/pending/UNWIND-EXIT-COST-1.toml` (D-NEW, series D-22: CI assigns the id). Plants:
`handoff/UNWIND-EXIT-COST-plants.txt`. Scratch (probes, measure.sh, step.py, gate.sh): `~/w/.scratch/UNWIND-EXIT-COST/`,
installed on keke in `~/work/unwind-exit-cost/`.

## What changed (D-NEW in records/pending, body has the detail)
1. `.eh_frame_hdr` table in datarel|sdata4 (0x3b, what linkers write): the table is taken once as a slice that must
   fit its segment; fields read straight from it (`last_at_or_below` shared with the generic path, kept for other
   encodings). A count running past the segment finds nothing (new case in `ip27_finds_fdes_through_the_header...`).
2. `run`: remembered rows stored lazily (`[Option<Row>; 4]`), not 4 row copies per call; `restore` no longer builds
   `Row::new` eagerly (`map_or` -> `match`).
3. `walk_with` generic over `O: Borrow<Object>` (listed objects borrowed; `Held` enum for `handler_above`'s found
   object); `row_at` and `step` write in place; two register buffers swapped by reference.
4. `LastCie`: the CIE parsed last is kept across the walk (keyed by address), shared by search and scan.

## Measurements (keke, user instructions per operation, delta = minus a returning thread; measure.sh)
| row | reference | libniji before (base0) | after round 1 (after1) | after (after2) |
|---|---|---|---|---|
| pthread_exit depth 0 | 8,825 | 57,115 | 35,139 | 30,980 |
| pthread_exit depth 10 | 21,325 | 97,321 | 61,816 | 52,340 |
| pthread_exit depth 10 + cleanup_push | 79,226 | 124,468 | 88,965 | 80,157 |
| pthread_cancel depth 3 | 14,065 | 70,106 | 43,910 | 38,161 |
| C++ exit 10 dtor frames (libgcc unwinder) | 76,957 | 96,125 | 96,046 | 96,036 |
| C++ throw 10 frames | 86,445 | 98,986 | 98,993 | 99,010 |
| static pthread_exit depth 3 | 17,344 | 129 | 129 | 128 |
Round 1 = fixes 1-3 without in-place rows/regs and CIE keep. Step profile after round 1 (step.py, depth 0): walker
~30k of the exiting thread's ~55k steps; `thread::finish` 19.5k (teardown, also in a returning thread). The C++
rows run libgcc's unwinder, not this code (their drift: see the report's noise check). Skylake-SP: not measured
(no Skylake-SP role was assigned; keke only).

## Candidates from the brief
* (a) per-object index: already there (binary search of PT_GNU_EH_FRAME's table); the cost was constant factors
  (generic per-row decode, segment lookup per row): fixed by 1.
* (b) skip the walk when no frame can have a handler: NOT landed, NOT measured. Needs a per-object "has a
  personality CIE" answer cached across dlopen/dlclose; libc's own .eh_frame (~5,000 records, ~260 KB) has
  personality CIEs for its guards (skip list), so libc would need a build-time answer. Estimated gain ~25k per
  plain-C dynamic exit; cost a one-time scan of the exe's (small) and other objects' records. Recommended as a
  follow-up story with its own measurement and the brief's "shortcut taken with a cleanup frame present" plant.

## Jobs (keke) on the final code
unwind-exit-cost-chk4 (xtask clippy, unwind tests, so, measure after2), unwind-exit-cost-plants2 (7/7 CAUGHT),
unwind-exit-cost-gate1 (direct tier, 7 suites, 265 passed, 0 failed), unwind-exit-cost-static1 (records check,
static, measure after3). Superseded: chk1/chk2 (per-package clippy: failed on unrelated niji-boundary cfg lints),
chk3 + after1 + step1 (round 1 head), plants1 (stopped: round 1 head).

## Next step
Land. Follow-ups: (b) as its own story; `thread::finish` 19.5k (THREAD-CREATE-COST's area); the remaining walk
(~2.6k/frame: FDE instruction interpretation in `run`, CIE instructions re-run per frame).

## Decisions / rulings in force
D-22.19, D-22.21, D-22.23 (the walk), ruling #6904 (lookup stays uncached), D-NEW (this story, pending id).
