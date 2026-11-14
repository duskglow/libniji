# TB-FOREIGN-WORDS-LOW: errno, TID, dtor, heap and node words (0x80-0x9b) in a block Liella did not make

State: branch `task/tb-foreign-words-low-14` (= -13's branch + a merge of origin/next 5d8cb467c + two commits), head:
see handoff/reports/TB-FOREIGN-WORDS-LOW-14.json (code head e3c932b74). Decisions in force: D-X.58, D-X.60, D-X.62,
D-3.20 (from next), rulings #4835, #4849, #4851, #4779, #4889, #5325, #5378, #4347, #5697, #5735 (b), and the owner
ruling of 2026-11-14 (errno cost accepted for safety). The owner's safety requirement (words 0x80-0x9b) was finished
in -1..-10; -12 fixed the merged-build layout cost; -13 settled the limits. Older reports:
handoff/TB-FOREIGN-WORDS-LOW-report.md (-1..-4), and the git history of this file (-5..-13).

## -14 (merge run; gates and plants on mei role s2, counts on keke)
- CI's -13 red was a merge conflict with HEAP-FIRST-USE-8 (lazy first-use heap, D-3.20) in crates/boundary/src/heap.rs,
  crates/rt/src/heap.rs, crates/rt/tests/heap_and_sync.rs. Merge 5c32da8fe keeps both sides: `Pages` has this branch's
  `General` source and next's `allowed_nodes`; `cache_slot` stays `Option` (D-X.60); `first_cache` returns on no cache
  word first, then takes the shard through `shard_of`; `FakePages` keeps the counted `allowed_nodes` and the `Option` slot.
- Found in the merge: `OwnPages` (OWN_CTX: memalign family, malloc_compat paths) reaches `alloc_slow`/`shard_of`, and
  the trait's default `allowed_nodes` = 1 would have answered for it. Since the topology is decided once per heap, a
  process whose first slow path came that way would keep NUMA placement off on a multi-node host. Fixed in the merge
  (OwnPages forwards it) and hardened in 705c47fd2: `allowed_nodes` is now a required method (NoPages in
  s01_startup_clauses answers 1), so a missing forward is a compile error.
- Counts on keke (Zen 1, CI's instruction host; RAM copies /dev/shm/claude/TB-FOREIGN-WORDS-LOW-14/{next,merged}, script
  keke ~/work/TB-FOREIGN-WORDS-LOW-14/cnt.sh): startup_instr_c next 5d8cb467c 2144, merged e3c932b74 2155 (+11, the
  same as -13 measured on e47a71997), so the limit stays 2161 (2150 + 11); errno_location next 9.0, merged 11.0
  (glibc 10.0), limit 12 as ruled. No limit changed.
- Plants (handoff/TB-FOREIGN-WORDS-LOW-14-plants.txt, job `tb-foreign-words-low-14-plants`): 6 of 6 CAUGHT, no WARN:
  errno round-trip in a foreign block, no read or write at 0x90 there, spare words hold no cache (the heap takes the
  slow path), and one per merge site (next's ungated cache slot, first_cache's shard through shard_of, a missing
  OwnPages forward).
- Jobs (mei): `-lint0` (step 0, inherited head) rc 0; `-lint2` (workspace clippy, xtask clippy, static, records check
  on e3c932b74) rc 0; `-gate` (heap_and_sync, `cargo xtask affected --run --base origin/next`, which escalates to FULL
  as in -13 and runs `cargo xtask test`, then LIELLA_FULL_PROFILE=1 so + hot_text_products) rc 0: heap_and_sync
  27/0, `cargo xtask test` 32 suites, 11807 passed / 0 failed, conformance 137/0, hot_text_products 26/0, peak 16G
  (80 min on mei at load 23-45, set by the FULL escalation).
  keke: `-kekenext`, `-kekemerged` rc 0. SUPERSEDED: `-lint` (stopped, rc 124: its tree predated 705c47fd2).

## Next step
Land. Still open from -10, outside this story: the text before the 32 KiB-aligned string kernels is about 9 KiB over
the boundary next sits under (LAYOUT-RETHINK's question); next's startup_instr_c (2144) is 6 under its own old limit.

## Not to redo
-1's audit; the registry/FOREIGN/OwnPages/asm design; -7's SPARE split; -8's Haswell-EP comparison; -10's mei
fault-set comparison; -11's keke page lists; -12's tunable-table diagnosis (fix a049c2f4f, `#[inline(never)]` on
`glibc_tunable_type` and `warnings`; scripts mei ~/work/TB-FOREIGN-WORDS-LOW-12/); -13/-14's keke counts.
