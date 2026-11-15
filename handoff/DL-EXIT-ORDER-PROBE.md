# DL-EXIT-ORDER-PROBE(-2,-3,-4): exit order of dlopen objects (handoff, run 4)

**State (run 4):** branch `task/dl-exit-order-probe-4` = run 3's head c8d61b018 (ids D-27.54 and COMPAT-31
assigned) with origin/next 8d38ba722 merged in at aef432c31 (includes LAYOUT-WINDOW-BINS-6, the structural fix
for the systemic localtime_r start-up fault bound, #1307), then 333de3260 linking COMPAT-31 to D-27.54. No
product code change in run 4; no bound touched. Jobs (keke `~/w/n2`, scripts `~/work/DL-EXIT-ORDER-PROBE-4/`):
`dl-exit-order-probe-4-lint0` rc 0 (clippy + static on the inherited head, step 0);
`dl-exit-order-probe-4-lint` rc 0 (clippy, static, `records check`: 647 decision / 31 compat / 0 pending);
`dl-exit-order-probe-4-gate` rc 0 (direct tier, 8 suites, 558 passed / 0 failed, includes
`liella::hot_text_products` and `liella-ld`; peak 1.6G); `dl-exit-order-probe-4-plants` rc 0 (9 plants,
handoff/DL-EXIT-ORDER-PROBE-2-plants.txt: 9 CAUGHT, 0 SURVIVED, 0 WARN-RC-ONLY/NO-MSG; WARN-DEV-SHM only);
`dl-exit-order-probe-4-startup` rc 0 (`cargo xtask test`: 6649/6649 passed incl.
`init_first_localtime_r_stays_within_its_page_fault_bound` and `..._and_mktime_...`, STARTUP-DATA-WINDOWS ok,
conformance 137/0; then `LIELLA_FULL_PROFILE=1 cargo xtask so` + `hot_text_products` 26/26; 1 h 53 min on 4
cores, peak 13G).

**Run 3's CI red:** `init_first_localtime_r(_and_mktime)` 41 > 40 page faults, systemic on next (#1307, three
unrelated stories); this branch adds no start-up code (the change is in the loader's exit path,
`fini_all`/`fini_exit`, and runs only at exit). Held for LAYOUT-WINDOW-BINS, now landed and merged.

**Probe-settled (run 1; record `== exit-order (DL-EXIT-ORDER-PROBE)` in spec/behaviour/dynlink.glibc.txt, text in
spec/behaviour/dynlink.md):** program first, then start-time and `dlopen`ed objects as ONE sequence, earliest
loaded first, each object after every object that needs it (`DT_NEEDED` or a global-scope binding, E21);
NODELETE / unique pinning / open handle: no effect. Inside a cycle the reference's order depends on the other
objects (E19-E30).

**Decisions in force:** D-27.54 (the order above; a cycle runs as one group, last initialised first; ruling
#5790). COMPAT-31 (registered, not implemented; now names `decisions = ["D-27.54"]`, story, date). Differences
row "Exit order inside a dependency cycle of loaded objects" (docs/differences-from-glibc.md) cites both ids.

**Code (run 2):**
- `crates/boundary/src/ld.rs`: `FINI_IDX` records each `FINI_HOOKS` slot's start-time object (node index);
  `fini_all` passes `(node, words)` per slot to `dlopen::fini_exit`; without `dlopen` (LOADER 0) the old loop.
- `crates/boundary/src/ld/dlopen.rs`: `fini_exit` / `exit_set` replace `fini_loaded` (start units chained in
  fini order, then live `dlopen`ed nodes by load order; edges `deps_of` + `binds_of`).
- `crates/ld/src/init.rs`: `exit_order` (Tarjan groups, earliest free group first, in-group last-initialised first).
- Tests: `exit_runs_loaded_objects_as_one_sequence` (crates/ld/tests/dlclose.rs), `exit_order_*`
  (crates/ld/tests/init_order.rs). Plants: handoff/DL-EXIT-ORDER-PROBE-2-plants.txt (9).

**Not to redo:** probe runs (run 1), exit_order and its tests, the wiring, the record links.
**Next step:** none for this story once CI is green. If CI reds again on a start-up fault bound with no
start-up code in this branch: per the LAYOUT-JITTER rule, measure the same count on origin/next and hand back
`INCOMPLETE: LAYOUT-JITTER` (never raise a bound). Later stories: specify the reference's in-cycle order from
probes and implement COMPAT-31.
