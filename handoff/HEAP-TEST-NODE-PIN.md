# HEAP-TEST-NODE-PIN handoff

State: branch task/heap-test-node-pin, HEAD = latest pushed (code commit 6d72bb391 + this note). Role keke:~/w/n1.

Done (code complete, not yet verified on the final head):
- `the_heap_keeps_address_space_near_its_use` pins both runs (all four steps, unlimited and RLIMIT_AS) to the
  lowest NUMA node with allowed CPUs via `numa_nodes` + `with_cpus`, copied verbatim from task/heap-spill-cold-2
  (coordinator note 04:03: same names, one pair after its merge). `assert_on_one_node` (inside the pinned closure)
  fails when the thread may run on CPUs of 2+ nodes. Bounds and AS_HEADROOM unchanged (ruling #7441).
- Plants file: handoff/HEAP-TEST-NODE-PIN-plants.txt (pin-skipped, pin-all-cpus; msg `not one \(pinning ...`).
- Proof scripts (local scratch ~/w/.scratch/HEAP-TEST-NODE-PIN/): heappin.sh (mei: 20 runs flip within node0
  halves + 20 cross-node control, limit 41296), par24.sh (mei: 20 batches x 24 pinned parallel copies, coordinator
  note 03:55), gate20.sh (keke: the target 20x as the gate runs it; installed at ~/work/heap-test-node-pin/).

Superseded (old heads, killed): keke heap-test-node-pin-gate, heap-test-node-pin-plants (head ddb09af42).
Last sync to keke:~/w/n1 was BUSY (rc 4) while those were being killed: the role still holds ddb09af42.

Exact next step:
1. sync_role.sh again (check rc 0); on the role run clippy (fg) and `cargo xtask static` (last run rc=1 on the
   stale tree mid-kill: read ~/logs/heap-test-node-pin-static.out tail; rerun after sync).
2. Start detached: heap-test-node-pin-gate2 (affected --run), heap-test-node-pin-plants2 (--dir
   /dev/shm/claude/heap-test-node-pin/plants-2), then heap-test-node-pin-gate20 (after gate2:
   `sh ~/work/heap-test-node-pin/gate20.sh ~/w/n1 20`).
3. mei: copy keke:~/w/n1/target/test-oracles/heap_address_space_niji (static; oracle and libc unchanged by this
   story) to mei ~/work/heap-test-node-pin/, scp heappin.sh and par24.sh there, run under jobrun with outdir
   /mnt/zram/claude/heap-test-node-pin/: labels heap-test-node-pin-mig20 (`heappin.sh <exe> 20 <dir>`),
   heap-test-node-pin-par24 (`par24.sh <exe> 20 <dir>`).
4. Report: the external taskset flip in -heapmig overrides any mask, so the pinned equivalent is the in-node flip
   (the moves the scheduler can still make); say so. Write handoff/reports/HEAP-TEST-NODE-PIN.json (+jobs),
   precheck, handback-check.
