# GATE-DEV-NO-FULL: a developer's `affected` run never escalates to FULL

State: branch `task/gate-dev-no-full`, final head in handoff/reports/GATE-DEV-NO-FULL.json. Role keke:~/w/n1
(Zen 1 host, 60 vCPU). Done: code, tests, plants (12/12 caught), gate green, item 5 comparison
(handoff/GATE-DEV-NO-FULL-report.md), CONTRIBUTING.md step 2 updated.

## What changed (xtask/src/affected.rs, xtask/src/precheck.rs, CONTRIBUTING.md)

- In `direct` and `wide`, `select_with(.., train=true)` gives every escalation to the train. `Selection::train`
  holds the `<file>: <reason>` lines, `full` stays empty, and `Selection::hand_to_train` picks each escalated
  file's bounded set by `TrainClass::of(path)`:
  - `Xtask`: xtask unit tests, xtask clippy, and `XTASK_TRAIN` (each module's own commands; generator modules map
    to their `*-check` prechecks; `so`/`hottext`/`coldbss`/`layout*`/`startup_ro` map to `layout-check` plus the
    start-up layout tests; `native_examples`/`rust_dylib` map to `native`/`rust-dylib`).
  - `Image` (csys/, crates/abi, libc-so, libm-so, native/link, linker lists, scripts/, `_start`): products build,
    `LAYOUT_TESTS`, `k5`, the file's own mapping, the owner crate's clippy and unit tests, and the unit tests of
    its one-hop dependents.
  - `Generated`: the generator checks, `readers_of` the file, and the owner crate's tests.
  - `Config` / `Unknown`: clippy and the owner crate's unit and test targets.
  - Every class: `cargo xtask static`, its readers, and every `precheck::CHEAP_STEPS` step first.
- The plan prints `FULL: the train's (<file>: <reason>)` where the FULL line used to be.
- Unchanged: `--tier full` (CI's landing gate), `select_opts` for bench-rows (FULL still means it cannot narrow),
  and `--reverse` / `--replay`, which keep `train=false` because the train's FULL gate runs them.
- Tests: `per_fix_tiers_leave_full_to_the_train_with_a_bounded_set_per_class` covers 12 files over the 5 classes
  in both tiers, plus the full tier and bench-rows. `precheck::a_gate_that_left_full_to_the_train_is_green` checks
  that the train line leaves the green fixture's `jobs` verdict unchanged. In the older tests, the
  "must not escalate" asserts now use `.no_escalation()` (no FULL and no train entry).

## Item 3

precheck needed no code change: a bounded gate still ends `affected: ok`. In liella-ops, `ops/handback.py` has no
rule that expects a FULL run (it checks tested content per job), and `ops/db.py:327` is the train's own `--tier
full --run`, which is unchanged. Nothing to report there.

## Open / for the coordinator

- No D-row was added (no reserved id). A pending D-NEW for the train classes may be wanted (RULING NEEDED in the
  report json).
- Item 5 (report file): the Xtask and Image bounded sets are still dominated by the products build and the xtask
  unit tests (about 1,100 s on keke under load). Making those cheaper is the next lever, not the selection.

## Not to redo

Gate `gate-dev-no-full-gate2` (rc 0), plants `gate-dev-no-full-plants2` (12 CAUGHT), selection replay
`gate-dev-no-full-sel`, measurements `gate-dev-no-full-meas`. Superseded: `-gate` and `-plants` (old precheck test
expected Ok, but the fixture's level is Note), `-t1`, `-t2`, `-lint`, `-t3` (a targeted rerun).

## Next step

None in this story. The coordinator lands it. Plan rows to read: none (the brief is the spec).
