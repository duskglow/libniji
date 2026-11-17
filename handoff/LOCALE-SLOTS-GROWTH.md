# LOCALE-SLOTS-GROWTH (audit HL-10): handoff

**State:** branch `task/locale-slots-growth`, code commit `8352fe7eb` (WIP), stopped at the context checkpoint
(~150k) after run 1. Brief: `~/state/briefs/LOCALE-SLOTS-GROWTH.md`. Decision: `records/pending/LOCALE-SLOTS-GROWTH-1.toml`
(D-NEW, series D-7; amends D-7.7, D-7.19, D-X.13).

## Done (run 1)
- `crates/legacy/src/global_locale.rs`: `CAPACITY`, `FILES`, `MAP_LIMIT` are gone. The static page-aligned table
  (`INLINE` = 1024 entries, unchanged layout) is the common case; entries past it go to spill chunks of `64 << k`
  entries (`SPILL_AT`, 40 words), allocated fallibly under `LOCK` before the entry's count is published, never
  moved or freed. Hot readers keep one bounds check; the spill read is `#[cold]` (`spilled`). The POSIX layer turns
  a chunk address into `&[Slot]` (`Objects::slots_at` -> `tb::locale_slots_at`, UNSAFE(slice)).
- Errors: `Mapper::map` returns `KResult`; `Files` notes ENOMEM (`nomem`); `replace`/`replace_changed` return
  `KResult` (ENOENT or ENOMEM); `newlocale`, `newlocale_env` and `invalidate_and_reload` pass the cause on; a failed
  `setlocale` records it (`FAILED_NOMEM`, `last_failure()`), and `setlocale_failed` in `boundary/src/locale.rs` sets it.
- (b) After an invalidation, a replaced file whose new bytes equal the mapping in use: the new mapping is unmapped
  and the old one kept (new identity recorded). Before this change the old mapping was never unmapped and every re-map counted against 2048.
- (c) Archive-form locales: `find` reaches the archive through ONE `src.file(ARCHIVE)` (core/src/locale/mod.rs:697)
  and the table is path-keyed, so all archive locales share one file entry. This comes from reading the code and is
  NOT measured (the build hosts do have a 4-locale archive).
- Tests (`crates/boundary/tests/reload.rs`, replacing `the_locale_file_table_is_bounded`):
  `locales_and_files_past_the_static_table_load` (1100 links to C.utf8: loaded 1100, 13,200 mappings),
  `a_load_that_runs_out_of_memory_fails_with_enomem` (RLIMIT_AS: 38 loaded, then ENOMEM; setlocale ENOMEM),
  `repeated_updates_that_change_nothing_grow_nothing` (5000 identical replacements + invalidations; mappings flat).
- Reference probe (1100 linked C.utf8 locales via newlocale): 1100 of 1100 load.

## Results
- Job `locale-slots-growth-dev3` on keke: workspace clippy clean, the four reload tests pass on the code of `8352fe7eb`.
- **Open defect: the 1100-locale test takes 205 s** on the Zen 1 build host: name interning (`intern_name_locked`), file lookup
  (`Files::file`) and `intern_locked` are linear scans, quadratic over ~14k names and files.

## Next step (exact)
1. Add a lock-held hash index (names, and files by name slot) used ONLY past the static table (or past a count
   threshold), so the common case's instructions and pages do not move; re-run the 1100 test (target: seconds).
2. `cargo xtask static`; plants: `cargo xtask plants handoff/LOCALE-SLOTS-GROWTH-plants.txt --dir /dev/shm/claude/LOCALE-SLOTS-GROWTH/plants-1`.
3. Start-up checks (the brief's 2026-11-14 rule): `cargo xtask test` and the `NIJI_FULL_PROFILE=1` hot_text_products
   run; the start-up instruction and fault rows against `bench/instr_limits.tsv`. `SPILL_AT` and `FAILED_NOMEM` are new
   `.bss` statics, so watch for layout jitter.
4. Direct-tier gate `cargo xtask affected --run --base origin/next`; the same probe against libniji (probe source in
   the run's scratch; re-create it from the test's setup: 1100 symlinks to C.utf8, newlocale(LC_ALL_MASK) each).
5. Drop "WIP:" with a final commit.

## Do not redo
- Do not keep a fixed larger cap; do not preallocate. The static table stays as it is (STARTUP-DATA-PAGE-ALIGN-3).
