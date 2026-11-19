# GATE-DEV-NO-FULL: a developer's `affected` run never escalates to FULL

State: branch `task/gate-dev-no-full`, code head 9236c2cf6 (WIP commits; the code is done, the proof is running).
Role: keke:~/w/n1 (Zen 1 host, 60 vCPU; plants in /dev/shm/claude/gate-dev-no-full/plants-<n>).

## What changed (xtask/src/affected.rs, xtask/src/precheck.rs)

- `select_tier` (Direct and Wide) hands every escalation to the train: `Selection::train` holds the `<file>: <reason>`
  lines, `full` stays empty, and `Selection::hand_to_train` selects the bounded set of each escalated file by
  `TrainClass::of(path)`:
  - `Xtask`: xtask unit tests, xtask clippy, plus `XTASK_TRAIN` (module -> own commands, generator prechecks, start-up
    layout tests: `so`, `hottext`, `coldbss`, `layout`, `layout_verdict`, `startup_ro` -> `layout-check` + layout tests;
    `native_examples`/`rust_dylib` -> `native`/`rust-dylib` on products; generators -> their `*-check` prechecks).
  - `Image` (csys/, crates/abi, libc-so, libm-so, native/link, linker lists, scripts/): products build with the start-up
    layout tests (`LAYOUT_TESTS`), `k5`, the file's own mapping (a .rs), the owner crate (csys -> `CSYS_CRATE`) clippy +
    unit tests and its one-hop dependents' unit tests.
  - `Generated`: generator checks (cheap steps), `readers_of` the file, own mapping for a .rs, the owner crate's unit
    tests for a data file.
  - `Config` / `Unknown`: clippy (`workspace` key at the root), the owner crate's unit and test targets.
  - Every class: `cargo xtask static`, its readers, and every `precheck::CHEAP_STEPS` step first (`prechecks`).
- The plan prints `FULL: the train's (<file>: <reason>)` (`train_lines`) where the FULL line was.
- Unchanged on purpose: `--tier full` (CI's landing gate); `select_opts` (bench-rows still sees FULL = cannot narrow);
  `--reverse` attribution and `--replay` (they call `select_with(.., train=false)`: an escalation still covers
  everything there, since the train's FULL gate runs it).
- Tests: the existing tests' `.full.is_empty()` became `.no_escalation()` (FULL or train) so "must not escalate"
  asserts keep their strength; two read `train` now. New: `per_fix_tiers_leave_full_to_the_train_with_a_bounded_set_per_class`
  (12 files over the 5 classes, both tiers, plus full tier / bench-rows unchanged) and precheck's
  `a_gate_that_left_full_to_the_train_is_green`.
- Plants: `handoff/GATE-DEV-NO-FULL-plants.txt` (12: each class re-escalated, full tier/bench-rows/reverse trained,
  cheap steps dropped, one-hop dropped, train line reworded, precheck asking for FULL).

## Item 3 (handback-check / precheck)

precheck needs no code change: a gate job is green with an `affected: ok` verdict, and a bounded run prints it after
the train lines (new test proves a log carrying `FULL: the train's` is green). liella-ops `ops/handback.py`: no rule
expects a FULL run (it checks tested content per job only); `ops/db.py:327` is the train's own `--tier full --run`,
unchanged. Nothing to report there.

## Jobs (keke)

- `gate-dev-no-full-lint2`: clippy + static + `affected --list` on 9236c2cf6: rc 0.
- `gate-dev-no-full-gate`: `affected --run` (xtask unit tests; the whole affected suite took ~13 min in a test run).
- `gate-dev-no-full-plants`: `cargo xtask plants handoff/GATE-DEV-NO-FULL-plants.txt --dir /dev/shm/claude/gate-dev-no-full/plants-1`.
- Superseded: `gate-dev-no-full-t1` (compile error, duplicate const), `-t2` (4 old tests expected Direct FULL; fixed),
  `-lint` (clippy `wrong_self_convention` on `to_train`; renamed).

## Next step (exact)

1. `~/bin/jobwait gate-dev-no-full-gate gate-dev-no-full-plants` on keke; fix any red (rerun only the failing test).
2. Item 5 measurement: find ~10 developer gate logs of 11-16..11-19 that printed `FULL (escalated` (gate logs on the
   build hosts' `~/logs/`, kind gate), across the classes; for each branch head on origin run
   `cargo xtask affected --list --base <its base> ` on a checkout of that head... (cheaper: `--files <its changed files>`
   from `git diff --name-only origin/next...<head>` against this tree) and compare the bounded selection's size/time
   estimate with the FULL run's wall time from jobs.tsv. Table into handoff/GATE-DEV-NO-FULL-report.md.
3. Final: sync, `cargo xtask precheck --jobs <labels> --head <sha>` on the role, report json, `ops handback-check`.

## Notes for the report

- Process lapse: one Bash call used a heredoc (`python3 - <<'X'` to edit reverse()); later edits used the Edit tool.
- Blocked call: `sleep 100; cat <background output file>` (tool policy: use jobwait/Monitor); not retried.
- No D-row added (no reserved id); a pending D-NEW may be wanted for the train classes.
