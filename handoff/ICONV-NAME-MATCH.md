# ICONV-NAME-MATCH (issue #6: `iso88591`): iconv_open name rule — run 1 (checkpoint)

**State.** Branch `task/iconv-name-match`, code head `1bbbafb1d` (handoff commits after it). Role keke:~/w/r2.
Decision: `records/pending/ICONV-NAME-MATCH-1.toml` (D-NEW, series D-6, amends D-6.8). Rule and probe
table: `spec/encoding/iconv-names.md`. Plants: `handoff/ICONV-NAME-MATCH-plants.txt`.

## What was found (black-box probes, reference glibc 2.39, 32,915 spellings of the 1,180 `iconv -l` names)

The issue's "drops punctuation and ignores case" is not the rule. `iso88591` opens because `ISO88591` is a
listed name of its own. The rule, written as libniji's in the spec file:
* Only letters, digits and `, - . / : _` count. Every other byte is dropped (space, controls, other ASCII
  punctuation, 0x7F and up; settled by a 255-byte sweep in three positions). Case is ignored.
* The name is `PART[/PART2]`; suffixes start after the second `/`. Trailing commas of the second part drop.
  If the second part is then empty, the name is the first part without its trailing commas.
* The result must equal a known name exactly: `cp-1252`, `-utf8` and `utf8.` name nothing.
* Collisions: only `NF_Z_62-010_(1973)` and `NF_Z_62-010_1973` reduce alike, and they denote one conversion.
* `iconv -l` listing: unchanged by the rule. libniji's built-in list gains `ISO88591`, as the reference lists it.

## What changed

* `crates/core/src/iconv/mod.rs`: `parse_name` returns the matching form (`Spec::name() -> Option<&[u8]>`;
  `None` when over `NAME_MAX` = 128). `builtin` compares exactly. New `names_charset`. `ISO88591` added to
  the Latin-1 names.
* `global_locale.rs`: `load_charmap`/`find_charmap` take the name predicate. Locales keep `same_codeset`;
  `iconv_open` uses `names_charset`.
* Callers: `boundary/src/iconv.rs` (`None` → `EINVAL`, empty → locale codeset), `legacy/src/intl.rs`, the fuzz target.
* Tests: `s06_multibyte_clauses.rs::s6_4_c_1_name_spellings` (core, fast) and
  `iconv_charmaps_products.rs::iconv_open_matches_name_spellings_like_the_reference`. The products test checks
  two things for 38 spellings across the probe classes. First, libniji and the reference both open or both
  reject each spelling. Second, an opening spelling must denote its listed name's conversion (fingerprint).
  It fails on next: next rejects `iso88591` and ` utf-8`, and accepts `cp-1252`.

## Evidence so far

* Before (next, products built on keke): 5328 of 9719 spellings open or fail as on the reference.
* `iconv_open` instructions before (1000 × open+close to and from UTF-8, static probe, `perf stat instructions:u`):
  UTF-8 1,697,344; utf-8 1,697,344; CP1252 3,208,789; ISO-8859-15 3,013,608. After: in
  `~/logs/iconv-name-match-counts.log` (`measure.sh after`; full table in `~/work/ICONV-NAME-MATCH/niji-after.tsv`).
* Jobs started on the final head, not yet read: `iconv-name-match-counts` (clippy, static, workspace clippy,
  clause test, `xtask so`, after-counts), `iconv-name-match-gate` (direct tier, starts after counts),
  `iconv-name-match-plants` (9 plants).

## Next step

1. Read the three jobs. Compare the after-counts with the before row above; the target is equal or fewer
   instructions (the matching form adds a short pass over the name; an exact compare replaces the
   case-insensitive one).
2. Run `python3 -I cmp.py niji-after.tsv` (scratch `cmp.py`, `names.tsv`, `ref.tsv`) and put the after
   figure in the spec table. Expected: every class agrees, apart from the ones the generated charmaps do not cover.
3. Fix any red, write the after numbers here and in the report, then run handback-check.

## Informational

* Suffix spelling (outside this rule): the reference accepts `//translit ` with a trailing space as TRANSLIT,
  but not `// TRANSLIT` or `//TRANS LIT`. libniji matches none of the three. Possible follow-up story.
* Do not redo: the probe set (`gen.py`, `probe.c`, `ref.tsv`) is in the scratch dir.
