# AFFECTED-LAYOUT-ROWS-2: overlap a direct gate's independent steps (step 3 of AFFECTED-LAYOUT-ROWS)

State: branch `task/affected-layout-rows-2` (= `task/affected-layout-rows` + this run; base origin/next d1a9ed061).
Code head 9cbe6b2ff; records and this note after it (see `git log`). Role mei:~/w/s2. Steps 1, 2 and 4 of the
original brief: `handoff/AFFECTED-LAYOUT-ROWS.md` (done). Decisions: **D-X.175** (entries-only rule, ruling #8698),
**D-X.176** (bounded run, amends D-X.142). Report: `handoff/reports/AFFECTED-LAYOUT-ROWS-2.json`.

## Done

- Step 0: clippy + static on the inherited head c79679e5d green (job affected-layout-rows-2-lint0).
- #8697 confirmed: a role synced from the b-4 tree had a 276-line `.changed.abi`; `affected --list` there said
  "selected: 5 of 328 test targets, 0 of 155 conformance programs" (was 225 / 103) with no hand-written file.
- Step 3 (93f2de0ad): `run_steps` runs the products build, the xtask tests and every crate's passes as steps, a step
  starting when the steps in its `after` have ended, up to `step_slots` at a time; test-group steps never overlap;
  output in step order; one slot = the old order. Rules in the `affected.rs` module doc, why in D-X.176.
- 25cec22dd: `step_slots` took the host's load off the run's own CPU count (a 6-CPU gate on a 48-CPU host printed
  "1 of 6 CPUs free"); free CPUs are now the host's online CPUs minus its load, at most the run's own.
- 9cbe6b2ff: the b-4 proof found `so::tests::a_product_build_leaves_no_earlier_product_for_the_manifest_lint`
  failing beside the products build (it reads `target/niji`; saw libc.so.6 but no locale sources). The D-X.142 pair
  had the same race. `product_unit_tests` (text scan of xtask/src: 9 tests today) now skips those by name beside the
  build and runs them after it ("xtask unit tests on the fresh products"); 1158 tests ran both ways.

## Proof (mei, gate 16 GiB, 6 CPUs for the run -> 2 steps; products cleared, test binaries warm)

| tree | before (c79679e5d code) | after | job |
|---|---|---|---|
| libm (`#[inline]` on `signed`, crates/math/src/hyper.rs; 12 targets) | 445 s | 365 s (-18%) | affected-layout-rows-2-proof-libm (after = 93f2de0ad, same slot decision) |
| headers-stage2-gaps-b-4 cff3b4cac (5 targets) | 789 s (788 s in b4-2) | 737 s (-6.6%) | affected-layout-rows-2-proof-b4-3 (after = 9cbe6b2ff) |

Peak anon per proof job: 2.8 GiB (libm), 4.4 GiB (b-4). b4 proof binaries are b-4's own xtask with each
`affected.rs` swapped in (b-4 changes the header generator: a binary from this branch's xtask saw b-4's headers
as stale, job affected-layout-rows-2-proof-b4, SUPERSEDED). Scripts: `~/work/affected-layout-rows-2/` on mei.

## Open

- INFORMATIONAL: the gate's 6-CPU allotment holds a bounded run to two steps (`STEP_CPUS` 12); b-4's gain is small
  because its xtask tests (506 s) already ran beside the products. More steps need a larger CPU share per gate
  (ops' gate wrapper, not measured here).
- Plants: out of scope (brief item 5).

## Gate

affected-layout-rows-2-gate1 (94fbcfbb0, direct tier, mei): ok, 7 suites, 709 passed, 732 s, peak anon 2.85 GiB.
Lint: affected-layout-rows-2-lint3 (clippy + static) green; unit: affected-layout-rows-2-unit4 (118 passed).
Scratch worktrees removed.

## Next step

Land. Then, if wanted: measure a gate with a larger CPU share (more than two steps).
