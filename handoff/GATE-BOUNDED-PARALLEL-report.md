# GATE-BOUNDED-PARALLEL report

Branch `task/gate-bounded-parallel`, base origin/next 189ab9cbc. Premise: GATE-DEV-NO-FULL's split of a bounded
(Xtask class) run on keke, products build 1,577 s and xtask unit tests 1,118 s one after the other.

## What changed

- `xtask/src/affected.rs` `run_selection`: a non-FULL run that selects both the xtask tests and the products runs
  `cargo test -p xtask --bins --test <each xtask target that runs no products>` BESIDE the products build, then
  `cargo test -p xtask --test transcripts` (targets whose source names products, `names_products`) on the fresh
  products. Before, `transcripts` ran BEFORE the products build, on the previous build's products.
  - Side by side only when `memory_room()` (smallest cgroup v2 `memory.max` on the path, and MemTotal) holds
    `SIDE_BY_SIDE_BYTES` = 6 GiB; else products first, then the same two test commands.
  - Side by side, the products build runs `/proc/self/exe so` (the running, fresh xtask), not `cargo xtask so`:
    the alias is `cargo run -p xtask -- so`, which would wait for the `target/debug` lock `cargo test -p xtask`
    holds while compiling.
  - The xtask targets are listed from `xtask/tests` by cargo's rule (`tests/*.rs`, `tests/*/main.rs`):
    `Repo::load` keeps no xtask targets. The leg-1 split read them from `repo.targets` and would have dropped all four
    xtask integration targets; the new unit test caught it before any gate ran it.
  - Output stays per step: the tests write `target/affected-xtask.log`, printed and appended to `target/affected.log`
    when both end; verdict = both green (as before).
- Decision record D-X.142 (reserved with `ops reserve`; `records/decisions/D-X/D-X.142.toml`): the five
  train classes, what each selects and why, the train's FULL gate covering the rest, and the side-by-side rule with
  its measurements. The module doc links it and holds the rules.
- 3 unit tests (rendezvous: both steps really overlap, verdict table, logs in one piece; the split on a fixture tree
  and the live tree; the cgroup limit and the threshold). 8 plants in `handoff/GATE-BOUNDED-PARALLEL-plants.txt`.

Nothing a class selects changed; `--tier full`, the train's gate, `--replay`, `--reverse` untouched.

## Profile of the xtask unit tests (keke, nextest, 4-core slot, warm build)

548 tests, 5 binaries, 277 s wall, 1,082 test-seconds, max RSS 613 MB. Per binary (test-seconds): `bin/xtask`
1,045.5 (536 tests), `header_p2_fixed_rows` 26.3 (1), `affected_mode` 9.8 (3), `probe_client` 0.4, `transcripts` 0.3.
Top tests: `affected::csys_falls_back_to_full_when_unsure` 105.0 s (the critical path), `layout_manifest_names_select
_the_layout_check` 50.6, `per_fix_tiers_leave_full_to_the_train_with_a_bounded_set_per_class` 41.3,
`bench::counters_files_hold_only_their_own_runs_rows` 36.3, `affected_run_runs_the_selection_and_list_does_not` 26.4,
`header_p2_fixed_rows::p2_visibility_of_the_fixed_rows` 26.3, the two `benchstage` staged-tree tests 24.2 each.
Not fixed (not cheap): the affected tests are slow because a direct-tier selection follows every user of a symbol
(~20 s per selection; xtask is already opt-level 2 in test builds and GATE-MATH-SHARD already removed the repeated
selections). A faster selection is its own story. Under `cargo test` (libtest threads) the tests took 190 s.

## Measurements (keke, Zen 1, each run in a 4-core slot of the shared queue; cold products, warm xtask test build)

`layout-ldso-order-3-gate`'s file list, `cargo xtask affected --run --tier direct` (meas.sh):

| step | before (origin/next, serial) | after (side by side) |
|---|---|---|
| xtask unit tests | 190.2 s | 280.3 s, inside the products build |
| products (cargo xtask so) | 1,275.2 s | 1,404.8 s |
| two steps together | **1,465.4 s** | **1,406.1 s** (+1.3 s transcripts after): **-60 s, -4.1%** |
| anon peak (gate cgroup) | 1.85 GiB | 2.27 GiB |
| run total | 2,425 s (rc 0) | 1,664 s (rc 1: static, record format, fixed in 51d0c1b22) |

The run totals differ by 761 s, mostly outside the changed steps (warm clippy 77 -> 0.6 s, `test -p niji` 216 -> 142 s,
~600 s of untimed cheap steps in the before run vs ~67 s after): not claimed. The gain is small because the
slot gives the run 4 cores, which the products build already fills; the brief's "a third more" assumed 1,118 s of
xtask tests, which included their compile and host load (190 s warm here). A run that must also compile the xtask
tests (an xtask change) overlaps that compile too: not measured. `startup-got-page-4-gate`: not measured (time).

## Results

- Unit tests (new 3): green (`gate-bounded-parallel-unit2`, with `cargo xtask clippy`).
- Plants `gate-bounded-parallel-plants`: 8 CAUGHT, 0 SURVIVED, no WARN lines (results in the plants file).
- Direct gate `gate-bounded-parallel-gate` (xtask unit tests 200.1 s + `cargo xtask static` 61.1 s; no products
  selected): 5 suites ok, 551 passed, 0 failed; peak 1.2G. Workspace clippy (`--all-targets --all-features -D
  warnings`) clean in the foreground on the role.
- The after row (`gate-bounded-parallel-after`) exercised the side-by-side path end to end: all 6 suites ok (587
  tests); its only red was `cargo xtask static` rejecting the record body's newlines, fixed in 51d0c1b22 (the gate
  above runs static green on it).
- SUPERSEDED: `gate-bounded-parallel-unit` (the split test failed: the leg-1 split found no xtask targets; fixed).

## For the coordinator

- INFORMATIONAL: with 4-core slots, running two CPU-heavy steps side by side gains little; the larger lever for a
  bounded run is the products build itself (1,275-1,577 s cold) and the ~20 s direct-tier selection in the affected
  unit tests (105 s critical-path test).
