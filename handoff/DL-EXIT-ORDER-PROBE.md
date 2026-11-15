# DL-EXIT-ORDER-PROBE(-2,-3): exit order of dlopen objects (handoff, run 3)

**State (run 3):** branch `task/dl-exit-order-probe-3` = run 2's branch with origin/next (7288d847a, which has
LAYOUT-WINDOW-BINS-2) merged in at 020113a50; no code change in run 3. Jobs on that tree (mei `~/w/s1`):
`dl-exit-order-probe-3-lint` rc 0 (`cargo xtask clippy` + `cargo xtask static`); `dl-exit-order-probe-3-gate`
rc 0 (direct tier, 8 suites, 548 passed / 0 failed, includes `liella::hot_text_products` and `liella-ld::dlclose`):
run 2's only red, the coreutils-shaped start-up fault bound (51 > 50), no longer fails after the merge, so it was
layout jitter that LAYOUT-WINDOW-BINS-2 removed; no bound was touched. `dl-exit-order-probe-3-plants` rc 0 (9/9
CAUGHT, 0 WARN). `dl-exit-order-probe-3-startup` (`cargo xtask test`, then `LIELLA_FULL_PROFILE=1 cargo xtask so` and
`hot_text_products`) was left running at hand-back (6614 tests; the dispatcher reads it).

**Run 2 record:** gate on 02951f4b4 (keke) had failed only
`init_coreutils_shaped_program_stays_within_its_page_fault_bound` (51 > 50, static program that does not run the
changed loader exit path); plants 9/9 CAUGHT.

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
**Next step:** if `dl-exit-order-probe-3-startup` (mei) ends rc != 0, read `~/logs/dl-exit-order-probe-3-startup.log`
and fix only failures this branch causes (scripts: mei `~/work/DL-EXIT-ORDER-PROBE-3/{gate,startup,lint}.sh`).
After landing (when CI assigns ids): add `decisions`, `story`, `date` to the compat record. Later stories: specify
the reference's in-cycle order and implement the registered compat flag.
