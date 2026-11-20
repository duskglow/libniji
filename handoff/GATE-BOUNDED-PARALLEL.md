# GATE-BOUNDED-PARALLEL (handoff, end of leg 2)

State: branch `task/gate-bounded-parallel`, code head 51d0c1b22 (later commits: handoff/ only), base origin/next
189ab9cbc. Role keke:~/w/r3, synced to the code head.

Decisions in force: D-X.142 (reserved; `records/decisions/D-X/D-X.142.toml`): the five train classes of GATE-DEV-NO-FULL and the side-by-side rule, with the measurements.

Done:
- `xtask/src/affected.rs`: side-by-side products build + xtask tests in a non-FULL run that selects both (memory
  guard `SIDE_BY_SIDE_BYTES` = 6 GiB from the measured peaks; `/proc/self/exe so` to avoid the `target/debug` lock
  wait; xtask targets listed from `xtask/tests`; `transcripts` after the products). 3 unit tests, 8 plants.
- Profile of the xtask tests and before/after of one replayed branch: `handoff/GATE-BOUNDED-PARALLEL-report.md`.

Do not redo: the before row (origin/next), the profile, the after row of `layout-ldso-order-3-gate`.

Open: the gain is 60 s (4.1%) of the two steps in a 4-core slot, short of the brief's "a third"; the premise's
1,118 s of xtask tests included their compile and load (190 s warm). `startup-got-page-4-gate` not measured.

Next step: the coordinator's (landing). If more rows are wanted: on keke, `~/work/gate-bounded-parallel/meas.sh
after startup-got-page-4-gate` under `~/bin/gate 16G` (role at this branch); a serial row of the same branch runs
under `~/bin/gate 5G` (the guard then takes the one-after-the-other path).

Plan rows to read: none beyond the record above; GATE-DEV-NO-FULL's report for the class table.
