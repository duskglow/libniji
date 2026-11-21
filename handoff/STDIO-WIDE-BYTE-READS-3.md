# STDIO-WIDE-BYTE-READS-3 (GitHub #23, the rest): a cut-short sequence keeps its bytes for byte reads

State: branch `task/stdio-wide-byte-reads-3`, base origin/next 305065b07, code head 8bed055f2 (plants file b88644ea5,
this page and the report after it). Role mei:~/w/s3 (Skylake-SP). Continues `handoff/STDIO-WIDE-BYTE-READS.md`
(D-12.27); the finding came from `handoff/RELEASE-SNAPSHOT-1120C.md` finding 1.

## Cause

`getwc_inner` (crates/legacy/src/stdio_compat.rs) decodes one byte at a time from the stream. With the locale's
encoding (`Codec::Text`), an `Invalid` result gave back only the byte that broke the sequence: the leading bytes
of a cut-short sequence (`\xe2\x82` before `t`, `\xc3` before `(`) were already taken and stored nowhere, so the
next `getc` returned the ASCII byte. A lone invalid byte (0xfc, 0x80, 0xff, 0xc0) was the breaking byte itself,
which is why the issue's own input already worked. A `ccs=` stream (`Codec::Charset`) dropped every byte of an
invalid sequence. Not a pushback limit: the reserve holds 8 bytes, a sequence at most 4.

## Change (8bed055f2)

* The `Codec::Text` loop stores each byte of a partial character in `Wide::pending` (unused by that codec; its
  `pending_len` stays 0, so `back_bytes` is unchanged). On EILSEQ, the cold `invalid_sequence` gives back every byte
  of the sequence (both codecs), then sets the error indicator.
* D-12.28 (amends D-12.27; reserved id): at end of input inside a character, the cold `end_of_input` gives the
  character's bytes back too, resets the conversion state and sets the end-of-file indicator again. A byte read
  returns them, `ftell` excludes them, a wide read after more input decodes them again. This matches the reference
  (probe below), so the differences row "Byte operations on a wide-oriented stream" loses its end-of-input clause.
* Hot path: only the store of an incomplete character's byte is added to the wide decode loop; the byte paths
  (getc/putc fronts, `getc_raw`, the engine) are untouched.

## Probe (black box, stock binary, C.UTF-8 and `ccs=UTF-8`, mei; source in the run's scratch, not committed)

Both modes gave the same: `p\x80q`: p, EILSEQ, 0x80, q. `\xc0\xafr`: EILSEQ, c0, EILSEQ, af, r. `\xffs`: EILSEQ,
ff, s. `\xe2\x82t`: EILSEQ (ftell 0), e2, EILSEQ (ftell 1), 82, t; getc x3 gives e2 82 74; getc, ungetc, getc gives e2
e2. `\xc3(u`: EILSEQ, c3, `(`. `\xc3\xc3\xa9z`: EILSEQ, c3, e9, z. `end\xe2\x82`: e n d, WEOF with feof (ftell 3), then
getc e2, 82. A repeated getwc after EILSEQ is EILSEQ again; the stream stays wide.

## Tests

`conformance/c/s12_wide_byte_reads.c` (both libraries; passes against the stock library too): checks 26-27 run
util-linux col's loop (getwc; on EILSEQ getc; at the end the bytes left) over the six invalid lines of y23/gen.sh
and require the output to equal the input, with the locale's encoding and with `ccs=UTF-8`; 28-31 the cut-short
case's first byte, positions and `ungetc`; 33-34 a lead byte before ASCII on a `ccs=` stream with `ungetc`; 35-37 the
cut-short-at-end case (WEOF with feof, position, then its bytes, then EOF).

## Jobs (mei)

All on b88644ea5 (code 8bed055f2), all rc 0. check1 (records check, clippy --all-features -D warnings, static, the
conformance program on both libraries). gate1 (direct tier, `affected --run --base origin/next`: 12 suites,
6706 passed, 0 failed, 15 conformance programs; peak 13G). plants1 (`handoff/STDIO-WIDE-BYTE-READS-3-plants.txt`:
5 CAUGHT, 0 WARN). hottext1 (NIJI_FULL_PROFILE so + hot_text_products, §9). percall1 (`validation/wl_percall.sh
--spec ~/work/STDIO-WIDE-BYTE-READS-3/stdio_char.c .`, STDIO-WIDE-BYTE-READS-2's spec, base 305065b07; instructions:u
per call, Skylake-SP): no row changes base to head: getc/fgetc 24.0 (reference 31.0), getc_unlocked 13.0 (13.0),
fputc/putc/putchar 26.1 (36.0), fputc_unlocked/putc_unlocked 13.1 (13.0), putchar_unlocked 14.1 (14.0). col3: the distribution's col (util-linux, /usr/bin/col on mei) on
y23's three inputs, plain and `-b`, stock vs the role's built libc through its loader: 6/6 cmp-identical, invalid
44 bytes (snapshot 20.3: 38). Superseded: col1 (libniji's loader takes no `--library-path`), col2
(`LD_LIBRARY_PATH` reached the host's `timeout`). y23.sh itself (deb swap into a trixie root) was not run: it needs
this branch's .deb packages; the next snapshot validation covers it.

## Open / next

* Next step: land. The next snapshot's y23 run should show `invalid` identical to stock.
* Not run: `cargo xtask test` (FULL tier, the train's per the owner's 11-19 rule).
