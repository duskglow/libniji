# LOCALE-SLOTS-GROWTH (audit HL-10): handoff

**State:** branch `task/locale-slots-growth-2` (stacks on `task/locale-slots-growth`), run 2 (LOCALE-SLOTS-GROWTH-2)
stopped at the context checkpoint with all code written and verification jobs running on keke role n3.
Brief: `~/state/briefs/LOCALE-SLOTS-GROWTH.md` (spec), `~/state/briefs/LOCALE-SLOTS-GROWTH-2.md`. Decision:
`records/pending/LOCALE-SLOTS-GROWTH-1.toml` (D-NEW, series D-7; amends D-7.7, D-7.19, D-X.13). Plants:
`handoff/LOCALE-SLOTS-GROWTH-2-plants.txt` (16: the 7 of run 1 and 9 for the indexes).

## Done
- Run 1: `crates/legacy/src/global_locale.rs` lost `CAPACITY`, `FILES`, `MAP_LIMIT`: static table (1024 entries,
  layout unchanged) plus never-moved spill chunks of `64 << k`; ENOMEM reported (`newlocale`, `setlocale` via
  `last_failure()`); a re-map with identical bytes after an invalidation is unmapped and the old mapping kept.
- Run 2 (the 205 s defect): lookups past 64 entries go through hash indexes (`INDEX`, `index_find`,
  `index_reserve`/`index_grow`, `index_add`): interned names by bytes, files by path, locales by category names, and
  (`TABS`) the view tables a new locale takes from an earlier one (LV-3) by category file address. Index chunks cover
  entries `0..64`, then `64 << (c-1)` from `64 << (c-1)`; each is an open-addressed table (tag in the top 16 bits,
  offset + 1 below) allocated fallibly before its first entry is written, never moved or freed. Below 64 entries the
  scans are unchanged. `Objects::words_at` -> `tb::locale_words_at` (UNSAFE(slice)).
- `names_interned()` / `locales_interned()` (like `mappings()`) so the 1100 test checks that a re-load interns nothing.
- Tests (`crates/boundary/tests/reload.rs`): the 1100 test now also checks the re-loaded locale is the same interned
  view and name; new `locales_past_the_scanned_entries_share_category_tables` (80 composite locales share l0's tables).

## Results (keke, Zen 1, instructions:u per call, static release products, probe many_locales_cost.c in the run's scratch)
| case | before (inherited head) | after | reference |
|---|---|---|---|
| at 1 loaded: newlocale new / loaded | 146,266 / 102,575 | 144,517 / 102,534 | 276,086 / 52,995 |
| at 1 loaded: setlocale new / loaded | 159,905 / 103,637 | 156,504 / 103,693 | 306,487 / 45,700 |
| at 1100 loaded: newlocale new / loaded | 54,755,736 / 42,614,426 | 192,637 / 135,035 | 70,330,267 / 2,555,722 |
| at 1100 loaded: setlocale new / loaded | 55,771,920 / 43,574,006 | 195,831 / 136,999 | 70,725,624 / 2,668,040 |
| loading 1097 more (wall, informative) | 7.6 s | 0.32 s | 30.2 s |
- The 4 reload tests: 5.9 s together (release, job `locale-slots-growth-2-dev1`). Lint on the final tree: rc 0
  (`locale-slots-growth-2-lint1`).
- Jobs started at hand-back (not yet read): `locale-slots-growth-2-gate` (direct tier, `--base origin/next`),
  `locale-slots-growth-2-plants`, `locale-slots-growth-2-startup` (`cargo xtask test`, then the NIJI_FULL_PROFILE
  hot_text_products check; it waits for the gate to leave the role). Logs: `~/logs/<label>.log` on keke.

## Open
- (c) archive-form locales share one file entry: from reading the code, not measured.
- The installed probe tool (`~/bin/probe`) looks for `target/liella/liella-crt1.o`; this tree builds
  `target/niji/niji-crt1.o`, so it rebuilds and dies. Measured instead with a direct static link (the run's measure.sh).

## Next step (exact)
1. Read the three jobs (`~/bin/jobwait locale-slots-growth-2-gate locale-slots-growth-2-plants
   locale-slots-growth-2-startup` on keke). All rc 0, plants all CAUGHT: hand back COMPLETE with those JOBS.
2. A red: fix it, rerun only that job. A start-up fault/window red on code this story touches: ALWAYS FORWARD rule.

## Do not redo
- No fixed larger cap, no preallocation; the static table stays (STARTUP-DATA-PAGE-ALIGN-3). No rehashing index:
  the chunked index never moves. The before/after counts above are measured; do not re-measure unless code changes.
