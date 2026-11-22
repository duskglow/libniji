# HEAP-TEST-NODE-PIN handoff

State: branch task/heap-test-node-pin; code commit 6d72bb391 (the test change), later commits are handoff only.
Role keke:~/w/n1 (synced at 265d9fe03, tree identical to the final head's code).

## What the branch does
- `niji::heap_meta_products the_heap_keeps_address_space_near_its_use` pins both of its runs (unlimited, and under
  RLIMIT_AS with a fixed layout) to the CPUs of the lowest NUMA node that has allowed CPUs, read at run time from
  `/sys/devices/system/node` (`numa_nodes` + `with_cpus`, the same helpers as task/heap-spill-cold-2, so one pair
  remains after both merge). On a one-node machine nothing changes. A comment names it a single-node measurement and
  cites test-scratch-cleanup-4-heaprep2 / -heapmig and ruling #7441.
- `assert_on_one_node` (run inside the pinned closure) fails when the thread may run on CPUs of 2+ nodes.
- Bounds (AS_STEPS) and AS_HEADROOM (41296 KiB) unchanged (ruling #7441). The migration case is HEAP-SPILL-COLD-2's.

## Results (final code, oracle sha256 40511152a955c5f4 on both hosts)
- keke heap-test-node-pin-gate2 (direct tier, 1 of 312 targets: heap_meta_products): 5 passed, 0 failed, rc 0, peak 1.7G.
- keke clippy (workspace, all targets/features, -D warnings) rc 0; `cargo xtask static` rc 0 (role at 265d9fe03).
- mei (Skylake-SP, 2 nodes) heap-test-node-pin-mig20: the oracle under RLIMIT_AS = start + 41296 KiB, fixed layout,
  with a mover flipping its affinity until it exits. Flip within node 0 (between CPUs 0-11 and 24-35): 0 of 20
  fail. Control, flip between the two nodes: 17 of 20 fail (exit 12), so the run reproduces the original failure.
- mei heap-test-node-pin-par24b: 20 batches x 24 parallel copies pinned to node 0 (half unlimited with the step
  bounds, half RLIMIT_AS fixed layout): 0 of 480 over a bound or failed.
- keke heap-test-node-pin-plants2 (2 plants: pin-skipped, pin-all-cpus; handoff/HEAP-TEST-NODE-PIN-plants.txt) and
  heap-test-node-pin-gate20 (the target 20 times as the gate runs it): see the hand-back (PENDING if still running).

## Why the flip test uses an in-node flip
The -heapmig flip was an external `taskset` that sets any mask and so overrides the test's own pinning. Once the test
pins itself, the only moves left to the scheduler are within the node; the in-node flip exercises exactly those.

## Superseded
- keke heap-test-node-pin-gate, heap-test-node-pin-plants (head ddb09af42, killed for the helper change).
- mei heap-test-node-pin-par24: the script applied the step bounds to the RLIMIT_AS runs too, which the test does
  not do (it only requires those runs to succeed). All 480 runs exited 0; the 240 flagged were all RLIMIT_AS
  fixed-layout copies, 8 KiB over the unlimited-run bound. Fixed script rerun as par24b.

## Leftover
- An orphaned `xtask affected --run` / `xtask so` tree from the killed gate held keke:~/w/n1 (sync rc 4). Ended
  with `~/bin/kill-mine n1 --kill`.

## Next step
Nothing in this story. If plants2/gate20 are named PENDING, the dispatcher checks them.
