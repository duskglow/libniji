# Unicode mapping tables (single-byte)

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
