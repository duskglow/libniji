# ROOT-HARNESS-FOLLOWUP: contain the root and namespace launches outside the tests

State: branch `task/root-harness-followup` (from origin/next 5f07429d3); see `git log origin/next..HEAD`.
Role: mei `~/w/s1` (Skylake-SP class), synced at the head after "Mark the bench tests' inspected sudo command".
Brief: `~/state/briefs/ROOT-HARNESS-FOLLOWUP.md`. Predecessor: `handoff/ROOT-HARNESS-CONTAIN.md`.
Legs: 1 (WIP code), 2 (this note: compile fixes, count route fix, kill tests, plants file, doc).

## Done
- `xtask/src/contain.rs`: `user(program)`, `root(cmd)`, `output(cmd)` (tie = `UnixStream::pair`, held for xtask's
  lifetime; std::io::pipe is 1.87 > MSRV 1.85). Kill test per kind + `root_form_keeps_arguments_and_variables`.
- `xtask/src/bench.rs`: `bench_output` (all C-bench spawns), `is_root_route`, NEW `past_sudo`: CBENCH_* vars
  passed as `sudo -n env K=V ...` args. FINDING: before this, sudo's env reset meant cbench_netdb's counted runs
  through the private-/dev/shm route printed NO counters (`--compare --counters` table: instr columns `-`;
  log mei `~/work/root-harness-followup/counts-before/run1.log`). `NIJI_CONTAINED=none` = uncontained route
  (opt-out, same as tools/contained.sh), used by the count check. Unit tests `the_root_route_alone_runs_contained`,
  `root_route_carries_the_bench_variables_past_sudo` (their `sudo()` helper is marked `contain: probe(...)`).
- cbench_nscache does NOT build for `--compare` (includes niji/native.h): no counted mode in xtask; it is checked
  by its budgets (ratio_to rows) uncontained vs contained instead.
- transcript run.rs, tools/contained.sh + prologues (list in the doc), static rule extension: from leg 1.
- `crates/core/tests/harness_containment.rs`: + `a_killed_root_script_leaves_no_process`,
  `a_killed_user_namespace_script_leaves_no_process` (temp script with the prologue, SIGKILL the script's pid).
- `handoff/ROOT-HARNESS-FOLLOWUP-plants.txt`: 13 plants (xtask kinds, bench route + carry, sh kinds, static per
  path kind: validation x2, tools/deb, tools/probe, oracle, xtask transcript, xtask bench ns).
- Doc `docs/testing/harness-containment.md`: rules + "Outside the tests" section; `COUNTS-TO-FILL` placeholder.
- Clippy clean (lint-2: only static failed, on the test sudo lines, since fixed).

## Jobs started (mei, queued behind release-snapshot slots at 10:37Z)
- `root-harness-followup-lint-3` (clippy + static on the synced head): must be rc 0.
- `root-harness-followup-count2`: `~/work/root-harness-followup/count2.sh` -> `~/work/root-harness-followup/count2/`
  netdb-{uncontained,contained}-{1..5}.tsv (counters-cbench_netdb.tsv copies; columns per COUNTERS_HEADER in
  bench.rs ~1091) and nscache-{uncontained,contained}.log (budget lines). Alternating, same head.
- `root-harness-followup-test-1`: `cargo test -p xtask -- contain:: bench::tests harness_contain` and
  `cargo test -p niji --test harness_containment`.
`jobwait` returns 3 at once while a job is QUEUED: one wait per call, do not poll.

## Next steps (in order)
1. jobwait lint-3, test-1, count2 (one call). Fix anything red (re-sync after a fix, re-run only that job).
2. Count table: per netdb row, instr/op libniji and glibc, uncontained vs contained (5 each): equal, or explain.
   If the PID namespace changes the counts: STOP and report the numbers (brief step 1). nscache: budgets pass both.
   Fill `COUNTS-TO-FILL` in the doc (rows, medians, range, "Skylake-SP"; one CPU class only, say so).
3. Namespaced transcripts: find the usage (`cargo xtask transcript` with no args / `--help`) and run the
   locale/getent/ldconfig ones on the role as a job (kind test).
4. Plants: `setsid nohup ~/bin/jobrun root-harness-followup-plants-1 plants -- sh -c '. ~/.cargo/env; cd ~/w/s1 &&
   cargo xtask plants handoff/ROOT-HARNESS-FOLLOWUP-plants.txt --dir /mnt/zram/claude/root-harness-followup/plants-1
   --timeout 600' </dev/null >/dev/null 2>&1 &` ; all CAUGHT; append the result lines to the plants file.
5. Gate: `cargo xtask affected` (no --run) first; FULL escalation -> precheck + changed crates' tests (kind test),
   write `FULL: the train's (...)`. Then precheck `--jobs <labels> --head <sha>`.
6. Report: `handoff/reports/ROOT-HARNESS-FOLLOWUP.json` (+ "jobs"), this note as the report, handback-check.
   INFORMATIONAL: counters never reached the shm-route benches before (fixed here); nscache uncountable in xtask.
