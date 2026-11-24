# HEAP-SECOND-NODE-MAP: handoff and report

State: branch `task/heap-second-node-map` (base origin/next `279a24ee4`): `93b352f3a` (the change),
`d9502cf5d` (test bounds, D-3.24, plants file), then this note and `handoff/reports/HEAP-SECOND-NODE-MAP.json`.
Prototype branch (NOT for landing): `task/heap-second-node-map-spill` = `93b352f3a` + `0c97a608a`.
Build role mei `~/w/s1` (Skylake-SP, two nodes). Scratch on mei: `~/work/heap-second-node-map/` (`save.sh`: one
job building base, prototype and head in the role with the cbench_malloc counters after each; `mig.sh`: the #21
probe per move point with a 16 KiB RLIMIT_AS bisection; `spill_cost.c`: user instructions per malloc in the probe's
refill phase per chunk of 1000, with how many came from a larger class). Products per build in `base/`, `spill/`,
`head/` there (`mig.txt`, `spill_cost.txt`, `bench.txt`, `layout-bins.tsv`). Decision D-3.24 (reserved id).

## (a) A later segment maps one growth step (landing)

`new_segment` maps the first reservation's segment as before (`SEG_FIRST`, 16 slabs) and every later segment
(a second node's, or a node's next one) with its metadata and `SEG_STEP` = 8 slabs (the growth step, now named
and used by `grow_segment` too), through one cold, out-of-line `map_later_segment` (which also makes the
`held_elsewhere` check); it returns the slab count and `take_slab` stores it. With and without a limit: the cost
is at most one more grow (one system call, no touched page) per later segment.

The #21 probe, Skylake-SP, fixed layout, 3 of 3 runs, KiB over start (job heap-second-node-map-base):

| move to node 1 | least RLIMIT_AS base | head | unlimited alloc / refill / realloc base | head |
|---|---|---|---|---|
| none (one node) | 41296 | 41296 | 28996 / 39236 / 41288 | same |
| after malloc1 | 43600 | **41552** | 33348 / 43588 / 45640 | same |
| halfway through alloc | 41552 | 41552 | 29252 / 39492 / 41544 | same |
| after alloc | 41552 | 41552 | 28996 / 39492 / 41544 | same |
| after refill | 41296 | 41296 | 28996 / 39236 / 45640 | 28996 / 39236 / **43592** |

What is left after a node change: the second node's metadata (+256). The single-node bound (`AS_HEADROOM`
41296) is unchanged: no margin appears (its second segment is mapped while the heap grows anyway).
The test `the_heap_bounds_address_space_after_a_node_change` now holds 41552 at point 1 and 43592 for point 4's
realloc. Zen 1 not measured in this run (HEAP-SPILL-COLD-2 found both classes identical for this probe).

cbench_malloc instr/op (`bench --compare --counters --runs 3`, same role, base -> head): malloc_free_* 122.1 ->
122.2, lifo_batch64 8339.2 -> 8343.6, realloc_grow_16step 5722.3 -> 5725.5, realloc_grow_x2 1748.3 -> 1748.7,
heap_mix_real 148.6 -> 148.7, calloc_free_4096 178.2 -> 178.0. The prototype build carries the same change and
reads 122.1 on the malloc_free rows, so the +0.1 there does not follow from this change's code (no fast-path
instruction changed); not proved further. Layout bin 0: `refill`
3922 -> 3899 B, `alloc_slow` 1280 -> 1299 B, no other heap section changed.

## (b) Spill prototype (measured, NOT landing)

A malloc (alignment <= 16) whose class has no free block and no partial slab, in a shard with no empty or
never-used slab, takes a free block of a class at most a quarter larger from the thread's cache (`try_spill`,
cold, from `alloc_slow`). Measured on the same host:

- #21 probe, least RLIMIT_AS over start: one node 41296 -> **34896** (under the issue's 35152 target), moves
  35408 at points 1-3, 34896 at 4; unlimited refill step 39236 -> 32836, realloc 41288 -> 34888.
- Cost per spilled malloc: 372.9 user instructions (51 chunks of 1000 all spilled) against 55.1 for an unspilled
  malloc in the same phase (55.1 on base): about +318 each; 51253 of the 100000 refill mallocs spilled.
- Cost per refill: lifo_batch64 8339.2 -> 9086.6 instr/op (+9.0%): the check runs on every slow-path entry.
  The malloc_free fast-path rows did not move (122.1).
- `find` (the issue's real workload) allocates one size range (257..272 B; HEAP-SPILL-COLD), so a spill would not
  change its need; the gain is the probe's mixed sizes only.

Recommendation: do not land. The address-space gain is real for mixed-size refills, but it costs +9% on a refill
row and ~7x per spilled malloc. A landable form would need the check off the refill path (e.g. only once the shard
has no held slab, latched per cache) and a spill route that does not re-enter `alloc_slow` per block.

## Not done

- The #21 formula with `find` under `ulimit -v` on the packaged build (RELEASE-SNAPSHOT-1122C's y21.sh needs .debs
  of the branch, `debs.sh`): not run (the host's queue took 47 min of this run before the first build). By the
  probe, single-node needs are unchanged and a node change costs up to 2048 KiB less; for `find` that is an
  estimate (measured: false), NOT VERIFIED.
- Zen 1 numbers for (a).

## Results on the final head

See the hand-back's JOBS: gate job heap-second-node-map-gate (clippy, static, direct-tier `affected --run`,
`cargo xtask test`, full-profile `so` + `hot_text_products`), plants heap-second-node-map-plants (4 plants).

## Next step

Coordinator: land `task/heap-second-node-map` if its jobs are green. Leave `task/heap-second-node-map-spill`
unmerged (evidence only); a spill story, if wanted, starts from the recommendation above.
