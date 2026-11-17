# LOCALE-SLOTS-GROWTH (audit HL-10): handoff

**State:** branch `task/locale-slots-growth-4` (run 4; stacks on `-3`, `-2`, `-1`), test commit `dfffe5975` + this
note. INCOMPLETE: plants 20/20 CAUGHT, but the direct gate is RED on a start-up text fault (below). Product code is unchanged since run 3 (`df9ab2569`); run 4 changed only `crates/boundary/tests/reload.rs` and the
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

## Run 4: gate RED on the start-up layout (product code of run 3), NOT FIXED
- `locale-slots-growth-4-gate` (keke n3, rc 1): `hot_text_products init_first_string_calls_after_printf_take_no_text_fault`
  (1 fault, want 0) and `init_zero_fault_calls_hold_at_every_text_phase` ("pre 0x800/0x2000/0x8000: strings 1
  faults"). The same tests fail under exports interposed. Everything else is ok.
- The base (merge base 7f7e6c9e4, run 3's products, copied to `/dev/shm/claude/LOCALE-SLOTS-GROWTH-4/base`) gives 0
  faults in every mode and pad (`locale-slots-growth-4-basepf`). So the fault comes from this story; ALWAYS FORWARD
  applies (it touches setlocale).
- Attribution (`locale-slots-growth-4-pfstr`, pf.py `strings` mode; output in `~/work/LOCALE-SLOTS-GROWTH-4/out/`): the
  new fault is `strlen`'s text page in the 64 KiB window 0x420000. The bin planner (`target/product/fast/layout-bins.tsv`,
  D-X.103) moved the whole strings group (strlen, strcpy, strncmp, niji_machine_compare, memchr entry/rest: 6129 B)
  from bin 2 (printf's window) to bin 1, which strings mode does not otherwise touch. Scripts:
  `~/work/LOCALE-SLOTS-GROWTH-4/{bins.py,bins2.py,bins3.py,mapdiff.py}`; diff `out/bins2.txt`.
- Bin 2's traced bytes: base 62,317 (with the strings group). Branch: 56,277 without it, so 62,406 with it (+89 B).
  The locale tier grew: `intern_name_locked` +293, `intern_locked` +139, `publish_global` each_block +133, `replace`
  +84, `setlocale_locked` +81, `setlocale_env` +58, Glue +27; the `Files` Source impl went 1594 -> 1055.
- HYPOTHESIS (not verified): the trace signatures changed too. `niji_machine_compare` went from
  `rebind,rebind.pre,strings` to `mktime,rebind,...`, and `niji_sys_newfstatat` from `rebind` to `mktime,rebind`, and
  new mktime-only sections appear in bin 1. So on the branch the mktime mode reaches a stat and a compare that it
  did not reach on the base. If so, the planner may group the strings sections with mktime's bin 1 because of the
  signature, not only for size.

## Open
- (c) Archive-form locales share one file entry: from reading the code, not measured.
- The 1100-locale counts (report) were measured on run 2's code. Past 64 entries a lookup now calls the out-of-line
  hash functions; re-measure only if the coordinator wants the figures.

## Next step (exact)
1. Find why the mktime mode now touches `newfstatat` and `niji_machine_compare` (crates/boundary/src/tb.rs and the
   Mapper identity/stat paths are shared with zone loading; check whether a change here runs on mktime's tz load).
   If it is unintended, remove it, then check whether the strings group returns to bin 2.
2. Otherwise, shrink the locale tier's traced text by about 100+ B (for example, move the index branches of
   `intern_name_locked` / `intern_locked` out of line and cold, as run 3 did for the others) until bin 2 holds the
   strings group again. Never touch a bound or the planner.
3. Rebuild (`cargo xtask so`), run pfstr.sh (strings 0) and the two hot_text tests, then the direct gate, the start-up
   checks (`cargo xtask test`; NIJI_FULL_PROFILE `so` + `hot_text_products`) and plants (run-4 file) on the new head.

## Do not redo
- No fixed larger cap, no preallocation; the static table stays (STARTUP-DATA-PAGE-ALIGN-3). No rehashing index.
- The run 2 counts are measured; do not re-measure unless lookup code changes.
- Run 1-3 jobs (`locale-slots-growth-{2,3}-*`) are superseded.
