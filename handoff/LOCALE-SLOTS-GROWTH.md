# LOCALE-SLOTS-GROWTH (audit HL-10): handoff

**State:** branch `task/locale-slots-growth-6` (run 6; stacks on `-5` .. `-1`). HEAD: see the last commit; code head
`23993a813` = the merge of origin/next `b2485a7f1` into run 5's `1ed232a5a`. The merge is clean, and the run made no
product change after it (as the 11-18 and 11-20 coordinator notes ask). Report: `handoff/LOCALE-SLOTS-GROWTH-report.md`
(run 6 section first), `handoff/reports/LOCALE-SLOTS-GROWTH-6.json`. Plants: `handoff/LOCALE-SLOTS-GROWTH-6-plants.txt`.

## Decisions in force
`records/pending/LOCALE-SLOTS-GROWTH-1.toml` (D-NEW, series D-7; amends D-7.7, D-7.19, D-X.13): no cap on locales,
names or files. There is a static table (1024 entries) plus spill chunks of `64 << k` that never move. Past 64
entries, lookups use chunked hash indexes that are never rehashed. ENOMEM is reported from `newlocale` and
`setlocale`. Paths that start-up never runs are cold and out of line. D-X.101 and D-X.103 are not touched.

## Run 6 results (mei s3, Skylake-SP; head `23993a813`)
- The inherited head `1ed232a5a`: clippy and static green (`locale-slots-growth-6-lint0`).
- Merged head: clippy, static and `cargo xtask so` green (`locale-slots-growth-6-lint`). All 13 layout contracts
  hold. STARTUP-DATA-WINDOWS: 7 of 7 written pages. The six start-up `.got` objects (0x86b018-0x86b0a1) are on one
  page. Run 5's straddle does not occur on this merge.
- Direct gate `locale-slots-growth-6-gate`: 14 suites ok, 10572 passed, 0 failed; conformance 36/36; peak 14G.
- Plants `locale-slots-growth-6-plants`: 21 of 21 CAUGHT, no WARN lines.
- Start-up checks: `locale-slots-growth-6-xtest` (`cargo xtask test`) and `locale-slots-growth-6-hottext`
  (`NIJI_FULL_PROFILE=1` so + `hot_text_products`) both rc 0: conformance 140/140; hot_text_products 30 passed; the
  13 contracts also hold on the full profile.
- Trace: the strings group (strlen, strncmp, niji_machine_strncmp, niji_machine_compare) is in bin 2. Bin 2 is
  63,056 B (run 5: 63,061). The signature diff and fault counts against the new base were not rerun: that needs a
  base fast build. Per the 11-20 note, the R2 byte budgets in `so` are the per-story gate, and they pass.
- Per-call counts against next (`locale-slots-growth-6-percall`, release profile; columns glibc / next / branch):
  - `warm_setlocale_enUS` (setlocale(LC_ALL) of the loaded locale): 156 / 137 / 157 (+20).
  - `setlocale_all_c`: +20. `setlocale_ctype_c`: +20. `setlocale_*_posix`: +22.
  - `setlocale_query` (setlocale(LC_ALL, NULL)): 44 / 71 / 92 (+21).
  - The `_empty` rows: +2 to +4.
  - Cold first calls: unchanged at the tool's 200-instruction resolution.
  - `bench/instr_limits.tsv` has no row for these calls. Its setlocale row (`cbench_startup/startup_instr_setloc`)
    is for the Zen 1 CPU class only, so it cannot be checked on this host.

## Open
- RULING NEEDED: setlocale is +20 instructions per call on its repeat and query paths, against next. Likely cause,
  from reading the code (not checked by disassembly): `query_cached` resolves the slot twice through `slot()`, and
  each call inlines the spill fallback (`spilled`: `spill_place`, the `SPILL_AT` load, `slots_at`). Recommendation: a
  follow-up story (or a run of this one, if allowed) that reads both name fields through one lookup (`name_bytes`
  already does this) and moves `spilled` out of line as `#[cold]`. Then count again; the target is next's 71 and 137.
- (c) Archive-form locales share one file entry: from reading the code, not measured.

## Next step (exact)
Coordinator: decide the setlocale per-call ruling above. If a fix is allowed, change only `query_cached` and
`spilled` in `crates/legacy/src/global_locale.rs`, then rerun the plants file, the direct gate, the start-up checks
and `wl_percall.sh --spec validation/percall/setlocale.c` (and `cold.c --cases warm_setlocale_enUS`).

## Do not redo
- Do not shrink locale code further (step-back decision). No fixed larger cap, no preallocation, no rehashing index.
- Runs 1-5 jobs are superseded by run 6's.
