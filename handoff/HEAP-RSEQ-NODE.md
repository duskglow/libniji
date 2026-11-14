# HEAP-RSEQ-NODE: the heap's node read takes rseq `node_id` (D-3.21)

State: branch `task/heap-rseq-node` (stacked on the HEAP-FIRST-USE chain, tip 8b24b6f94 at start) +
cf4dcc5ec (the change, tests, D-3.21) + the plants-file commit + this note. Role shiki:~/w/sh1 (ONE NUMA
node, Linux 6.8). Status: INCOMPLETE: gate and plants running at hand-back; the two-node run and the
counts were not run (budget).

## Decisions in force
D-3.12 (node read from rseq when owned and filled, else vDSO getcpu), D-3.20 (one node never reads a
node), D-3.21 (new, amends D-3.12: what "owned" and "filled" mean; records/decisions/D-3/D-3.21.toml,
written by hand in `cargo xtask records new decision` shape because cargo is not on the agent host;
`cargo xtask static` on shiki was green with it).

## What was done
- `uapi::Rseq`: `node_id` (offset 20) and `mm_cid` (24) replace the padding (kernel UAPI linux/rseq.h
  on mei confirms the order); `Rseq::node_id_filled(feature_size, registered_len)`: true only when
  `AT_RSEQ_FEATURE_SIZE` (27, absent before 6.3) and the registered length both reach byte 24.
- `thread::own_rseq()`: the descriptor's area when `DESC_RSEQ` (created threads, static initial
  thread), or tp + RSEQ_OFFSET for a dynamic program's initial thread whose area Liella's ld.so
  registered (`LOADER_RSEQ_TP`, set in `init_initial` when `LOADER_RSEQ`; its descriptor is adopted
  elsewhere, so `d.rseq` is not the registered area). `tb::rseq_at` is the one new UNSAFE(slice).
- `heap::rseq_node()` (feature answer cached once per process) is tried first in `node_hint`;
  getcpu otherwise. `heap::rseq_owned()` is public for the hosted test.
- Tests: kernel `rseq_node_id_is_read_only_when_the_feature_size_covers_it`; boundary (hosted)
  `a_registration_liella_does_not_own_is_never_read`; products `several_nodes_read_the_node_from_rseq`
  (oracle rseq_node.c: on >= 2 nodes and 6.3+, unmaps the vDSO, pins a new thread to a node n > 0 CPU,
  mallocs; a getcpu read would fault; asserts the page's node == n; skips with the reason otherwise).

## Jobs (shiki)
hrn-lint (clippy + static): rc 0. hrn-gate (`cargo xtask so && cargo xtask affected --run --base
origin/next`) and hrn-plants-2 (2 plants, --dir /dev/shm/claude/HEAP-RSEQ-NODE/plants-1): running.
Re-attach: `~/bin/jobwait hrn-gate hrn-plants-2`; logs ~/logs/hrn-gate.log, ~/logs/hrn-plants-2.log.
SUPERSEDED: hrn-plants (refused at once: --dir missing).

## Open / next step
1. Collect hrn-gate and hrn-plants-2 (both plants must be CAUGHT with their msg:).
2. The two-node run: needs a two-node build host (mei, Linux 6.8, 2 nodes; no role of mine there):
   `cargo xtask so && cargo test -p liella-core --test heap_meta_products several_nodes_read_the_node_from_rseq -- --nocapture`
   must print `rseq node read: node n, page on node n` (not `skipped`). A third plant belongs there:
   drop the rseq branch in `node_hint` (`if let Some(n) = rseq_node()` -> `if let Some(n) = None::<usize>`)
   must fault in that test.
3. Counts: multi-node slab path before/after (expect the vDSO getcpu call gone), single-node rows
   unchanged (the node is never read on one node, so the one-node paths do not change by construction).
NOT VERIFIED: the products test on two nodes, the counts.

## Not to redo
The layout check (offset 20 confirmed from the kernel's UAPI header on mei); the ownership analysis
(dynamic initial thread's descriptor is adopted, its registered area is at tp + RSEQ_OFFSET).
Plan rows: D-3.12, D-3.20, D-3.21, D-22.7, D-27.26, #4198.
