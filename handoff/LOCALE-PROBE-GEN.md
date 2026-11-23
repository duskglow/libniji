# LOCALE-PROBE-GEN: locale data CLDR lacks, generated from black-box probes (ruling #7791)

**State.** Branch `task/locale-probe-gen` (stacked on `task/locale-sources-gen-2`), role keke `~/w/r5`. Head: `git log -1`.
Decision **D-31.30** (`records/decisions/D-31/D-31.30.toml`; extends D-31.27). Full report:
`handoff/LOCALE-PROBE-GEN-report.md`; tables `handoff/LOCALE-PROBE-GEN-names.tsv` (68 probe-generated names:
LC_CTYPE / LC_COLLATE copied or gap), `-xcheck.txt` (probe vs CLDR codes and names), `-cldr-diff.tsv` (step 4:
every CLDR-generated value that differs from the reference; not changed here). Plants: `handoff/LOCALE-PROBE-GEN.plants`.
JSON: `handoff/reports/LOCALE-PROBE-GEN.json`.

## Done (do not redo)

* Probe tools `tools/probe/locale_dump.py` (dump, `--fingerprints`, `--names` + `LOCPATH` read-back) and
  `tools/probe/locale_fingerprint.c`; data `spec/locale-probe/locales.tsv`, `fingerprints.tsv` (taken on the host
  with the reference's precompiled locales; regenerate there, see the script's docstring).
* Generator: LC_NAME / LC_ADDRESS / LC_TELEPHONE of all 349 generated names from the probe; 68 names (65 CLDR-less +
  eo, syr, tok) generated from the probe in every category; manifest rows; differences rows updated (collation,
  CLDR-lacks, names offered); labnames allow-list for `spec/locale-probe/`.
* Verified at full scale (compile all 349 with libniji localedef, read back with the reference's `locale -k`):
  LC_NAME/ADDRESS/TELEPHONE 0 differences; probe names differ only in LC_IDENTIFICATION (by design).
* No reference locale source was viewed under the ruling's exception.

## Not done / next

1. LC_CTYPE gaps: U+1361 in `space` for the Ethiopic names (also CLDR-generated am_ET, om_ET, so_ET, ti_ER, ti_ET);
   Turkic case maps for ku_TR, tt_RU@iqtelif (no differences row yet).
2. Collation tailorings for the 12 gap names (with LOCALE-SOURCES-GEN's tailoring step).
3. Step-4 table: coordinator decides per class.

## Not to redo / traps

* `locale -k` prints `era` / `alt_digits` as `"a";"b"` but fixed lists (`abday`) as `"a;b"`.
* Parallel compare output must go to one file per locale (lines over the pipe buffer interleave).
