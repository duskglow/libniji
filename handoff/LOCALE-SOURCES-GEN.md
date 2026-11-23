# LOCALE-SOURCES-GEN: locale sources generated from Unicode CLDR (leg 2 done; LOCALE-SOURCES-GEN-2 re-verified)

**State.** Branch `task/locale-sources-gen-2` (leg 2's `task/locale-sources-gen` 7cf43e7c2 + a merge of origin/next
`0495c4684` = 1121a870c), role shiki `~/w/sh1`. Head: `git log -1`.
Decision **D-31.27** (`records/decisions/D-31/D-31.27.toml`, updated in leg 2). Coordinator ruling #7767: generate
the CLDR-covered names, omit the names CLDR lacks (differences row). Ruling #7791: LC_TELEPHONE / LC_ADDRESS postal /
LC_NAME come from black-box probes (story LOCALE-PROBE-GEN), not from this generator.
Reports: `handoff/reports/LOCALE-SOURCES-GEN.json`, `handoff/reports/LOCALE-SOURCES-GEN-2.json`.

**LOCALE-SOURCES-GEN-2 (this run).** CI train 946 went red on 7 locale tests; settled by the coordinator (#7846) as a
host change on the CI host (the system locale archive was replaced by per-locale directories; LOCALE-PER-DIR-LOAD
fixes the loader), not this branch. This run changed no code: clippy + static clean on the inherited head (shiki
`locale-sources-gen-2-lint0`), merged origin/next (clean, no conflicts), direct gate on 1121a870c GREEN
(`locale-sources-gen-2-gate`: 8 suites, 700 passed / 0 failed, peak 5.1G; includes niji-locale::reference and the
deb manifest / products tests), plants rerun on the merged head (`locale-sources-gen-2-plants`; results in
`handoff/LOCALE-SOURCES-GEN-2-plants.txt`). The 7 train tests were not investigated (coordinator: do not).

## Done (do not redo)

* Leg 1: CLDR 49 pin + extractor (`tools/cldr/cldr_extract.py`, `spec/cldr/49/`), name list, generator
  `xtask/src/locales.rs` (`cargo xtask locale-sources <dir>`), all 12 categories, `i18n`/`i18n_ctype`.
* Leg 2 collation (`iso14651_t1`): UTS #10 "shifted" with UTS #35 maxVariable `symbol` (bound: first currency
  symbol's primary); completely ignorable elements dropped; table lines sorted level by level (L1, L2, L3), then
  ignorable-from-start before shifted, then code point; UTS #10 implicit groups (Tangut, Nushu, Khitan, core Han,
  other Han; `crate::unicode::implicit_groups`) as `..` runs after every listed item; no UNDEFINED line (unassigned
  code points: ignorable but last level, as the reference).
* Transliteration: `translit_combining`, `translit_compat` from UnicodeData decompositions, marks removed; every
  locale's LC_CTYPE includes both.
* Packaging: `cargo xtask so` writes `target/niji/locale-sources` afresh; 287 manifest rows (system `locales`,
  `/usr/share/i18n/locales/<name>`, `SUPPORTED` at `/usr/share/i18n/SUPPORTED`; `manifest.py` accepts `@` in a
  product name); `tools/deb/copyright` Files paragraph + Unicode-3.0 licence text.
* CI test `crates/locale/tests/reference.rs::generated_sources_keep_the_reference_ctype_numbers_and_root_order`.
* 4 differences rows (Utilities section, after `iconv -l`). `spec/cldr/` exempt from lab-names (CLDR word `mei`).
* Plants: `handoff/LOCALE-SOURCES-GEN.plants` (8, xtask unit tests).
* Jobs on the code head bfa45e6bd (keke): `locale-sources-gen-gate2` (direct tier) and `locale-sources-gen-plants2`,
  started 05:50Z: gate2 GREEN (8 suites ok, 690 passed / 0 failed, peak 2.0G); plants2 PENDING at hand-back. Superseded: `locale-sources-gen-gate` (a087da3f1: 689 passed / 1 failed,
  ja_JP maps tojhira/tojkata, test narrowed in bfa45e6bd), `locale-sources-gen-plants` (7 CAUGHT; zero-element
  SURVIVED, now `equivalent:` with its reason). Clippy and `cargo xtask static` clean on a087da3f1 (job
  `locale-sources-gen-lint2`); bfa45e6bd changed only a test, the plants file, docs and handoff.

## Comparison (keke, black box; job `locale-sources-gen-cmp5`, before the level-by-level sort fix)

Harness in keke `~/work/locale-sources-gen/` (`runall.sh [locales]`: regenerate, refdump, cmp, bcmp; `summ.py <log>`).
strcoll of the 68-string corpus now EQUAL in en_US, de_DE, fr_FR, it_IT, pt_BR, ru_RU, nl_NL (was "differs" in
all). Differs (tailorings, not applied): ja_JP, tr_TR, sv_SE, es_ES, pl_PL, zh_CN, ko_KR. Every locale: ctype
classes/maps/widths equal, rpmatch equal; printf equal except sv_SE, ru_RU, pl_PL (2 lines each, not examined);
strftime differs everywhere (CLDR patterns); strfmon differs in ja, tr, sv, ru, pl, nl, zh, ko. The reference's
sv/es/tr strcoll lines in that log look like byte order: NOT VERIFIED (summary quirk or a refdump problem).
CI test (4000-string corpus): equal for en_US/de_DE/fr_FR after excluding Ext-A/supplementary Han, Ŀ/ŀ, and
collseq beyond ASCII (reference table line order: Greek, ŉ, þ before A).

## Not done / next steps

1. C source: generated `C` with a `..` code point order costs 31 MB LC_COLLATE (1.2 GB RSS) in libniji's
   localedef (reference: 1406 bytes); an empty LC_COLLATE is refused by the reference's localedef and libniji's
   88-byte no-rules LC_COLLATE is refused by the reference's loader. Needs a compact code point order in
   localedef (own story); then add `C` back (`c_source()` was written and removed, see D-31.27).
2. CLDR tailorings (`common/collation/<lang>.xml`) as `reorder-after` (extend the extractor; tr, sv, es, pl, zh,
   ja, ko first). ja era/alt_digits, ja_JP classes jspace/jhira/jkata/jkanji/jdigit. CLDR Latin-ASCII translit.
3. Widen the comparison to all 281 names (`runall.sh` with the list).

## Open questions / RULING NEEDED candidates

* LC_TELEPHONE / LC_ADDRESS postal / LC_NAME sources (Apache-2.0 libphonenumber / libaddressinput metadata?).
* localedef accepts an empty LC_COLLATE (the reference refuses) and writes a file the reference's loader refuses.
