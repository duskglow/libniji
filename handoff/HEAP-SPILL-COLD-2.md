# HEAP-SPILL-COLD-2: handoff and report

State: branch `task/heap-spill-cold-2`, base origin/next `5a9b5d0a1`; commits `b55385e46` (the change), `e69555d58`
(clippy), then this note and `handoff/reports/HEAP-SPILL-COLD-2.json`. Build role mei `~/w/s3` (Skylake-SP);
probes also on keke (Zen 1) under `~/work/heap-spill-cold-2/` (`mig.sh <niji dir> <oracle.c> <out> [runs]`; base and
head `libc.a` in `base-niji/`, `head-niji/`). Decision: D-3.23 (amends D-3.12, D-3.22). Ruling in force: #7441.

## Done: the node-migration part (ruling #7441)

1. `the_heap_keeps_address_space_near_its_use` runs its oracle child pinned to the first NUMA node's allowed CPUs
   (test thread's affinity, inherited; restored after); comment names it a single-node measurement and cites
   jobs test-scratch-cleanup-4-heaprep2 / -heapmig. Bounds and `AS_HEADROOM` (41296) unchanged.
2. Measured (`heap_address_space.c` gained `- | <limit> CPU AT`: move to CPU at point AT, 1 after malloc1, 2 mid-alloc,
   3 after alloc, 4 after refill; fixed layout; 3/3 runs; Zen 1 and Skylake-SP identical). Base:

   | move at | alloc | refill | realloc (VmSize over start, KiB, no limit) | least RLIMIT_AS over start |
   |---|---|---|---|---|
   | none | 28996 | 39236 | 41288 | 41296 |
   | 1 | 33348 | 43588 | 45640 | 45648 |
   | 2 | 29252 | 39492 | 41544 | 41552 |
   | 3 | 28996 | 39492 | 41544 | 41552 |
   | 4 | 28996 | 39236 | 45640 | 45648 |

   Cause: a second node's segment. The node with no segment maps `SEG_MIN` (256 KiB metadata + 16 slabs = 4352 KiB)
   while the first node's shard still holds never-used (point 1) or empty (point 4) slabs; mid-run (2, 3) only the
   second segment's metadata (+256) is extra. Not a spill, not a reservation elsewhere.
3. Change (D-3.23): under an `RLIMIT_AS`, `new_segment` maps nothing while another shard holds an empty or
   mapped-never-used slab (`held_elsewhere`, cold, out of line, lock-free hint); `refill`'s existing
   `take_elsewhere` takes it. Without a limit placement stays per node (address space only, no touched page; keeps
   NUMA locality). Head, least RLIMIT_AS, identical on both CPU classes (jobs mei:heap-spill-cold-2-so2,
   keke:heap-spill-cold-2-mig-zen-head; base: heap-spill-cold-2-mig-sky, keke:heap-spill-cold-2-mig-zen): 43600,
   41552, 41552, 41296 at points 1..4; the unlimited VmSize steps unchanged (by design). Left: the second node's metadata (+256, per-node segments) and at point 1 its new segment's 16 slabs
   against the first node's 8-slab growth step (+2048). Removing that needs a smaller first mapping for a second
   node's segment under a limit (touches `take_slab`'s `SEG_FIRST` store, inlined in `refill`, bin 0): not done.
4. Test `the_heap_bounds_address_space_after_a_node_change` (core products): per point, the unlimited steps' VmSize
   bounds and the run under its measured need; skips with why when the affinity mask spans one node. rt test
   `under_an_address_space_limit_a_moved_thread_takes_held_slabs_before_a_new_segment` (never-used and empty slabs,
   with and without a limit).

Issue #21 formula (find under `ulimit -v`, single-threaded): a move of `find` between nodes mid-run raised its need by
up to 4352 KiB before; under the limit it is now at most +2304 by the probe's points (+256 when it moves while
growing). Not measured with `find` itself (estimate from the probe, measured: false).

## Not done: the spill (the story's original part)

No spill was built in this run: the coordinator's migration part took the budget. Analysis from the code (NOT
VERIFIED, no measurement): `alloc_small`/`alloc_hint` stride by the constant `CLASSES[c]`, so a spilled request can
only be served while class c's `K_CUR` stays 0, i.e. every spilled `malloc` runs fast miss -> `alloc_group` miss ->
`alloc_slow` -> `refill` (hint scan, collect, own shard lock, `take_held`) before the spill decision; and a decision
kept in the grow path never fires in a program whose heap fits a segment's first 16 slabs (the kits' case per
HEAP-CLASS-SPACING's slab counts). Next run: build the prototype, measure shift.c / the probe / find and one kit pass
per CPU class, base vs head, before deciding.

## Next step

Coordinator: land the migration part if the jobs in the hand-back are green; re-brief the spill (HEAP-SPILL-COLD-3)
with the per-spilled-malloc slow-path cost above as its first measurement.
