# LOCALE-SLOTS-GROWTH (audit HL-10): handoff

**State:** branch `task/locale-slots-growth-4` (run 4; stacks on `-3`, `-2`, `-1`), test commit `dfffe5975` + this
note. Product code is unchanged since run 3 (`df9ab2569`); run 4 changed only `crates/boundary/tests/reload.rs` and the
plants file. Report: `handoff/LOCALE-SLOTS-GROWTH-report.md`, `handoff/reports/LOCALE-SLOTS-GROWTH-4.json`.

## Decisions in force
`records/pending/LOCALE-SLOTS-GROWTH-1.toml` (D-NEW, series D-7; amends D-7.7, D-7.19, D-X.13): no cap on locales,
names or files. There is a static table (1024 entries, layout unchanged) plus spill chunks of `64 << k` that never
move. Past 64 entries, lookups go through chunked hash indexes that are never rehashed. ENOMEM is reported from
`newlocale` and from `setlocale` (`last_failure()`). A re-map with identical bytes after an invalidation keeps the old
mapping. Paths that start-up never runs are out of line and cold: `spill_for_write`, `spilled`, `tabulated`,
`note_failure`, `Files::revisit`, the hashes.

## Run 4 (LOCALE-SLOTS-GROWTH-4): the two plant gaps of run 3
- `nomem-not-noted` survived run 3: `note_failure` with the plant noted ENOENT instead of ENOMEM, and every load meets
  some missing path, so `missing()` still said ENOMEM. The test never checked the converse. Now
  `a_load_that_runs_out_of_memory_fails_with_enomem` also checks that a missing locale gives ENOENT from `newlocale` and
  `setlocale` after the out-of-memory phase. CAUGHT (`newlocale of a missing locale after ENOMEM: ENOENT`).
- `spill-read-offset` was WARN-NO-MSG in run 3: the 23 extra mappings came from `setlocale(l1099)`, which ran after
  `made` was measured, so the failure blamed the re-load of l0. Now `locales_and_files_past_the_static_table_load`
  selects and re-loads both l0 (static entries) and l1099 (spilled entries). Each step is checked for no new mapping,
  no new interned entry and the same interned view, and the message names the locale. CAUGHT
  (`l1099, already loaded, is selected without mapping`). The 5 index plants' messages changed with the test
  (`l<k> loaded again maps nothing`); the plants file expects that.
- Run 3's own gate and start-up jobs never completed: its chain in keke role n1 was still on the gate (run 3's head)
  when run 4 started. Run 4 stopped it (the gate shows rc 124 in jobs.tsv) and ran its own gate and start-up checks on
  the final head in n3. pfbranch2 (run 3) measured the start-up tool fault count at 44 on the run-3 code (bound 44).

## Open
- (c) Archive-form locales share one file entry: from reading the code, not measured.
- The 1100-locale counts (report) were measured on run 2's code. Past 64 entries a lookup now calls the out-of-line
  hash functions; re-measure only if the coordinator wants the figures.

## Next step (exact)
Run 4's jobs on keke (`~/bin/jobwait locale-slots-growth-4-plants locale-slots-growth-4-gate
locale-slots-growth-4-startup`). If all are green, land. A red in the gate or start-up: fix it and rerun only that job.
A start-up fault or window red follows the ALWAYS FORWARD rule (attribution recipe in the report, run 3).

## Do not redo
- No fixed larger cap, no preallocation; the static table stays (STARTUP-DATA-PAGE-ALIGN-3). No rehashing index.
- The run 2 counts are measured; do not re-measure unless lookup code changes.
- Run 1-3 jobs (`locale-slots-growth-{2,3}-*`) are superseded.
