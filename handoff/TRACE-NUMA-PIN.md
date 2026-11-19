# TRACE-NUMA-PIN: the start-up trace sees one fixed NUMA topology on every build host

State: branch `task/trace-numa-pin` (stacked on TRACE-TIME-PIN, base origin/next e035b3034), role keke:~/w/n4.
Tooling, plus one attribute in the library: the library's live topology reads behave as before.

## What changed

- `tools/hottext/trace_gdb.py` (header, TRACE-NUMA-PIN): every trace sees one virtual topology, 2 allowed nodes,
  the program on node 0, in every class (`host` too):
  - node count: `get_mempolicy(MPOL_F_MEMS_ALLOWED)` (nr 239) at `niji_sys_syscall`, the generic entry its one
    caller (`raw::allowed_node_count`) uses: the mask gets nodes 0-1, rax 0, the call never reaches the kernel;
  - node read: at `<KernelPages as Pages>::shard` (v0 name matched by `11KernelPages.*5Pages5shard$`), answered 0.
    The rseq `node_id` is kernel-written memory with no entry, and pinning the tracee to node 0's CPUs (tried
    first) fails where a gate slot's CPU set holds none of node 0's CPUs.
  - `HOT_TEXT_HOST_NODES=<n>` (test-only): a simulated n-node host; the virtual topology answers over it.
  - prints `trace_gdb: node count answered N times, node read M times`.
- `crates/boundary/src/heap.rs`: `#[inline(never)]` on `KernelPages::shard` (it was already out of line in the
  products; the attribute keeps it the single node-read entry; `OwnPages::shard` calls it). The bins of a 2-node
  build are identical with the old and the new tracer (below), so no layout moved.
- `tools/hottext/regen.sh`: traces `page_faults true` a second time with `HOT_TEXT_HOST_NODES=1` and fails when it
  differs from the start-up trace; fails when the start-up trace answered no node count or no node read (a
  renamed entry or a moved call, on any host).

## Measurement (NIJI_FULL_PROFILE=1 so + hot_text_products; md5 prefixes of release outputs)

| build | host (live nodes) | signatures | layout-bins | hot_text_products |
|---|---|---|---|---|
| new (2 nodes, node 0) | keke (2) | 5400401c | 19e5eaab | green, every row at its bound (45/36/36/5/10/43) |
| new, HOT_TEXT_HOST_NODES=1 | keke (2) | 5400401c | 19e5eaab | green, same counts |
| new | mei (2) | 5400401c | 19e5eaab | green, same counts |
| new | shiki (1) | 5400401c | 19e5eaab | green, same counts |
| old tracer | keke (2) | 5400401c | 19e5eaab | green, same counts |
| old tracer (= 1-node trace) | shiki (1) | 41fac4b7 | aba4e9bf | green, same counts |
| 1-node candidate (virtual 1 node) | keke (2) | 41fac4b7 | - | build FAILS: STARTUP-DATA-DECLARED, live `page_faults true` touches `heap::RSEQ_NODE` outside the declared set |

Verdict: 2 nodes on node 0. It serves the 2-node and the 1-node live host with no bound change and keeps the
2-node hosts' layout exactly as before; a 1-node topology does not build on a 2-node host.
Jobs: keke `trace-numa-pin-meas-keke2` (rc 1 by design: the 1-node candidate's failing build), mei
`trace-numa-pin-meas-mei`, shiki `trace-numa-pin-shiki-1node` (ended 19:07:05Z on 11-17; nothing left running there).
Scripts: `~/work/TRACE-NUMA-PIN/meas.sh` (keke, mei), `~/w/trace-numa-pin/meas.sh` (shiki).

## Plants (handoff/TRACE-NUMA-PIN-plants.txt), gate

- `trace-numa-pin-plants`: tnuma-off (virtual topology removed: the 1-node simulation differs), tnuma-read-entry
  (node-read entry not found: guard), tnuma-count-nr (count call not recognised: guard).
- `trace-numa-pin-gate`: clippy, static, direct tier, `cargo xtask test`, full-profile so + hot_text_products.
- Superseded: `trace-numa-pin-meas-keke` (CPU-pinning design, stopped).
Verdicts: see the hand-back.

## Notes

- On a 1-node host the simulated-host comparison cannot see a missing answer (both say one node); the guard
  (answered counts) can, on every host.
- Next step: none for this story once the jobs are green.
