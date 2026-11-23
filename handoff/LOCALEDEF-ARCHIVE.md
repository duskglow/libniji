# LOCALEDEF-ARCHIVE: locale archive writer, the #7534 gaps, a freeing arena

**State.** Branch `task/localedef-archive` (from origin/next 8e1837914), role keke `~/w/n2`. Code head `3cc67e6ff`
(later commits: handoff, plant results, report only). Run 1 ended after leg 4 (no refreshes left). Records
D-31.23..26 (`records/decisions/D-31/`); three differences rows; no COMPAT (two RULING NEEDED below).
Sync: `bash ~/src/liella-ops/tools/sync_role.sh --story LOCALEDEF-ARCHIVE --base origin/next ~/w/LOCALEDEF-ARCHIVE keke n2`.

## What the branch does

* `localedef` writes the locale archive by default (NAME without `/`; `--no-archive` for directories, NAME's codeset
  normalized), `--list-archive [FILE]`, `--add-to-archive DIR...`, `--delete-from-archive NAME...`, `--replace`;
  flock on the archive directory; names with the charmap's codeset alias as the reference stores them (D-31.25,
  `crates/locale/src/archive.rs`, `prog.rs`, `crates/boundary/src/localedef.rs`).
* All 12 category files always written; undefined ones get defaults (D-31.24, `defaults.rs`); `-i` omitted reads
  stdin (`<stdin>` in messages); lines outside a category are reported and skipped; a category syntax error under
  `-c` writes output with status 1, without `-c` status 4 and no output (D-31.26); `--big-endian` refused (D-31.26).
* Core reader (`crates/core/src/locale/archive.rs` `entry`): locrec offset 0 = deleted name, absent.
* Freeing tool arena shared with ldconfig (`crates/boundary/src/ldconfig.rs`, D-31.23): power-of-two classes with
  free lists, big blocks own mappings. Peak RSS (KiB, before -> after): en_US 224640 -> 109552, de_DE 344160 ->
  111576, fr_FR 344160 -> 111560, ja_JP 234240 -> 49936, tr_TR 283680 -> 112540.
* reference.rs temp-dir race fixed (two tests shared pid+locale dirs).

## Results (all on code head 3cc67e6ff, keke n2)

* Gate `localedef-archive-gate` (direct tier, not escalated): 16 suites ok, 10074 passed, 0 failed, 1372 ignored;
  conformance 146 passed; peak 12G, rc 0.
* Plants `localedef-archive-plants` (`handoff/LOCALEDEF-ARCHIVE.plants`): 8 of 8 CAUGHT, no WARN-RC-ONLY/NO-MSG
  (one WARN-DEV-SHM, informational); results `handoff/LOCALEDEF-ARCHIVE-plants.txt`.
* Start-up checks: `localedef-archive-test` (`cargo xtask test`) and `localedef-archive-hot` (NIJI_FULL_PROFILE so +
  hot_text_products), started 01:06Z by `~/work/localedef-archive/startup.sh` (sequential). Result: see the report
  json / hand-back. `layout-bins.tsv` was NOT diffed against the merge base (no base products on keke; a base
  build costs a full products build); the only start-up-path change is one `filter` in the archive reader.
* E2E (leg 3, `localedef-archive-e2e`): the reference's `localedef --list-archive` lists libniji's archive; libniji
  `locale -a`/`locale -k` read it byte-identically to the directory form for 6 locales.

## Open

* RULING NEEDED (a) D-31.24: undefined categories get POSIX values where the reference's defaults differ; no compat
  option (recommend keep). (b) D-31.26: `--big-endian` refused; no compat option (recommend keep).
* INFORMATIONAL: the reference.rs temp-dir race is a flake on next too (fixed here).

## Next step

Land (CI train). If a start-up job is red: fix, rerun only that job, precheck, handback-check.
Probe material and the reference's archive behaviour: `spec/behaviour/utilities.glibc.txt` (§31.8 rows).
