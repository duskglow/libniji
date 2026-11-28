# PIPELINE-TEST-TIME-1127: handoff

State: branch `task/pipeline-test-time-1127` from origin/next af6f8acaf. Code commit 750cace21 (`xtask/src/so.rs`
`EarlyData`, `write_data_products`); report `handoff/PIPELINE-TEST-TIME-1127-report.md`; plants results
`handoff/PIPELINE-TEST-TIME-1127-plants.txt`; record `handoff/reports/PIPELINE-TEST-TIME-1127.json`. Role mei:~/w/s1.

## Result

Where the time went since 10-19, measured: not more or longer tests (full-gate test-seconds +7%, plants per job
unchanged at a median of 6), but builds. Plants jobs: ~70% product builds, of which a cold `cargo xtask so` per copy
is ~45% (each run is a new directory without `target/`). The CI gate's +9 min: +7 min from the release-profile
start-up check added on 11-13, +5.6 min inside the full run (products step and 338 test binaries against 192).
Agent legs: +37 min median, of which +39 min are job waits; model time +3 min.

Fix landed here: the data products (charmaps, locale charmaps, locale sources) are written beside the library
builds: cold copy build 273 -> 254 s, no-op 64 -> 52 s (Zen 1 48-CPU host, two rounds each, identical products).

## Decisions in force

No new D-row: a local build-order detail of `cargo xtask so` (the outputs and checks are unchanged). Related:
PLANTS-BUILD-SLOWDOWN's `EarlyBuilds` (same pattern), D-X.177 (plants build products only where read).

## Open (RULING NEEDED; table in the report, section 5)

- Plants warm copy per role (saves the cold baseline: 6.8-14.5 min median per job): changes the new-dir rule and
  adds a cache; recommend yes as its own story.
- CI gate's release-profile start-up check: run it in the ci-instr job (which builds the release products for the
  same tip) or beside the full run: ~7 min per train. The ops repo is the coordinator's; not changed here.
- Skip unchanged post-cargo product assembly in `cargo xtask so` (80-110 s per call): a cache; ruling first.
- `--copies auto` by CPUs too; `cargo xtask bench` step lines (ci-instr has 12 min not split).

## Not to redo

The data pulls (scripts in the scratch dir and `~/work/pipeline-test-time-1127/` on mei: `plants_parse.py`,
`ci_parse.py`, `affected_parse.py`, `instr_parse.py`, `legs.py` reads the coordination host's session logs). The
per-plant logs of old runs are pruned; the 10-19 logs have no phase lines.

## Next step

Land. Then the coordinator routes the rulings above. Read: report sections 1-5; `xtask/src/so.rs` `EarlyData`.
