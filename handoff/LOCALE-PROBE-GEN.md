# LOCALE-PROBE-GEN (chain; last run LOCALE-PROBE-GEN-3): locale data from black-box probes (#7791, #7890, #7892)

**State.** Branch `task/locale-probe-gen-3` (continues `task/locale-probe-gen-2`, origin/next merged in run 3), role
keke `~/w/r5`. Head: `git log -1`. Decision **D-31.30** (`records/decisions/D-31/D-31.30.toml`; extends D-31.27).
First run's report: `handoff/LOCALE-PROBE-GEN-report.md`. Plants: `handoff/LOCALE-PROBE-GEN-3.plants` (final head).
JSON: `handoff/reports/LOCALE-PROBE-GEN-3.json`. Remaining differences (pre-merge): `handoff/LOCALE-PROBE-GEN-2-diff.tsv`.

## Done (do not redo)

* Run 1: probe tools (`tools/probe/locale_dump.py`, `locale_fingerprint.c`), `spec/locale-probe/locales.tsv`,
  `fingerprints.tsv`; LC_NAME/ADDRESS/TELEPHONE of every name from the probe; 68 CLDR-less names from the probe.
* Run 2, decision #7890: every category after LC_COLLATE of every generated name the probe has comes from the probe
  (`probed_rest`); no CLDR value kept. #7892: `spec/locale-probe/ctype-diff.tsv` drives `ctype_edits`. Verified on
  mei: 349 sources compile (UTF-8), 39,566 probed values equal, 13 differ (`@euro` wide thousands separator); all
  LC_CTYPE fingerprints equal; 182 legacy entries compile, 251 values in 81 entries differ (no compile-time translit then).
* Run 3: CI's red (release start-up test `init_data_clusters_hold_at_every_text_phase`, rewritten on next by
  GATE-HONEST-METRICS) addressed by merging origin/next (coordinator note). Conflicts: `xtask/src/locales.rs`
  (`C` now from next's `c_source()`, D-31.31; every other name CLDR-plus-probe or probe alone), `tools/deb/manifest.tsv`
  (both rows), `crates/locale/tests/reference.rs` (both tests), `docs/differences-from-glibc.md` (this chain's rows;
  names-offered row: all 509 entries now generated, C.UTF-8 included; Locale sources row: localedef transliterates at
  compile time now (LOCALEDEF-TRANSLIT), the generated table lacks the data, 251 labelled as a pre-merge count).

## Run 3 jobs (keke)

* `locale-probe-gen-3-lint0`: clippy and static on the inherited head d4481acb1: green.
* `locale-probe-gen-3-lint2`: clippy (`--workspace --all-targets --all-features -D warnings`) and static: green.
* `locale-probe-gen-3-gate2` (direct tier; FULL is the train's: spec/locale-probe/*, xtask/src/*): 8 of 8 suites ok,
  705 passed, 0 failed, peak 6.8G.
* `locale-probe-gen-3-relht` (`NIJI_FULL_PROFILE=1 cargo xtask so` + `cargo test -p niji --test hot_text_products`,
  CI's red step): 36 passed, 0 failed (init_data_clusters_hold_at_every_text_phase included).
* `locale-probe-gen-3-plants`: 15 plants, 13 CAUGHT with their message, 0 SURVIVED; `merge-c-source` and
  `merge-probe-only` WARN-NO-MSG: caught by the products build's shipping check (a manifest-listed source missing,
  `locale-sources/C` / `locale-sources/aa_DJ: No such file`). msg widened; the two re-run alone as
  `locale-probe-gen-3-plants2` (results: `handoff/LOCALE-PROBE-GEN-3-plants.txt`, first run).
* Superseded: `locale-probe-gen-3-gate` (stopped in its prechecks: the head moved by a docs commit), `lint1` (older head).

## Not done / next

Nothing left in this chain. Follow-ups are their own briefs (coordinator queue):
1. #185 (#7957): the generator emits the shared transliteration include the LOCALEDEF-TRANSLIT probes established;
   re-measure the 251 legacy values (mei: `~/work/locale-probe-gen-2/allL.sh`, `cmpL.py` show the method) and drop the
   gap from the "Locale sources" differences row.
2. #188: per-locale collation tailorings wired into the generated sources (the 12 probe-only gap names included).

## Traps

* `locale -k` prints `era` / `alt_digits` as `"a";"b"` but fixed lists (`abday`) as `"a;b"`.
* The reference's `@euro` names are ISO-8859-15 with compacted `same-as` rows: compare after resolving the sibling
  and transcoding.
* The `same-as` plant breaks the products build (`None is not a number`), not the unit test: its `msg:` names both.
* A plant that stops a source being generated must regenerate into an emptied directory: `cargo xtask so` in the
  plants copy has already written every source, and `locale-sources` does not remove stale files.
* Reference locales are installed only on mei; keke has 4 (`locale -a`).
