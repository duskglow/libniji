# The reference library's variants of the East Asian multi-byte charsets

One file per built-in East Asian codec (`BIG5`, `EUC-KR`, `CP949`, `CP932`, `CP936`, `GB18030`): what a
black-box probe of the reference library's converter found beyond the public mapping table the
codec is generated from (`../unicode/`, `../whatwg/`). The probe is
`crates/core/tests/oracle/iconv_codespace.c` run against the reference C library (every byte
sequence decoded, every code point encoded); `tools/probe/iconv_variant.py` compares its output
with the public table and writes the file. `cargo xtask encodings-gen` (`xtask/src/encodings_cjk.rs`)
builds each codec's tables from the public table plus this file, so every difference is data
here, none is code. These files are probe results, not C-library data.

## Format

Tab-separated records, one per line; `#` starts a comment line. Bytes and code points are hex,
GB18030 four-byte pointers decimal. A byte list is comma-separated bytes or ranges `A-B`, or `-`
for none.

| Record | Fields | Meaning |
| --- | --- | --- |
| `leads` | byte list | the lead bytes of two-byte sequences |
| `trails` | byte list | the trail bytes a lead accepts |
| `single` | first, last, code point | the bytes first..last (0x80..0xFF) decode alone, to consecutive code points |
| `cells` | first, last, code point or `none` | the two-byte cells first..last, in lead and trail order, map to consecutive code points from the one given (`none`: no character), replacing the public table's |
| `four` | pointer, code point | GB18030: the four-byte pointer decodes to this code point, not the one `index-gb18030-ranges.txt` gives; the code point is also written as that pointer unless another form is recorded |
| `encode` | code point, bytes or `none` | the code point is written as these bytes (`none`: it has no encoding), whatever the decoder reads them as |
| `skip1` | leads, trails | with `//IGNORE`, an invalid sequence of one of these leads and one of these trail bytes is skipped as the lead alone (otherwise as both bytes) |
| `ignore-result` | `error` or `success` | how a call that `//IGNORE` let skip invalid input ends: `-1` with `EILSEQ`, or success |

Without an `encode` record a code point is written as its shortest, then lowest, sequence.
