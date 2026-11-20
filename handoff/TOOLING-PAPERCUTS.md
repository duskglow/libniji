# TOOLING-PAPERCUTS: tooling defects (brief says six, lists five)

State: branch `task/tooling-papercuts` (from origin/next 90f71362a). Code commit 4db04d4ba; this note and the
report record after it. Plants: `handoff/TOOLING-PAPERCUTS-plants.txt`. Report: `handoff/reports/TOOLING-PAPERCUTS.json`.
The brief's title says six defects; its list has five (1-5). All five are handled below; there is no sixth to do.

## 1. jobwait (liella-ops `vm/jobwait`, deployed as `~/bin/jobwait`; the tree has no `vm/`)

The deployed copy on the Skylake-SP build host is byte-identical to liella-ops `vm/jobwait` (md5), so the fix belongs
there. liella-ops is read-only for task runs: the fixed script, its test and the diff are in
`handoff/TOOLING-PAPERCUTS-jobwait/` (`jobwait`, `test_jobwait.sh`, `jobwait.diff`).
**Deploy step (coordinator):** copy `jobwait` and `test_jobwait.sh` to liella-ops `vm/`, commit, then install
`vm/jobwait` as `~/bin/jobwait` on every build and timing host, the same way as the other `vm/` tools.

Causes found (code reading, then tested):
* `--help` / `-h`: only an argument-less call printed usage; the label check allowed `-`, so `--help` was a label
  that never gets a row: it waited the whole `--max` (570 s). Now usage, exit 0; an unknown `-x` option is exit 2.
* `--max` was parsed only as the first argument: `jobwait <label> --max 1800` made `--max` and `1800` labels, so the
  call always ended after the 570 s default as "still running", whatever timeout was asked (#6771's "few minutes").
  Options now go anywhere, `--max=S` too.
* A dead or unknown label (no jobs.tsv row because the job died, was killed without its row, was never started, or
  was a `host:label` form, which jobrun labels can never match) waited the whole `--max` and said "still running".
  Now a label with no row, no slots-queue entry and no jobrun holding `~/logs/.locks/<label>` (read from
  /proc/locks, never by taking the lock, which would make a starting jobrun refuse its job) for `JOBWAIT_GRACE` s
  (30) ends the wait with exit 4 "not running: ..." (a stale gate job is told apart from a live one). `host:label`
  for this host is accepted; another host is exit 2 with the ssh command to use.
* Test `test_jobwait.sh` (15 cases, private logs dir and slots state): job `tooling-papercuts-jobwait-test` on the
  Skylake-SP build host, all OK, rc 0. It was not run against the old script (several cases would block 570 s each).
* "Did it shorten a wait a run then treated as finished?": NOT VERIFIED. jobwait itself never reported a job as
  finished early (exit 3 always printed "still running"); a hand-back naming a job without its row is caught by
  precheck's WAIT and the dispatcher's verify. The findings themselves are not readable through `ops`, so no
  per-run check was done.

## 2. plants: bare "rc 124"
`ended()` names a timeout as "timed out after N s (--timeout <secs>)" (or --prepare-timeout). A failed baseline
names the plant(s) whose command it ran (`baseline_plants`). The restore build's error no longer prints `Ok(124)`.
A timeout verdict names the command. Test `a_timeout_names_its_limit_and_the_baseline_names_its_plants`.

## 3. probe looks in target/liella
Not a tree defect: `tools/probe/probe` already uses `target/niji`. The installed `~/bin/probe` on the Zen 1 build
host is a copy from 10-09 (before the rename) that still uses `target/liella`. **Deploy step (coordinator):** copy the
tree's `tools/probe/probe` over that `~/bin/probe` (no other build host has one).

## 4. `cargo xtask so` ran headers-gen --check before every build
`headers::stage` no longer calls `run(root, true)`; it stages the committed `libc-include/`. `headers-gen-check`
stays a CI step and a precheck/affected cheap step. Proof: the three S17 plants (#6677) whose patterns still match
the tree are in the plants file and must be CAUGHT with their own msg. Three S17 plants (`object-fnptr-name`,
`tag-forward-decl`, `mcheck-enum-value`) no longer match the current generator/template text, so they were not run.
Result (job `tooling-papercuts-plants`, 7 plants: 6 CAUGHT, 0 SURVIVED, 1 WARN-NO-MSG): `obstack-finish-align` and
`printf-va-list` are now CAUGHT by their own msg. `queue-tailq-remove` is WARN-NO-MSG, but for a new reason: it now
reaches its own command, and `validation/headers_p10_s17.sh` fails it (rc 1) because the P10 program does not
compile on the mutant `TAILQ_REMOVE` (`-Werror=incompatible-pointer-types`), not with the runtime "P10 ... DIFFERENT"
the S17 regex expects. The right test catches it; that plant's msg in `validation/headers_s17.plants` should become
`P10 .* DIFFERENT|incompatible-pointer-types` (not changed here; not re-run).

## Jobs (Skylake-SP build host, final code head 4db04d4ba)
`tooling-papercuts-lint` (clippy + static + affected --list, rc 0), `tooling-papercuts-gate` (affected --run, xtask
bounded set, rc 0, 231 s; FULL: the train's, xtask files), `tooling-papercuts-plants` (rc 0, above),
`tooling-papercuts-jobwait-test` (rc 0).

## 5. escalated FULL ran the cheap steps twice
`affected::full_steps`: after the escalation prechecks (every cheap step, green, or the run stops), the FULL run
skips the second `cargo xtask stubs` and runs `cargo xtask test --cheap-steps-done` (`precheck::CHEAP_DONE`),
which skips the cheap steps. `cargo xtask test` alone still runs all of them. Test
`escalated_full_run_does_not_repeat_the_cheap_steps`. The `test_all` argument split has no plant (no unit test
reaches `test_all`); it is a three-line match.

## Next step
Coordinator: deploy jobwait (1) and probe (3); land the branch. Not to redo: the causes above.
