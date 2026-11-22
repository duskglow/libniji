# HEAP-TEST-NODE-PIN-2 handoff (continues HEAP-TEST-NODE-PIN; that note, handoff/HEAP-TEST-NODE-PIN.md, still holds)

State: branch task/heap-test-node-pin-2; code commit 70cdce7ed; later commits are handoff/report only.
Role keke:~/w/n1. Decisions in force: ruling #7441 (pin as a single-node measurement, bounds unchanged), #7528 (plants
widen the inherited mask; Skylake-SP gets a mask probe, no plants tree).

## Why HEAP-TEST-NODE-PIN's plants2 survived (found in this run)
A jobrun job is put in a slot of a few CPUs inside ONE node (keke probe job heap-test-node-pin-2-probe:
`Cpus_allowed_list: 16-19`, node0 = 0-29, node1 = 30-59). The test inherits that mask, `numa_nodes()` sees one node,
so pinning is a no-op and `assert_on_one_node` passes with or without it: in that setting both mutants really were
equivalent. The Zen 1 host (keke VM) shows 2 NUMA nodes, one per socket (not several per socket).

## What this run changed
- `assert_on_one_node` prints the CPUs per node the measurement may run on (so each run states its placement) and
  its doc comment says the check sees the pinning only when the inherited mask spans several nodes.
- Plants (handoff/HEAP-TEST-NODE-PIN-2-plants.txt): the cmd widens the inherited mask by one CPU of another node
  (first CPU of the first node not holding the slot's first CPU), prints `plant cpus: <mask>`, then runs the test with
  `--nocapture`. pin-skipped and pin-all-cpus then leave a two-node mask and must be CAUGHT. On a one-node host the
  widening adds nothing.
- Bounds, AS_STEPS and AS_HEADROOM unchanged.

## Results
All on code commit 70cdce7ed (role synced, tree identical).
- keke heap-test-node-pin-2-lint0 (inherited head 507126452): clippy -D warnings + `cargo xtask static`, rc 0.
- keke heap-test-node-pin-2-lint (final code): clippy workspace/all targets/all features -D warnings + static, rc 0.
- keke heap-test-node-pin-2-gate (direct tier, 1 of 312 targets: niji::heap_meta_products): 5 passed, 0 failed,
  rc 0, peak 1.7G.
- keke heap-test-node-pin-2-plants: 2 CAUGHT, 0 SURVIVED (msg matched, no WARN-RC-ONLY/NO-MSG). One WARN-NOT-RAM: the
  copy was on the role's SSD (/mnt/nvme), because keke has no /mnt/zram and the coordinator asked to keep plants off
  /dev/shm (WARN-DEV-SHM). Either choice gives one warning on keke. CPUs per run: inherited slot 16-19 (node0),
  widened 16-19,30. Baselines pinned to [[16-19]] and passed. pin-skipped saw [[16-19],[30]] (2 nodes) and
  pin-all-cpus saw [[0-29],[30-59]]; both failed with "not one (pinning skipped or wrong mask)".
- mei (Skylake-SP; node0 0-11,24-35, node1 12-23,36-47) heap-test-node-pin-2-meimask + -meiprobe (mask probe per
  #7528, a Python model of numa_nodes/with_cpus/assert_on_one_node using real sched_setaffinity, not the test
  binary): inherited 24-29 (1 node, check passes); widened 12,24-29 (2 nodes, check FAILS); pinned to the lowest
  node 24-29 (1 node, passes).
- Start-up checks (`cargo xtask test`, hot_text_products) not run: only a test file changed, no start-up code.

## Not done, and why
- Plants on the Skylake-SP host: no role there (only keke:~/w/n1 admitted); ruling #7528: a mask probe instead.

## Next step
Nothing in this story once the named jobs are green.
