# Unicode mapping tables

The sources of the charmaps `cargo xtask so` generates (`xtask/src/charmaps.rs`, rows in
`../charmaps.tsv`). Each file is copied unchanged from the Unicode Consortium's mapping tables,
<https://www.unicode.org/Public/MAPPINGS/>, under the same relative path (`ISO8859/`,
`VENDORS/MICSFT/WINDOWS/`, `VENDORS/MICSFT/PC/`, `VENDORS/MICSFT/EBCDIC/`, `VENDORS/MISC/`,
`VENDORS/APPLE/`); each file's header carries its version, date and contact. Only the files whose
decoding of every byte equals the reference library's converter of the same names (black-box
probe, SYS-ICONV-COVERAGE) are vendored.

## Licence

Data files published by the Unicode Consortium under <https://www.unicode.org/Public/>, used
under the Unicode terms of use for data files (<https://www.unicode.org/terms_of_use.html>,
Unicode License v3), as each file's header states. The generated charmaps name their source file.

## East Asian multi-byte tables

The built-in codecs BIG5, EUC-KR, CP949, CP932 and CP936 (`cargo xtask encodings-gen`,
`xtask/src/encodings_cjk.rs`, tables in `crates/core/src/iconv/cjk_tables.rs`) are generated from
five more files of the same collection, copied unchanged under the same relative path, plus the
reference library's variant of each as probed (`../reference/`):

| File | Table version | Date | Codec |
| --- | --- | --- | --- |
| `VENDORS/MICSFT/WINDOWS/CP950.TXT` | 2.01 | 1/7/2000 | BIG5 (of the published Big Five tables, the one closest to the reference's BIG5) |
| `OBSOLETE/EASTASIA/KSC/KSC5601.TXT` | 1.1 | 2011 October 14 | EUC-KR (its KS X 1001 part) |
| `VENDORS/MICSFT/WINDOWS/CP949.TXT` | 2.01 | 1/7/2000 | CP949 (Unified Hangul Code, also named UHC) |
| `VENDORS/MICSFT/WINDOWS/CP932.TXT` | 2.01 | 04/15/98 | CP932 |
| `VENDORS/MICSFT/WINDOWS/CP936.TXT` | 2.01 | 1/7/2000 | CP936 |
