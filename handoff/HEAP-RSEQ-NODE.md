# HEAP-RSEQ-NODE(-2): the heap's node read takes rseq `node_id` (D-3.21)

State: branch `task/heap-rseq-node-2` (stacked on the HEAP-FIRST-USE chain, tip 8b24b6f94) = HEAP-RSEQ-NODE's
cf4dcc5ec (change, tests, D-3.21) + e4f0b9fa7 + 819105db2 (plants file, handoff) + 02751a80e + fd71dfaba (this
run: start-up state declarations, plants file, count spec) + this note. Role mei:~/w/s4 (Skylake-SP, TWO NUMA
nodes, Linux 6.8). Status: INCOMPLETE: blocked on HEAP-FIRST-USE-8's open ruling (first localtime_r faults),
which this change now also triggers on two-node hosts. Do not land before that ruling and its fix.

## Decisions in force
D-3.12, D-3.20, D-3.21 (reserved for HEAP-RSEQ-NODE; hand-written record, `cargo xtask static` green on mei
on the inherited head, job heap-rseq-node-2-lint0). D-X.62 / D-X.64 (start-up data windows, the `.data1`
cluster declared in crates/libc-so/startup-state.list).

## What this run found and did
1. Inherited head: clippy + static green (`heap-rseq-node-2-lint0`).
2. Why hrn-gate (shiki) failed: `cargo xtask so`'s STARTUP-DATA-WINDOWS contract: `heap::RSEQ_NODE` is an operand
   of the heap's first-allocation path (a start-up function), so it counts as a written start-up object; it sat
   alone on a page (fast: 8 pages > 7). The release build (first count run) also showed `thread::LOADER_RSEQ`
   alone (9 > 8): cf4dcc5ec made `init_initial` read it at every start. Fix: both, and `LOADER_RSEQ_TP`
   (written by a dynamic start), declared in startup-state.list (02751a80e, fd71dfaba). Release contract now
   green (`heap-rseq-node-2-startup-rel`: products built, 24 of 26 hot_text_products pass).
3. Two-node proof: plants run on mei (`heap-rseq-node-2-plants-3`): the unmutated `several_nodes_read_the_node_from_rseq`
   passes on two nodes (not skipped); the plant that ignores the rseq answer (`rseq_node().filter(|_| false)`)
   faults (SIGSEGV, vDSO unmapped): CAUGHT. A plant that drops the call entirely does not build: the archive
   loses RSEQ_NODE and the declared-state check refuses it (plants-1 WARN-NO-MSG, superseded).
   The two-node run itself (`heap-rseq-node-2-twonode`): `rseq node read: node 1, page on node 1`.
   Direct gate (`heap-rseq-node-2-gate2`, fast, `cargo xtask so && cargo xtask affected --run --base origin/next`):
   32 suites ok, 11800 passed, 0 failed; conformance 137/0; peak 15G.
4. Counts (mei, two nodes, release, `heap-rseq-node-2-percall2`, spec validation/percall/thread_first_malloc.c,
   base 8b24b6f94 = the tip before cf4dcc5ec): thread_first_malloc base 36402.1, new 36381.1 (-21 instr per
   new thread's first allocation, sd 0); glibc 3410.0. One-node paths do not read the node (D-3.20), unchanged
   by construction.

## RED: release first localtime_r 20 > 18 on two nodes (same cause class as HEAP-FIRST-USE-8's ruling)
`heap-rseq-node-2-startup-rel` (release, mei): init_first_localtime_r_stays_within_its_page_fault_bound 20 > 18,
init_first_localtime_r_and_mktime_stay_within_their_combined_page_fault_bound 20 > 18. On two nodes the first
allocation at start-up used to call the vDSO getcpu, which looked the vDSO up; it now reads rseq, so the lookup
moves into the first clock call, as HEAP-FIRST-USE-8 measured on one node (vDSO image page + vcall2 text window).
Inferred from that run's fault map, not re-mapped here. Its options (a) p_true also calls clock_gettime so
vcall2 joins the start-up head, (b) eager vDSO lookup at start-up vs. accepting the lazy page, apply to both host
classes now. No bound changed.

## Jobs (mei)
Final (code head fd71dfaba): heap-rseq-node-2-lint0 (inherited head), -startup-rel (rc 101, the RED above),
-plants-3, -percall2, -gate2, -twonode. `cargo xtask test` not run: the head cannot land before the ruling.
SUPERSEDED: shiki hrn-gate (layout red, fixed), heap-rseq-node-2-gate (stopped: head changed),
heap-rseq-node-2-plants (two-node plant did not build, re-planted), heap-rseq-node-2-plants-2 (stopped: head
changed), heap-rseq-node-2-percall (release layout red on 02751a80e, fixed).

## Next step
After the HEAP-FIRST-USE-8 ruling and its fix (on the chain below this branch): bring that fix onto this branch
(merge allowed only if the brief says so), then on mei: release so + hot_text_products, `cargo xtask so && cargo
xtask affected --run --base origin/next`, `cargo xtask test`, plants (handoff/HEAP-RSEQ-NODE-2-plants.txt).

## Not to redo
The layout check of `Rseq` (offset 20), the ownership analysis, the three plants (all CAUGHT on mei), the count.
crates/core is package `liella` (not liella-core). Plan rows: D-3.12, D-3.20, D-3.21, D-X.62, D-X.64, #4198.
