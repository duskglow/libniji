# LOCALE-PROBE-GEN report: locale data CLDR lacks, from black-box probes (ruling #7791, D-31.30)

## What was built

* `tools/probe/locale_dump.py`: runs the reference's own `locale charmap` / `locale -ck` under every name of
  `locale -a` (511 on the reference system: libc6 2.39-0ubuntu8.9, locales-all 2.39-0ubuntu8.12) for LC_TELEPHONE,
  LC_ADDRESS, LC_NAME, LC_PAPER, LC_MEASUREMENT, LC_IDENTIFICATION, LC_NUMERIC, LC_MONETARY, LC_TIME, LC_MESSAGES.
  Output `spec/locale-probe/locales.tsv` (46,087 rows, 2.4 MB): `name, category, keyword, value`, value = the printed
  bytes, everything outside printable ASCII and `\` as `\xHH`; `charmap` row per name; a non-UTF-8 name keeps only
  rows whose decoded text differs from its UTF-8 sibling (`same-as` row; 180 names compacted). `time-era-entries`
  is not dumped (not a text item; `era` carries the entries). `--names` + `LOCPATH` reads other compiled locales
  with the same program (used for the comparisons below). Reproducible: re-running gives the same bytes.
* nl_langinfo probes: none needed. Every item of these categories that has a public `nl_langinfo` meaning is
  printed by `locale -k` (the wide-character copies of the strings are the same text).
* `tools/probe/locale_fingerprint.c` (`locale_dump.py --fingerprints`): per UTF-8 name, FNV-1a hashes of the 12
  POSIX classes, towupper, towlower, wcwidth over all code points and of the wcscoll order of 13 script-group
  corpora -> `spec/locale-probe/fingerprints.tsv` (327 names). Re-run gave identical output.
* Generator (`xtask/src/locale_probe.rs`, `xtask/src/locales.rs`): LC_NAME, LC_ADDRESS, LC_TELEPHONE of every
  locale from the probe; the 65 CLDR-less names plus `eo`, `syr`, `tok` generated from the probe in all
  categories; LC_CTYPE / LC_COLLATE by fingerprint match (below). 349 of 350 source names now generated (508 of
  509 SUPPORTED entries; `C` waits for D-31.27's compact order). 68 manifest rows added.

## Coverage and provenance

| category | from the probe | from CLDR | viewed under the exception |
|---|---|---|---|
| LC_NAME, LC_ADDRESS, LC_TELEPHONE | 349 names (7,678 keyword values) | 0 | 0 |
| LC_NUMERIC, LC_MONETARY, LC_TIME, LC_MESSAGES, LC_PAPER, LC_MEASUREMENT | 68 names | 281 names | 0 |
| LC_IDENTIFICATION | 68 (language, territory) | 281 | 0 |
| LC_CTYPE / LC_COLLATE (68 probe names) | by behaviour match: 57 / 56 copied, 11 / 12 gaps | - | 0 |

**Source looks under the narrow exception: none.** No reference locale source file was opened.

## Verification (keke, Zen 1 host)

All 349 generated sources compiled with libniji's `localedef` (0 failures), then read back by the reference's
`locale -k` through `LOCPATH`, compared with the probe dump:
* LC_NAME / LC_ADDRESS / LC_TELEPHONE: **7,678 values equal, 0 differ** (all 349 names).
* The 68 probe-generated names: every probed keyword of every category equal (6,005 values) except
  LC_IDENTIFICATION (by design: libniji names itself, 365 values).
* Compiled size: median 8.75 MB per locale (LC_COLLATE 8.40 MB of it, the shared root table); libniji localedef
  median 5.5 s per locale (min 3.4, max 8.3; 12 at once), peak RSS 223 MB.
* CI test `crates/locale/tests/reference.rs::generated_sources_carry_the_probed_name_address_and_telephone`
  (15 locales: Latin, Cyrillic, Arabic, Devanagari, Han, Kana, Ethiopic, syllabics, Turkic; CLDR and probe names).

## The 68 probe-generated names (`handoff/LOCALE-PROBE-GEN-names.tsv`)

* LC_CTYPE: 56 equal en_US's (copy `i18n`), crh_UA equals tr_TR's (dotted/dotless i: copied). **Gap, 11 names**
  (aa_ER, aa_ET, byn_ER, gez_ER, gez_ER@abegede, gez_ET, gez_ET@abegede, sid_ET, ssy_ER, tig_ER, wal_ET): the
  reference puts U+1361 ETHIOPIC WORDSPACE in class `space` (the only differing code point, probed over all
  0..0x10FFFF); en_US's is used and the gap is in the source comment and the differences row.
* LC_COLLATE: 56 equal en_US's root order (copy `iso14651_t1`). **Gap, 12 names** (reference tailors): an_ES,
  ber_DZ, csb_PL, eo, ik_CA, tl_PH (latin), crh_UA (ascii, latin), gez_ER@abegede, gez_ET@abegede (ethiopic),
  cmn_TW (Han), iu_CA, shs_CA (ascii, latin, greek, cyrillic, armenian/hebrew, ethiopic). Root order used.

## Cross-check of the probe (`handoff/LOCALE-PROBE-GEN-xcheck.txt`)

Against CLDR 49 (ISO 3166 alpha-3 / numeric, territory and language names, ISO 639 codes); the reference's value
is generated (parity). country_ab2 1/346 disagree (as_IN prints two spaces), country_ab3 3/346 (as_IN, my_MM,
shn_MM: spaces), country_num 0/346, country_name 32/280 (spelling or script choices), lang_name 23/281,
lang_term 8/254 (eu_FR prints `fra`; bho `bho` vs CLDR `bih`; fil `fil` vs `tgl`), lang_ab 3/281 (eu_FR `fr`,
sd_PK empty), lang_lib 2/51. Dialling codes: CLDR 49 has no telephone code data any more and no ITU E.164 copy
was at hand, so only an internal check: every territory has one `int_prefix` across its locales, none empty.

## CLDR-generated vs reference (step 4; `handoff/LOCALE-PROBE-GEN-cldr-diff.tsv`, 5,278 rows; not changed here)

Per category, values that differ over the 281 CLDR names (equal: 18,294): LC_MONETARY 2,325, LC_TIME 2,224,
LC_MESSAGES 407, LC_NUMERIC 320, LC_PAPER 2, LC_MEASUREMENT 0. Largest keyword classes: d_t_fmt / date_fmt 274,
t_fmt_ampm 246, am_pm 228, d_fmt 214, abmon 180, abday 172, ab_alt_mon 163, yesexpr 158, t_fmt 137, noexpr 130,
int_[pn]_sep_by_space 110-115, thousands_sep 108, day 105, mon_thousands_sep 101, crncystr 91, mon / alt_mon 82,
[pn]_cs_precedes 64-69, nostr 62, sign_posn 26-61, yesstr 57, currency_symbol 54, negative_sign 45,
mon_decimal_point 44, decimal_point 36, frac_digits 34, grouping 30, mon_grouping 22, week-1stweek 15,
first_weekday 13, cal_direction 9, int_curr_symbol 8, alt_digits 6, first_workday 5, era 4, LC_PAPER 1 locale.
The coordinator decides per class (owner: no gratuitous incompatibility).

## INFORMATIONAL

* CLDR-generated Ethiopic locales (am_ET, om_ET, so_ET, ti_ER, ti_ET) share the U+1361 `space` gap; ku_TR and
  tt_RU@iqtelif case maps differ from en_US's in the reference (Turkic i) but are generated without the tr lines.
  The Ethiopic gap is now in the differences row (Locale data CLDR does not have); ku_TR / tt_RU@iqtelif are not
  in a row yet (found by the fingerprints; outside this story's generator). A fix for both is a small LC_CTYPE
  addition (next story).

## Leftovers / next

* Close the LC_CTYPE gaps (`space` addition for Ethiopic, Turkic i for crh-like names) and the 12 collation
  tailorings (with LOCALE-SOURCES-GEN's tailoring step).
* Step-4 table: coordinator's per-class decision.
