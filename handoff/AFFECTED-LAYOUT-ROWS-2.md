# AFFECTED-LAYOUT-ROWS-2: overlap a direct gate's independent steps (step 3 of AFFECTED-LAYOUT-ROWS)

State: branch `task/affected-layout-rows-2` (= `task/affected-layout-rows` + this run; base origin/next d1a9ed061),
HEAD 93f2de0ad (code). Role mei:~/w/s2. Steps 1, 2 and 4 of the original brief: `handoff/AFFECTED-LAYOUT-ROWS.md`
(done, do not redo). Reserved ids: **D-X.175** (entries-only rule, ruling #8698), **D-X.176** (bounded run, step 3).

## Done this run

- Step 0: clippy + static on the inherited head c79679e5d: green (job affected-layout-rows-2-lint0).
- Named jobs of the last run were rc 0: nothing to fix.
- Step 3 code (`xtask/src/affected.rs`, commit 93f2de0ad): `run_selection` builds `RunStep`s (products, xtask
  tests, every crate pass) and `run_steps` runs them: a step starts when the steps in its `after` have ended, up to
  `step_slots` at a time (memory below 6 GiB: one; else min(room / 3 GiB, max(2, free CPUs / 12)), free = CPUs
  minus the 1-min load). A crate's passes stay in order; the first pass splits into what runs no products (beside
  the products build) and the products targets (`first_pass_split`; a target's `products` flag reads its whole
  text with support files, so most niji-math targets count as products). Steps holding nextest test-group binaries
  (`test_group_binaries` reads `.config/nextest.toml`; any non-`binary(=x)` grouped filter: every test step is
  grouped) never overlap. Test steps beside others get `NEXTEST_TEST_THREADS`/`RUST_TEST_THREADS` = their CPU
  share (`share_test_threads`). Each step logs to `target/affected-steps/<n>.log`; console text and log are
  emitted whole, in step order. One slot = exactly the old sequential path. `products_beside_xtask_tests` and
  `side_by_side_room` are replaced (the D-X.142 test names are kept).
- Unit tests: 118 passed (job affected-layout-rows-2-unit3), new: steps order/needs, test groups never overlap,
  group-binary parsing, first-pass split, thread share, slot bound.

## Proof in progress (mei, role s2)

Proof binaries on the role: `target/proof-bin/xtask-step2` (c79679e5d, before) and `xtask-step3` (93f2de0ad's
code, after). `~/work/affected-layout-rows-2/proof.sh <case>` (scratch copy in `$S/proof.sh`): warm run (step 3),
then before (step 2) and after (step 3), products cleared before each, under `~/bin/gate 16G`; results in
`~/work/affected-layout-rows-2/<case>/summary.txt` and `{warm,before,after}.log`.

- **libm** (base d1a9ed061 + `#[inline]` on `signed` in crates/math/src/hyper.rs; scratch worktree `$S/libm`):
  job `affected-layout-rows-2-proof-libm` started 02:37Z. Selection: 12 targets, 0 programs, 1 lib.
- **b-4** (scratch worktree `$S/b4` = origin/task/headers-stage2-gaps-b-4, already made): not started.

## Next step (exact)

1. `~/bin/jobwait affected-layout-rows-2-proof-libm`; read summary.txt and the step lines (`grep -E '^ +[0-9.]+s  '`)
   of before.log/after.log; note the job's peak (jobs.tsv). A failure only in `after` is a concurrency defect: find
   it (no rerun to decide).
2. Sync b-4: `bash ~/src/liella-ops/tools/sync_role.sh --story AFFECTED-LAYOUT-ROWS-2 --base d1a9ed061
   /home/claude/w/.scratch/AFFECTED-LAYOUT-ROWS-2/b4 mei s2`; check `.changed.abi` is non-empty and
   `target/proof-bin/xtask-step3 affected --list` says 5 targets / 0 programs (confirms #8697: narrowing without a
   hand-written file). Then start `affected-layout-rows-2-proof-b4` like the libm job (`... proof.sh b4`).
3. Meanwhile write D-X.175 (rule text in AFFECTED-LAYOUT-ROWS.md "Open") and D-X.176 (bounded run) under
   records/decisions/D-X/ (read developer-flow-reference §10 first), and a TIERS/bounded-run sentence in the
   affected.rs module doc if it names the old side-by-side pair.
4. Resync this worktree (`--base origin/next`), lint job (clippy + static), own gate (`cargo xtask affected --run`
   under `~/bin/gate 16G`), precheck, report json, handback-check. Plants: out of scope (brief item 5).
5. Remove the scratch worktrees `$S/b4` and `$S/libm` (`git worktree remove`) at the end.
