# GATE-BOUNDED-PARALLEL (handoff, end of leg 3: GATE-BOUNDED-PARALLEL-2)

State: branch `task/gate-bounded-parallel-2` (same commits as `task/gate-bounded-parallel` plus handoff/ only),
code head 1f7e3c438 (later commits: handoff/ only), base origin/next 189ab9cbc. Role keke:~/w/n4, synced.

Decisions in force: D-X.142 (`records/decisions/D-X/D-X.142.toml`): the five train classes of GATE-DEV-NO-FULL and
the side-by-side rule, with the first row's measurements.

Done:
- `xtask/src/affected.rs`: side-by-side products build + xtask tests in a non-FULL run that selects both (memory
  guard `SIDE_BY_SIDE_BYTES` = 6 GiB; `/proc/self/exe so`; xtask targets listed from `xtask/tests`; `transcripts`
  after the fresh products). 3 unit tests, 8 plants (`handoff/GATE-BOUNDED-PARALLEL-plants.txt`).
- Leg 3: clippy (workspace, all targets, -D warnings) and `cargo xtask static` green on the inherited head; gate,
  plants and unit rerun on code head 1f7e3c438 (the D-X.142 id commit) under new labels, all green; the second
  replayed row (`startup-got-page-4-gate`) measured, serial vs side by side on the same head. Details:
  `handoff/GATE-BOUNDED-PARALLEL-report.md` ("Leg 3").

Result: two rows, -60 s (-4.1%) and +8 s (+0.6%) on the two steps; sum -51 s (-1.8%). The products build's own
run-to-run spread (1,126-1,405 s) is larger than either difference. Target ("a third") not reached.

Open (RULING NEEDED, in the hand-back): keep the side-by-side path, or land only the `transcripts`-after-products
ordering and the D-X.142 record and drop the concurrency (recommended: drop it; simpler and no measured gain in a
4-core slot). The record holds only row 1; row 2 should be added to it if the side-by-side path stays.

Do not redo: the profile, row 1 (before and after), row 2 (serial and side by side), the leg-3 reruns.

Next step: the coordinator's ruling above. If the concurrency is dropped: remove the `beside` branch of
`run_selection` (keep `names_products` and the after-products order), drop the side-by-side unit test and plants
`gbp-serial`, `gbp-verdict-tests-lost`, `gbp-log-not-appended`, `gbp-cgroup-last-wins`, `gbp-room-off-by-one`, and
edit D-X.142's side-by-side paragraph to the ordering rule with both rows as the reason.

Plan rows to read: none beyond the record above; GATE-DEV-NO-FULL's report for the class table.
