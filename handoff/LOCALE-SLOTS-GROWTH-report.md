# LOCALE-SLOTS-GROWTH (audit HL-10): full report, runs 1-4

Briefs (spec): `~/state/briefs/LOCALE-SLOTS-GROWTH.md`, `-2`, `-3`, `-4`. Decision: `records/pending/LOCALE-SLOTS-GROWTH-1.toml`.
Plants: `handoff/LOCALE-SLOTS-GROWTH-4-plants.txt` (20; earlier files `-plants.txt`, `-2-plants.txt`, `-3-plants.txt`).

## Run 4 (tests only)
- Commit `dfffe5975` (crates/boundary/tests/reload.rs, plants file).
  - `a_load_that_runs_out_of_memory_fails_with_enomem`: after the out-of-memory phase, a missing locale fails with
    ENOENT from `newlocale` and `setlocale` (only running out of memory is reported as ENOMEM).
  - `locales_and_files_past_the_static_table_load`: l0 and l1099 each selected and loaded again, with mappings,
    interned names and locales, the interned view and the name checked per locale. Before this, the mapping count was
    taken before the `setlocale` calls, and only l0 (all static entries) was loaded again.
- Jobs (keke, role n3): `locale-slots-growth-4-lint0` (clippy and static on the inherited head: rc 0),
  `locale-slots-growth-4-dev1` (reload suite on the new tests: 10 passed, none skipped),
  `locale-slots-growth-4-plants` (20 plants), `locale-slots-growth-4-gate` (direct tier),
  `locale-slots-growth-4-startup` (`cargo xtask test`, then NIJI_FULL_PROFILE `so` and `hot_text_products`),
  Results: lint0 rc 0; dev1 rc 0; plants rc 0 (20 CAUGHT, no WARN); gate rc 1 (two hot_text_products start-up text
  tests: strings mode 1 fault; see the handoff); `locale-slots-growth-4-basepf` (base: 0 faults, rc 0) and
  `locale-slots-growth-4-pfstr` (attribution) are diagnostics. The start-up job never ran (chain stopped after the
  red gate, so the gate's products could be attributed); no lint1 (no code changed after the red).
- Run 3's chain in n1 (`~/work/LOCALE-SLOTS-GROWTH-3/chain.sh`) was still running its gate on run 3's head. Run 4
  stopped it (chain by PID, then `kill-label`, then `kill-mine n1`); its gate row reads rc 124 and is superseded.

## Run 3: the start-up fault red
- Red: `hot_text_products init_coreutils_shaped_program_stays_within_its_page_fault_bound`, fast products 45 > 44.
  Attribution (pf.py from `handoff/LAYOUT-TOOL-BOUND-SHARED-tools/`; copy in `~/work/LOCALE-SLOTS-GROWTH-3/` on keke):
  the one new fault was the text page of `bindtextdomain`. The traced locale tier grew about 3.7 KB, window bin 2
  overflowed, and the bin planner (D-X.103) moved `bindtextdomain` into bin 1.
- Fix (`df9ab2569`, no bound touched): `slot_for_write` inline over the static table, spill allocation in cold
  `spill_for_write`; hashes cold out of line; `intern_locked` scans as the base did, with the TABS lookup in cold
  `tabulated`; `Files::recorded` notes ENOMEM in cold `note_failure`; a known file's first use after an invalidation
  goes through cold `Files::revisit`; `replace`/`replace_changed` return `Option`, and the cause comes from
  `Files::missing()`.
- `locale-slots-growth-3-pfbranch2`: tool mode 44 in every run (bound 44). Plants: 18 CAUGHT, 1 SURVIVED, 1
  WARN-NO-MSG (both closed in run 4).

## Runs 1-2
- Run 1: `crates/legacy/src/global_locale.rs` lost `CAPACITY`, `FILES` and `MAP_LIMIT`. Added the static table plus
  spill chunks, ENOMEM reporting, and identical re-map reuse.
- Run 2: hash indexes past 64 entries (`INDEX`, `index_find`, `index_reserve`/`index_grow`, `index_add`) for interned
  names, files, locales and view tables (TABS). Index chunks are open-addressed, allocated fallibly before their first
  entry is written, and never moved or freed. Added `names_interned()` and `locales_interned()`.

## Results (run 2 code; Zen 1, instructions:u per call, static release products)
| case | before | after | reference |
|---|---|---|---|
| at 1 loaded: newlocale new / loaded | 146,266 / 102,575 | 144,517 / 102,534 | 276,086 / 52,995 |
| at 1 loaded: setlocale new / loaded | 159,905 / 103,637 | 156,504 / 103,693 | 306,487 / 45,700 |
| at 1100 loaded: newlocale new / loaded | 54,755,736 / 42,614,426 | 192,637 / 135,035 | 70,330,267 / 2,555,722 |
| at 1100 loaded: setlocale new / loaded | 55,771,920 / 43,574,006 | 195,831 / 136,999 | 70,725,624 / 2,668,040 |
| loading 1097 more (wall, informative) | 7.6 s | 0.32 s | 30.2 s |

## Open / informational
- Archive-form locales share one file entry: from reading the code, not measured.
- The installed `~/bin/probe` looks for `target/liella/liella-crt1.o`, but this tree builds `target/niji/niji-crt1.o`.
  Measured with a direct static link instead.
