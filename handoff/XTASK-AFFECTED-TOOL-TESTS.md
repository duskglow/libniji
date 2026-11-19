# XTASK-AFFECTED-TOOL-TESTS: a change to a tool that has tests selects those tests

State: branch `task/xtask-affected-tool-tests`, code HEAD d4032f98, jobs on 213d7f7b (on origin/next 86b4af50). No decision record
(local, testable selector detail; follows D-X.35's docs-readers rule).

## What changed (xtask/src/affected.rs)
- `Repo::load`: the xtask integration tests (`xtask/tests/*.rs`) join `xtask_texts`, the xtask sources scanned for
  quoted paths (`names_path`). A docs/tool path one of them names selects the xtask tests (`cargo test -p xtask` runs
  unit and integration tests). This is what was missing for `tools/probe/probe` (`xtask/tests/probe_client.rs`).
- `TOOL_REACH` (next to `docs_readers`): tools a test reaches at run time without naming the file, mapped to the path
  the test does name; the changed file selects that path's readers. Rows are written relative to `tools/` on purpose:
  written in full they are quoted paths in this module's own scanned text, and affected.rs became their "reader"
  (first attempt; the full-tree survey showed it, fixed in ea943b03, tests now assert it cannot recur).
  - `tools/hottext/*.sh` (direct children) -> readers of `tools/hottext` (xtask: labnames' test lists the directory
    and reads every script; hottext's runner test runs runner.sh, which runs canon_host.sh).
  - `tools/deb/copyright` -> readers of `tools/deb/system.sh` (niji::deb_symbols_products: system.sh stages it).
  - `tools/deb/system/*` (direct children) -> the same (system.sh stages its configuration files).
- Rule 2's comment now says: tools are docs-only unless a test names them (any test tree, xtask/tests included) or
  reaches them through a `TOOL_REACH` row; a tool no test reaches selects no tests.

## How the tests were found
grep of the test trees (crates/*/tests, crate src, xtask/src, xtask/tests, conformance) for `tools/` paths on
non-comment lines and for run-time joins (`join("../tools/hottext")`, `"$here/..."` in the scripts the tests run:
deb build.sh -> system.sh -> symbols.py, copyright, system/*; hottext runner.sh -> canon_host.sh, trace_gdb.py).
Then `cargo xtask affected --list --files <f>` for all 135 tracked tool files, before and after (keke jobs
xtask-affected-tool-tests-survey0 / -survey1; the after-fix check in -check2).

Before -> after (`--list`):
- tools/probe/probe: no tests -> xtask (xtask/tests/probe_client.rs)
- tools/hottext/canon_host.sh, probe_text.sh, startup_data.sh: no tests -> xtask
- tools/deb/copyright, tools/deb/system/* (8 files): no tests -> niji::deb_symbols_products
- unchanged: every tool already named (deb build.sh/system.sh/symbols.py/stage-release.sh, hottext *.py/regen/runner,
  probe/dl/order.sh, printf_oracle.py, ...); tools no test reaches (tools/prof/prof.sh, hottext/faultmap.py) stay
  docs-only.

## Tests (no spec clause; xtask tooling)
`direct_tier_tool_named_by_an_xtask_integration_test_selects_the_xtask_tests`,
`direct_tier_hottext_scripts_reached_by_listing_select_the_xtask_tests`,
`direct_tier_deb_copyright_selects_the_deb_symbols_test`, `direct_tier_deb_system_files_select_the_deb_symbols_test`;
`docs_only_runs_no_tests` now uses `tools/prof/prof.sh` (no test reaches it) instead of the probe client.

## Jobs (keke, role n4)
- check1 (da6e5b73): clippy, the new tests: ok. SUPERSEDED by check2 (self-reader fix).
- plants (da6e5b73): 6/6 CAUGHT. SUPERSEDED by plants2 (rows rewritten).
- check2 (ea943b03): cargo xtask clippy, cargo xtask static, direct_tier_/docs_ tests, the --list checks: rc 0.
- plants2 (ea943b03): handoff/XTASK-AFFECTED-TOOL-TESTS-plants.txt, 8 plants (each row removed alone, the
  directory-depth and file-vs-prefix checks, the tools/ root, the table reading itself): 8 CAUGHT, no WARN.
  SUPERSEDED by plants3 (same affected.rs, rerun on the final head).
- gate (ea943b03): RED, affected_run_runs_the_selection_and_list_does_not (see Informational). SUPERSEDED.
- gate2 (213d7f7b, final code head): cargo xtask clippy, cargo xtask static, `cargo xtask affected --run`:
  4 suites, 482 passed / 0 failed, peak 1.1G, rc 0.
- plants3 (213d7f7b): the same 8 plants; result in the hand-back.

## Informational
- Scanning xtask/tests made xtask/tests/affected_mode.rs a "reader" of `docs/plants.md`, which it only passes as a
  `--files` argument; the gate (gate, ea943b03) went red on `affected_run_runs_the_selection_and_list_does_not`, which
  expects that file to stay docs-only. Fixed at the cause: affected_mode.rs builds the path at run time
  (`docs_file()`), the module's convention for a test that names a path as data (as `docs_paths_select_static_only`
  does). No other xtask/tests file quotes a docs path.
- Known limit, unchanged: a tool reached only through a path assembled from pieces is not seen unless a TOOL_REACH
  row names it; a new run-time reach needs a row (the comment says so).

## Next step
Coordinator: land via CI. Nothing of this story is running after the gate and plants2 end.
