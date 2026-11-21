# SYS-ICONV-CHARMAPS: iconv converters with our packages alone (issue #6), run 1

**State.** Branch `task/sys-iconv-charmaps`; see the report json for the head. Verdict: EUC-JP done and verified, so
issue #6's case works; the story's target (every converter the reference's libc6 has) is **not reached**: 58 of 1180
names open with our packages alone (53 before).

## Inventory (black box, reference system glibc 2.39; full list `handoff/SYS-ICONV-CHARMAPS-inventory.tsv`)

* Reference: every one of the 1180 `iconv -l` names opens both ways against UTF-8; 1116 round-trip "Az09" (the
  rest are not ASCII-compatible: EBCDIC, UTF-7, ...). Its converters come from its gconv modules, not from charmaps,
  so a root without `locales` is the same.
* libniji before this branch: 53 names with our packages alone (charmaps directory hidden by a bind mount in a user
  namespace), 168 with the host's charmaps visible. Now: 58 (+ the five EUC-JP names).

## Design (decision record `records/pending/SYS-ICONV-CHARMAPS-1.toml`, D-NEW amending D-6.8)

Option (a) of the brief: built-in tables generated from public mapping specifications.
* Data: WHATWG Encoding Standard indexes, vendored unchanged in `spec/encoding/whatwg/` (CC BY 4.0, identifiers
  and attribution in its README.md). Generator `cargo xtask encodings-gen` (`xtask/src/encodings.rs`) writes
  `crates/core/src/iconv/jis_tables.rs` (`@generated`); `cargo xtask static` checks it is current.
* Tables hold the indexes as published; the codec (`crates/core/src/iconv/jis.rs`) holds the reference's variant.
  New `Codec::EucJp` (descriptor tag 15). Built-in names are found before any charmap file.
* Sizes: JIS X 0208 decode 17,672 B, encode 2 x 14,672 B; JIS X 0212 decode 17,672 B, encode 2 x 12,134 B;
  total 88,956 B of `.rodata`, no heap, touched only when an EUC-JP descriptor converts. No start-up code changed.

## Probe-settled behaviour (EUC-JP; all in the codec's doc comment and unit tests)

Rows 13 and 89..92 of the index absent; six cells remapped (A1C1 U+301C, A1C2 U+2016, A1DD U+2212, A1F1 U+00A2,
A1F2 U+00A3, A2CC U+00AC); 0x80..0x8D, 0x90..0x9F are C1 controls; U+00A5 -> 5C and U+203E -> 7E one way; tags
U+E0000..E007F encode to nothing; a code point in both sets is written in JIS X 0208. Invalid input: lead
0xA0..0xFE or 0x8E plus a byte that ends nothing; `//IGNORE` skips the lead alone if that byte is below 0xA0, else
both; after 0x8F only 0xA2..0xED wait for a third byte, any invalid 0x8F sequence skips the 0x8F alone; 0xFF invalid.
Aliases probed identical: EUC-JP, EUCJP, UJIS, CSEUCPKDFMTJAPANESE, OSF00030010 (EUC-JP-MS and EUCJP-WIN/-OPEN are a
different converter; EUC-JISX0213 another).

## Tests and results (mei, role s2)

* `crates/core/tests/iconv_codespace_products.rs` + `tests/oracle/iconv_codespace.c`: every byte sequence, every
  code point, `//IGNORE` on every invalid sequence, `//TRANSLIT` and `//IGNORE` on every code point, host vs libniji.
  Equal except 32 `//TRANSLIT` blocks, which come from the built-in C locale's UCD-derived transliteration (D-6.9:
  U+00AD, U+00B2, U+00B5, U+2014 ...), not from the codec; they are listed in the test.
* Unit tests `iconv::jis::tests` (3). Jobs: sys-iconv-charmaps-gate2 (clippy, static, unit, both iconv product
  tests, reproducer) rc 0; issue #6's reproducer prints `EUC-JP: ok` with the charmaps directory empty.
  sys-iconv-charmaps-affected (direct tier) and sys-iconv-charmaps-plants (9 plants,
  `handoff/SYS-ICONV-CHARMAPS-plants.txt`) were started at hand-back.

## Not done (proof items of the brief still open)

* Every other converter (about 240 more). Packaging proofs: both package flavours, `check-system`, swapped root
  52/52, gettext's `AM_ICONV` configure run. Start-up counters not measured (no start-up code touched; the tables
  are unreferenced until an EUC-JP descriptor converts).
* `gh` is not installed on the coordinator machine: issue #6 was not read; the brief's summary was used.

## Next step

Add codecs on the same pattern, each with the reference's code-space dump first (`iconv_codespace.c` names the
charset on its command line; add it to `CHARSETS` in the test): Shift_JIS and CP932 (index-jis0208 incl. its
pointers >= 8836, which the generator now drops) and ISO-2022-JP (stateful; needs descriptor state), EUC-KR/CP949
(index-euc-kr), GBK/GB18030 (index-gb18030 + ranges), Big5/Big5-HKSCS (index-big5), then the single-byte code pages
(WHATWG `index-*.txt` single-byte files and Unicode's mapping files for the IBM/EBCDIC pages). EUC-TW has no WHATWG
index: CNS 11643 needs a source and a licence check (RULING if none is public).
