# DL-EXIT-ORDER-PROBE(-2): exit order of dlopen objects (handoff, run 2)

**State:** branch `task/dl-exit-order-probe-2` (stacks on `task/dl-exit-order-probe`), HEAD = the commit adding
this note. The work is done; final jobs (gate, start-up checks, plants) run on this head on keke `~/w/n4`:
`dl-exit-order-probe-2-gate`, `dl-exit-order-probe-2-startup`, `dl-exit-order-probe-2-plants`.

**Probe-settled (run 1; record `== exit-order (DL-EXIT-ORDER-PROBE)` in spec/behaviour/dynlink.glibc.txt, text in
spec/behaviour/dynlink.md):** program first, then start-time and `dlopen`ed objects as ONE sequence, earliest
loaded first, each object after every object that needs it (`DT_NEEDED` or a global-scope binding, E21);
NODELETE / unique pinning / open handle: no effect. Inside a cycle the reference's order depends on the other
objects (E19-E30).

**Decisions in force:** pending decision `records/pending/DL-EXIT-ORDER-PROBE-2-1.toml` (D-NEW, series D-27):
the order above; a cycle runs as one group, last initialised first (ruling #5790). Registered compat flag
`records/pending/DL-EXIT-ORDER-PROBE-2-2.toml` (COMPAT-NEW, status registered, not implemented). Differences
row "Exit order inside a dependency cycle of loaded objects" (docs/differences-from-glibc.md, after the
`dlsym` foreign-handle row). Ids are assigned by CI.

**Code (run 2):**
- `crates/boundary/src/ld.rs`: `FINI_IDX` records each `FINI_HOOKS` slot's start-time object (node index);
  `fini_all` passes `(node, words)` per slot to `dlopen::fini_exit`; without `dlopen` (LOADER 0) the old loop.
- `crates/boundary/src/ld/dlopen.rs`: `fini_exit` / `exit_set` replace `fini_loaded`. Units: start-time objects in
  fini order, chained; then logged, live, not finalising `dlopen`ed nodes by index (= load order). Edges:
  `deps_of` + `binds_of` mapped to units. Rank: start units below every `dlopen` unit, `dlopen` units by log
  position. `init::exit_order` (run 1) gives the sequence; `Finis::push` adds start units, `Finis::add` marks
  `dlopen` units finalising. Unreachable error path keeps the old order.
- Test `exit_runs_loaded_objects_as_one_sequence` (crates/ld/tests/dlclose.rs, `prog DIR exit SCENARIO`, stdout
  at exit): indep (E1-E9), dep (E17), bind (E21), cycle (E20, Liella's rule), start (E24/E25 with start-time S
  needing V, M opened needing S). Passed on keke, both kernel-started and `ld.so PROGRAM`.
- Plants: handoff/DL-EXIT-ORDER-PROBE-2-plants.txt (9: rank reversed in cycle, no bind edges, no dep edges, no
  start chain, start units dropped, FINI_IDX zero, exit_order group sort / free pick / waits).

**Open / informational:** a pending compat record cannot name a pending decision (`records check` resolves
`decisions` among assigned ids only, and `date` requires `decisions`, `story` requires `date`), so
DL-EXIT-ORDER-PROBE-2-2.toml has no `story`/`date`/`decisions`; the link is in its body: when ids are
assigned, add `decisions`, `story`, `date`. The start's search in this build resolves start-time
`DT_NEEDED` by path only (the test links S, V by path).

**Not to redo:** probe runs (run 1), exit_order and its unit test, the wiring.
**Next step:** none for the story once the three jobs are green; a later story may specify the reference's
in-cycle order from more probes and implement the registered compat flag.
