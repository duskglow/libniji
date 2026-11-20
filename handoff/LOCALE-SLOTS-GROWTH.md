# LOCALE-SLOTS-GROWTH (audit HL-10): handoff

**State:** branch `task/locale-slots-growth-7` (run 7; stacks on `-6` .. `-1`, merge base with next `b2485a7f1`).
Code head `34cc0ded1`; HEAD: see the last commit. Report: `handoff/LOCALE-SLOTS-GROWTH-report.md` (run 7 section
first), `handoff/reports/LOCALE-SLOTS-GROWTH-7.json`. Plants: `handoff/LOCALE-SLOTS-GROWTH-7-plants.txt` (run 6's 21
plus 4 for run 7).

## Decisions in force
`records/pending/LOCALE-SLOTS-GROWTH-1.toml` (D-NEW, series D-7; amends D-7.7, D-7.19, D-X.13): no cap on locales,
names or files. There is a static table (1024 entries) plus spill chunks of `64 << k` that never move. Past 64
entries, lookups use chunked hash indexes that are never rehashed. ENOMEM is reported from `newlocale` and
`setlocale`. Paths that start-up never runs are cold and out of line. D-X.101 and D-X.103 are not touched.
Run 7 (ruling #6981): the lock-free setlocale answers make no returning call into spill code (see below).

## Run 7 (ruling #6981: setlocale +20 instructions per call against next)
- Cause (disassembly of the release `libc.a`, Zen 1 host): the query cache read the name through two `field()`
  lookups. Once that was one lookup (`cached_name`, commit `a1f6ccc83`), 7 to 9 instructions were still left: the
  call into the spill fallback is never taken, but because it returns into `setlocale`, `setlocale` saved four
  registers and kept a frame (next saves one, `push rbx`). The `""` rows were +4 to +6 for the same reason: the spill
  calls in `view()` and `query_cached_for` were inlined into `setlocale_env`. `spilled()` was already
  `#[cold] #[inline(never)]` on the inherited head.
- Fix (`34cc0ded1`): `setlocale` reads the cached name from the static table only. A name past it goes to a cold
  `setlocale_spilled` by a tail call. `setlocale_env_kept` reads the static table only, and leaves a spilled global
  locale or name to `setlocale_locked`, which gives the same answer.
- Test: the reload test (`locales_and_files_past_the_static_table_load`) now selects and queries l0 and l1099 a
  second time, which exercises the spilled lock-free answer. Bench: `cbench_locale` has new rows `setlocale_all_c`,
  `setlocale_query_en_us` and `setlocale_en_us_repeat`.
- Per call, release profile, columns glibc / next (`b2485a7f1`) / branch:
  - Zen 1 host (`locale-slots-growth-7-percall3`): `setlocale_query` 44 / 71 / 71; `setlocale_all_c`
    159.2 / 121.6 / 119.7; `setlocale_ctype_c` 159.1 / 126.3 / 124.3; `warm_setlocale_enUS` 159 / 137 / 135. The
    `""` rows are equal to next, and `setlocale_*_posix` are equal (+0.1 on ctype_posix).
  - Skylake-SP (mei `locale-slots-growth-7-percall-sk3`): `setlocale_query` 44 / 71 / 71; `setlocale_all_c`
    156.2 / 121.6 / 119.6; `setlocale_ctype_c` 156.1 / 126.3 / 124.3; `warm_setlocale_enUS` 156 / 137 / 135. The
    `""` and `_posix` rows are the same as on the Zen 1 host.
- Gate (`locale-slots-growth-7-gate`, keke n2): 14 suites ok, 10572 passed, 0 failed, conformance 36/36, peak 12G.
  Plants (`locale-slots-growth-7-plants`): 25 plants, 23 CAUGHT, 1 EQUIVALENT (spilled-name-skipped: the locked path
  returns the same name), 1 WARN-NO-MSG (env-kept-wrong-slot, caught; its `msg:` is fixed). Start-up checks:
  `locale-slots-growth-7-startup` (`cargo xtask test`, then the full-profile `so` and `hot_text_products`).
- `bench/instr_limits.tsv`: new rows `cbench_locale` `setlocale_all_c` 119, `setlocale_query_en_us` 70 and
  `setlocale_en_us_repeat` 135, for the Zen 1 host and Skylake-SP, at next's level. The bench counts
  (`-bench-zen`, mei `-bench-sk`) are 117.1, 69.0-69.1 and 133.1-133.2 instructions per call.

## Open
- (c) Archive-form locales share one file entry: from reading the code, not measured.

## Next step (exact)
The CI train. Nothing is left on this chain apart from the already queued STARTUP-GOT-PAGE layout story (ruling #6526).

## Do not redo
- Do not shrink locale code further (step-back decision). No fixed larger cap, no preallocation, no rehashing index.
- Runs 1-6 jobs are superseded by run 7's. Run 6's `so`, layout contracts and 7 of 7 start-up pages were not rerun.
- Do not put a returning call into spill code back on `setlocale` or `setlocale_env_kept`: even when it is never
  taken, it costs every call a frame or register moves.
