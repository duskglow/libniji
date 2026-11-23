# LOCALE-SOURCES-GEN: locale sources generated from Unicode CLDR (leg 1 done, leg 2 next)

**State.** Branch `task/locale-sources-gen` (from origin/next `1c70414bd`), role keke `~/w/r5` (products built:
`target/niji/localedef`, `locale-charmaps/`). Head: `git log -1`. Decision **D-31.27** (`records/decisions/D-31/D-31.27.toml`).
Coordinator ruling #7767: generate the CLDR-covered names + C (from POSIX XBD 7.3), OMIT the names CLDR lacks (no
territory-only synthesis); list them in a differences row and the hand-back (roadmap: another permissive source).

## Done (leg 1, do not redo)

* Pin: CLDR 49 `cldr-common-49.zip`, SHA-256 `78403d8d...6a89` (CLDR 50/51 directories on unicode.org have no zip yet).
  `tools/cldr/cldr_extract.py` (`--zip F`, `--check`) writes `spec/cldr/49/{locales,territories}.tsv`,
  `allkeys_CLDR.txt`, `LICENSE`. Choice: vendored resolved subset (3.4 MB), never a build-time fetch.
  A local copy of the zip: scratch `dl/cldr-common-49.zip` (untrusted-download dir; scripts live in `work/`).
* `spec/cldr/locale-names.tsv`: 509 SUPPORTED entries / 350 source names (the reference's name list, names only).
  281 generated; 65 not covered: listed in the first comment line of `spec/cldr/49/locales.tsv`.
* `xtask/src/locales.rs` (`cargo xtask locale-sources <dir>`): all 12 categories, `i18n` -> `i18n_ctype` (from
  `crate::unicode::ctype_props`, the C.UTF-8 derivation; classes, combining, toupper/tolower/totitle), tr/az i-mappings,
  `iso14651_t1` (allkeys_CLDR, non-ignorable, symbols per weight), `SUPPORTED`. 7 unit tests (`cargo test -p xtask locales::`).
* Not wired into `cargo xtask so` or the `locales` package yet (see next steps).

## Comparison (keke, black box; job `locale-sources-gen-cmp4`, head before the handoff commit, same code)

Harness (keke `~/work/locale-sources-gen/`, copies in scratch `work/`): `refdump.sh` (reference localedef over the
reference's sources into `ref/`, then `locale -k`), `cmp.sh` (libniji's localedef inside `unshare -r -m` with the
generated tree and libniji's charmaps bind-mounted over /usr/share/i18n, so no reference file is read; `locale -k`
diff), `bcmp.sh` + `lprobe.c` + `psum.py` (strftime of 4 dates x 14 formats, strfmon, printf `'`, every ctype class,
toupper/tolower/totitle and wcwidth over 0..0x10FFFF, strcoll order of a 68-string corpus, strxfrm agreement, rpmatch).

| locale | compiles | ctype classes/maps/width | printf | rpmatch | strftime lines differ | strfmon | strcoll |
|---|---|---|---|---|---|---|---|
| en_US | ok | equal | equal | equal | 16 | equal | differs |
| de_DE | ok | equal | equal | equal | 46 | equal | differs |
| fr_FR | ok | equal | equal | equal | 32 | equal | differs |
| ja_JP | ok | equal | equal | equal | 78 | 12 | differs |
| tr_TR | ok | equal (incl. dotted/dotless i) | equal | equal | 42 | 56 | differs |

`locale -k` keyword differences (27-51 per locale) fall in these kinds:
1. CLDR's date/time conventions: `d_t_fmt` (`%a, %b %d, %Y, %r` vs `%a %d %b %Y %r %Z`), `date_fmt`, ja `d_fmt`
   (`%Y/%m/%d` vs `%Y年%m月%d日`), tr `d_fmt` (`%d.%m.%Y` vs `%d-%m-%Y`).
2. CLDR's names: de `abmon` `Jan.;Feb.;März;...` and `abday` `So.`, fr `avr.`, tr `abday`, ja `abmon` unpadded.
3. am_pm/t_fmt_ampm present in 24-hour locales (CLDR has them; de/fr reference empty: `%r` gives `09:30:59 PM`).
4. Money placement from CLDR's pattern: tr `¤#,##0.00` (symbol first) vs `1.234.567,89 ₺`; ja sign_posn 4.
5. Missing data (no CLDR source): LC_NAME, postal_fmt/country_post/car/isbn, LC_TELEPHONE, ja era/alt_digits.
6. LC_IDENTIFICATION (by design), `ctype-translit-*` (no transliteration tables yet), compiled-layout keys
   (`ctype-width`, `ctype-map-offset`, `collate-symb-hash-sizemb`: localedef's own, D-31.20/21).
Fixed in the generator during leg 1: U+202F/U+00A0 in CLDR time patterns written as the space (reference: space);
`totitle` map added.

## Next steps (leg 2, in order)

1. Collation: (a) Han sorts FIRST today (the `UNDEFINED ..;...` line): give the UCA implicit ranges (CJK unified
   blocks) explicit ellipsis lines after all listed items; (b) variable weighting: CLDR root is non-ignorable, so
   punctuation sorts first (`-a`, `.hidden`, `_x` before digits); the reference ignores it at levels 1-3. Decide
   (UTS #10 "shifted" vs non-ignorable), record the choice in D-31.27 or a new D-31 row, measure with lprobe;
   (c) CLDR tailorings (`common/collation/<lang>.xml` standard rules; extend the extractor) as `reorder-after`; tr first.
2. Transliteration (`translit_combining` from UCD decompositions, CLDR `transforms/Latin-ASCII.xml`), ja era and
   alt_digits (CLDR japanese calendar), the C source (POSIX XBD 7.3).
3. Packaging: `so` hook (`target/niji/locale-sources`), manifest rows + `manifest.py lint` for `locale-sources/`,
   the Unicode notice in `tools/deb/copyright`.
4. CI test in `crates/locale/tests/reference.rs`: generated #14 sources compile; the equal parts stay equal.
5. Differences rows (one per kind above; sorting gets its own row with examples; the 65 omitted names) in
   `docs/differences-from-glibc.md`; compat option: none possible without the reference's data files: say so in the row.
6. Widen the comparison to all 281 names (refdump + cmp + bcmp loops), then gate, plants on `xtask/src/locales.rs`
   (`handoff/LOCALE-SOURCES-GEN.plants`), clippy, static, precheck, report json.

## Open questions / RULING NEEDED candidates

* LC_TELEPHONE, LC_ADDRESS postal formats and LC_NAME: CLDR has none. Candidate permissive sources: ITU-T E.164
  assignments (calling codes, a published list), UN Vienna Convention signs (country_car), Google libphonenumber /
  libaddressinput metadata (Apache-2.0): needs an owner ruling on Apache-2.0 data in `locales`.
* Reserved but unused: SR-89, SR-90, SR-91 (reserved by mistake; saturation ids are not needed).
