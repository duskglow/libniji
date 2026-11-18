# TRACE-TIME-PIN: the start-up trace sees a virtual clock

State: branch `task/trace-time-pin`, base origin/next e035b3034. Role mei:~/w/s1. Tooling only: no library
code changed (the library's live clock reads are untouched; only what the tracer answers changes).

## What changed

- `tools/hottext/trace_gdb.py`: every time source the library reads is answered from one virtual clock, in
  every class (`host` too); the entry's own code never runs (header comment, TRACE-TIME-PIN):
  - `niji_machine_rdtsc` (the TSC gate of `crates/legacy/src/recheck.rs`): a virtual counter, +1000 ticks per read;
  - the vDSO's `clock_gettime`, `gettimeofday`, `time` at the library's vDSO call entries `niji_machine_vcall2` /
    `niji_machine_vcall3` (the target is matched against the vDSO's own dynamic symbols, read from the
    process's `[vdso]` mapping with `nm -D`; getcpu and clock_getres run as they are);
  - the system call fallbacks `niji_sys_clock_gettime`, `niji_sys_gettimeofday`, `niji_sys_time`.
  Each read advances the clock 1 us (monotonic ids from 1000 s, realtime ids from 2025-10-04, the probes'
  date); the clock restarts with every run of the program. `HOT_TEXT_SLOW=<s>` sleeps at every time read.
- `tools/hottext/regen.sh`: every build traces `page_faults mktime` a second time with `HOT_TEXT_SLOW=2`
  (in parallel with the other traces) and fails when the two traces differ, naming the differing functions.
  So a new time source on that path that the tracer does not answer fails the build. Before, it changed the
  layout on a slow trace.

## Results (mei unless named)

- regen alone on the role's trace products (`trace-time-pin-regen1`, rc 0): "page_faults mktime traced
  with 2 s between time reads: the same trace".
- Determinism: `trace-time-pin-det-mei2` (b1, b2 normal builds, b3slow with `HOT_TEXT_SLOW=2` on every trace)
  and keke `trace-time-pin-det-keke2` (k1): compare `~/work/TRACE-TIME-PIN/det-*/<build>/` hot-text.signatures
  and layout-bins.tsv (the job prints SAME/DIFF per pair; keke vs mei by md5 lines `=== <name>: ...`).
- Plants `trace-time-pin-plants` (handoff/TRACE-TIME-PIN-plants.txt): clock off, TSC to the host, vDSO
  clock_gettime unrecognised.
- Gate `trace-time-pin-gate`: clippy, static, direct tier, `cargo xtask test`, full-profile so +
  hot_text_products (the start-up checks).
Job verdicts: see the hand-back (written after the jobs).

## Sweep (other nondeterminism on traced paths)

- getrandom: tmpfile/tempnam/sem names, `random_bits` (its clock fallback is now virtual): values only, no
  traced probe branches on them. AT_RANDOM: canary and pointer guard values only. getpid: values only.
  ASLR: off already (`disable-randomization`). None changes a signature: not changed.
- RLIMIT_AS (heap registries sized to an address-space limit, `crates/rt/src/heap.rs`): inherited from the
  caller of the build; unlimited on the build hosts (the gate limits memory with a cgroup, not RLIMIT_AS).
  Changes a signature only under `ulimit -v`: not changed (informational).
- NUMA (D-3.20): the heap's first slow path asks `get_mempolicy(MPOL_F_MEMS_ALLOWED)`; more than one node
  turns on node reads (rseq node id, else vDSO getcpu through `niji_machine_vcall3`). keke and mei allow 2
  nodes, shiki and kanon 1: traces of one tree differ between those hosts. RULING NEEDED (below).

## RULING NEEDED

- Virtual NUMA topology in the tracer: which topology should the trace assume? Two nodes keeps keke/mei
  (CI) layouts as they are and needs the node read pinned too (rseq node id is kernel memory, not an entry:
  needs a tracer write to the rseq area or a library entry function); one node is simpler (answer
  get_mempolicy at its syscall stop) but drops the NUMA path from the start-up group on 2-node live hosts.
  Recommend: a follow-up story, virtual 2-node topology with node 0, decided by measuring hot_text_products
  on a 2-node and a 1-node host.

## Next step

Read the job verdicts; if all green, nothing remains. Follow-up: the NUMA ruling above.
