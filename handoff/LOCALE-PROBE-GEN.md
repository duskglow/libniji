# LOCALE-PROBE-GEN (chain; last run LOCALE-PROBE-GEN-2): locale data from black-box probes (#7791, #7890, #7892)

**State.** Branch `task/locale-probe-gen-2` (continues `task/locale-probe-gen`, stacked on `task/locale-sources-gen-2`),
role mei `~/w/s2`. Head: `git log -1`. Decision **D-31.30** (`records/decisions/D-31/D-31.30.toml`; extends D-31.27).
First run's report: `handoff/LOCALE-PROBE-GEN-report.md`. Plants: `handoff/LOCALE-PROBE-GEN-2.plants` (final head).
JSON: `handoff/reports/LOCALE-PROBE-GEN-2.json`. Remaining differences: `handoff/LOCALE-PROBE-GEN-2-diff.tsv`.

## Done (do not redo)

* Run 1: probe tools (`tools/probe/locale_dump.py`, `locale_fingerprint.c`), `spec/locale-probe/locales.tsv`,
  `fingerprints.tsv`; LC_NAME/ADDRESS/TELEPHONE of every name from the probe; 68 CLDR-less names from the probe.
* Run 2, decision #7890: every category after LC_COLLATE of every generated name the probe has comes from the probe
  (`probed_rest`, shared by `source` and `probe_source`); no CLDR value kept (none had a stated gain). CLDR remains for
  names the probe lacks (none offered) and LC_IDENTIFICATION's English names.
* Run 2, #7892: `locale_fingerprint --diff en_US.utf8` (via `locale_dump.py --ctype-diff`) lists the exact code
  points where a name's classes / case maps differ from en_US's -> `spec/locale-probe/ctype-diff.tsv` (U+1361 in
  `space`: 16 Ethiopic names; i/I pairs: az_AZ, crh_UA, ku_TR, tr_CY, tr_TR, tt_RU@iqtelif). `ctype_edits` applies
  additions and pairs after `copy "i18n"`; removals or unlisted fingerprint differences become `% gap` comments (none).
* Verified on mei (Skylake-SP, reference locales installed), scripts in mei `~/work/locale-probe-gen-2/`
  (`all.sh`/`one.sh`/`cmp.py` UTF-8; `allL.sh`/`oneL.sh`/`cmpL.py` legacy codesets): all 349 sources compile for
  UTF-8; 39,566 probed values equal, 13 differ (wide thousands separator of 7 `@euro` names); all LC_CTYPE fingerprints
  (12 classes, toupper, tolower, wcwidth) equal for all 349. 182 legacy SUPPORTED entries all compile; 251 values in
  81 entries differ (below). Collation fingerprints differ widely (known: root order, tailorings not done).

## Run 2 jobs (mei)

* Gate `locale-probe-gen-2-gate` (direct tier; FULL is the train's: inherited `spec/cldr/` paths): 7 of 8 steps ok;
  `test -p niji` 46/47: `hot_text_products::init_data_clusters_hold_at_every_text_phase` (fault count cold 0x2000 = 59,
  every other phase 58). LAYOUT-JITTER (#4347): this run changed no libc or start-up code; **origin/next
  (0495c4684) fails identically on mei** (`locale-probe-gen-2-nextht`: cold 0x2000 59, others 58); the same libc code
  passed on keke in LOCALE-PROBE-GEN's gate.
* Plants `locale-probe-gen-2-plants`: 12 of 13 CAUGHT with their message; `resolve-utf8` caught by the products build
  (`no fingerprints for aa_DJ`), msg widened, re-run alone as `locale-probe-gen-2-plants2`.

## Not done / next

1. **localedef transliteration at compile time** (new story, crates/locale): a character the target charmap lacks is
   dropped by `Charset::encode` (cats.rs `mb`); the reference transliterates it (`€` -> `EUR` in ISO-8859-1, U+202F
   -> U+00A0, `’` -> `'`). 251 values / 81 legacy entries (`handoff/LOCALE-PROBE-GEN-2-diff.tsv`); needs translit data
   beyond decompositions (row "Locale data CLDR does not have"). Labelled as a gap in the "Locale sources" row.
2. Collation tailorings for the 12 gap names (with LOCALE-SOURCES-GEN's tailoring step).

## Traps

* `locale -k` prints `era` / `alt_digits` as `"a";"b"` but fixed lists (`abday`) as `"a;b"`.
* The reference's `@euro` names are ISO-8859-15 with compacted `same-as` rows: compare after resolving the sibling
  and transcoding (cmp.py's first version reported all 23 as wholly different).
* The `same-as` plant breaks the products build (`None is not a number`), not the unit test: its `msg:` names both.
