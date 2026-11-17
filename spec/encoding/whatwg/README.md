# WHATWG Encoding Standard indexes

The mapping tables the built-in multi-byte `iconv` codecs are generated from (`cargo xtask
encodings-gen`, `xtask/src/encodings.rs`). The files are copied unchanged from the Encoding
Standard, <https://encoding.spec.whatwg.org/>; each file's header carries its identifier (a
SHA-256 of its contents) and date.

| File | Identifier | Date | Used by |
| --- | --- | --- | --- |
| `index-jis0208.txt` | `cbaa91f3deb7d0841faf5c33041fc15a285da0e87e64ab802c4bf04b7c4da861` | 2024-09-18 | EUC-JP (JIS X 0208) |
| `index-jis0212.txt` | `83bf90dd1c591a4355730d8c4567efc499d74da7490531019ef22a879991cfb7` | 2024-09-18 | EUC-JP (JIS X 0212) |

## Licence

The Encoding Standard and its index files are licensed under the Creative Commons Attribution 4.0
International License (CC BY 4.0, <https://creativecommons.org/licenses/by/4.0/>), as the
standard's own page states. Attribution: "Encoding Standard", WHATWG (Apple, Google, Mozilla,
Microsoft), <https://encoding.spec.whatwg.org/>. The generated tables in
`crates/core/src/iconv/jis_tables.rs` are a transformation of these files (the same pointer to code
point pairs, laid out as arrays) and carry this attribution in their header.

## How the tables are used

The indexes give the character sets; the reference library's behaviour (probed as a black box)
decides each codec's variant: which rows of a set a codec accepts and which cells it maps
differently. Those rules live in the codec (`crates/core/src/iconv/jis.rs`), not in the tables.
