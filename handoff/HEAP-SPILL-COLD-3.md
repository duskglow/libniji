# HEAP-SPILL-COLD-3: handoff and report

State: branch `task/heap-spill-cold-3` (continues `task/heap-spill-cold-2`; its handoff
`handoff/HEAP-SPILL-COLD-2.md` holds the change itself: D-3.23, ruling #7441). Build role mei `~/w/s3`
(Skylake-SP, two nodes: node 0 = CPUs 0-11,24-35, node 1 = 12-23,36-47). Scratch on mei:
`~/work/heap-spill-cold-3/` (`slots.sh`). Plants file: `handoff/HEAP-SPILL-COLD-3-plants.txt`.

## What this run did

1. Merged origin/next (`8b4031953`) now that HEAP-TEST-NODE-PIN-2 has landed (ruling #7492). The pin story already
   used this branch's `numa_nodes` / `with_cpus`: one pair of helpers, nothing duplicated. The only conflict was where
   `assert_on_one_node` (next) and the migration test (this branch) sit; both kept.
2. Cause of the verify red (plants2 `no-check-products` SURVIVED): `the_heap_bounds_address_space_after_a_node_change`
   took its nodes from the test process's affinity mask, so in a slot inside one node (cpus 24-29) it skipped.
   Fix (test only, same commit): the nodes come from the machine (`node_cpus`: online CPUs per node from sysfs;
   `numa_nodes` now filters that list by the mask). The program starts on the mask's CPUs of one node and moves to a
   CPU of another node: one of the mask's if it has one, else that node's first online CPU that
   `sched_setaffinity` accepts (`may_run_on`), only for the moved part of a short run. Job slots on the build hosts
   are taskset masks (no cpuset controller in the cgroup tree), so the move is always allowed there. The test skips
   only on a machine with one node, or under a cgroup cpuset that refuses every CPU of the other nodes; the skip
   note says which. No seam in the shipped library: node choice still comes only from the CPU the thread runs on.
3. Proof that it no longer depends on the slot (job heap-spill-cold-3-slots, `slots.sh`): under `taskset -c 24-29`
   (node 0 only) the test starts on 24-29 and moves to CPU 12; under `taskset -c 36-41` (node 1 only) it starts
   there and moves to CPU 0. Both run all four points with the MOVE_STEPS numbers exactly (node 1 -> node 0 gives
   the same VmSize steps as node 0 -> node 1), and the pinned test beside it passes.

## Results (mei, Skylake-SP, final code head)

- Gate (direct tier, `--base origin/next`), heap-spill-cold-3-gate: 16 suites ok, 2071 passed, 0 failed, 626
  ignored; conformance 9 passed; peak 12G.
- Plants, two runs of the same file (5 plants each; the products plant is the one that survived before):
  heap-spill-cold-3-plants1 in slot cpus 0-5 (node 0 only), heap-spill-cold-3-plants2 in slot cpus
  6,7,12,13,30,31 (both nodes). Both: 5 CAUGHT, 0 SURVIVED, 0 WARN. no-check-products' log shows the move in each:
  plants1 "start on CPUs [0..5] ... move to CPU 12", plants2 "start on CPUs [6, 7, 30, 31] ... move to CPU 12",
  both failing at point 1 under RLIMIT_AS = start + 43600 KiB (exit 12). Results committed in the plants file.
- Start-up checks (heap is start-up code): heap-spill-cold-3-startup (`cargo xtask test`, full-profile `so`,
  `hot_text_products`). Product code is unchanged from HEAP-SPILL-COLD-2 apart from the merge.
- clippy (`--workspace --all-targets --all-features -D warnings`) and `cargo xtask static`: clean
  (heap-spill-cold-3-lint).

## Open (not this story; ruling #7492's resolution owes #7493/#7494 as one follow-up)

- The spill (ruling #7493): prototype first and measure the per-spilled-malloc slow-path cost (see
  HEAP-SPILL-COLD-2.md "Not done").
- A smaller first mapping of a non-first node's segment under a limit (ruling #7494, +2048 KiB at point 1).
- INFORMATIONAL: `several_nodes_read_the_node_from_rseq` (same file) has the same slot dependence: its probe
  `oracle/rseq_node.c` skips on "one allowed node" / "no CPU of a node other than 0 is allowed", i.e. in any slot
  inside one node. The same fix (move to another node's online CPU) would apply; not changed here (outside this
  story's direct tests). Recommendation: a short follow-up.

## Next step

Coordinator: land if the jobs in the hand-back are green; then brief the #7493/#7494 follow-up.
