# STDIO-WIDE-BYTE-READS (GitHub #23): byte operations on a wide-oriented stream

State: branch `task/stdio-wide-byte-reads` (base origin/next 50abc44ed). Role keke:~/w/n3.
Decision: D-12.27 (`records/decisions/D-12/D-12.27.toml`, amends D-12.14). Differences row
"Byte operations on a wide-oriented stream" replaces the old `fputc` row (no compatibility option, reason in the row).

## What changed

`crates/legacy/src/stdio_compat.rs`: `getc`, `read` (fread), `gets_into` (fgets), `getdelim`, `ungetc` and `putc`
call `byte_access` (orient an undecided stream to bytes; a wide stream passes) instead of failing on a wide-oriented
stream. The wide layer decodes one character at a time from the same byte buffer, so a byte read gets the byte after
the last wide character delivered (after EILSEQ, the invalid byte, which `getwc` leaves unread). `write`
(fputs/fwrite/fprintf), `read_record` and `unget_last` are unchanged. The boundary front paths (`getc_c`,
`read_front_ext`, `getc_lock_c`, `refill`) are not touched: the read/write window stays closed on a wide stream, so a
wide stream reaches the engine as before.

## Probe (black box, C.UTF-8, keke; sources in the run's scratch, not committed)

| case | reference | libniji before | libniji after (expected; the conformance program checks it) |
|---|---|---|---|
| `a\xfccd`: getwc, getc, getwc | a, 0xfc, c | a, EOF (no indicator), WEOF EILSEQ forever | a, 0xfc, c |
| getwc a, getwc (EILSEQ), getc, getwc | a, WEOF/EILSEQ/err, 0xfc, c (err stays) | ..., EOF, WEOF EILSEQ | as reference |
| same with clearerr before getc | 0xfc, c, err clear | EOF, WEOF | as reference |
| after EILSEQ: fread 3 / fgets | `fc 63 64` then wide `\n` / `\xfccd\n` then wide `x` | 0 EINVAL / NULL | as reference |
| after EILSEQ: getc, ungetc, getc, getwc | fc, fc, fc, c | EOF, EINVAL, ... | as reference |
| after EILSEQ: ungetwc Q, getc, getwc, getwc | fc, Q, c | EOF ... | as reference |
| `abcdef`: getwc, getc (wide layer already converted ahead) | EOF, no indicator | EOF, no indicator | `b` (differences row) |
| empty wide stream: getc | EOF, no indicator | EOF, no indicator | EOF with eof indicator (wstream_products KNOWN) |
| incomplete char at EOF, then getc | the char's bytes | EOF | EOF (differences row) |
| putc on wide stream, wide output unflushed | byte lands before the wide output | EOF, EINVAL | byte in program order |
| wide, fflush, putc, wide | `wpx` | `wx`, putc EINVAL | `wpx` |
| fputs / fwrite / fprintf on wide | fail (no errno) | fail EINVAL | unchanged |
| fwide afterwards | stays wide (1) | 1 | 1 |
| getline after EILSEQ (err set) | -1, no errno | -1 | -1 (sticky error, D-12.1) |

## Tests

* `conformance/c/s12_wide_byte_reads.c`: the issue sequence, col's pattern (EILSEQ, getchar, go on), clearerr,
  fread/fgets/ungetc/ungetwc after EILSEQ, a byte written between flushed wide output. Runs on both libraries.
* Updated: `s12_8_c_6` (fputc on a wide stream writes), `read_front` in `s12_stdio_symbols.rs` (byte input reads the
  next byte), `wstream_products.rs` KNOWN (fgetc on an empty wide stream sets EOF; `wstream_diff.c` clears it after).
* Plants: `handoff/STDIO-WIDE-BYTE-READS-plants.txt` (getc EOF again, fread/fgets/ungetc/putc failing again).

## Jobs (keke)

Final head (code 18bbf0107): -check4 green (records check, clippy, static, the new conformance program on both
libraries); -plants 5/5 CAUGHT (same code); -gate2 (direct tier), -counts (getc/putc bench rows), -hottext2
(NIJI_FULL_PROFILE hot_text_products) run in sequence from 10:06Z: PENDING at hand-back.
Superseded: -check/-check2 (test bug: fwrite oriented the tmpfile to bytes), -check3 (record key order), -gate (red:
three niji-boundary assertions of the old rule, fixed in 18bbf0107; stopped), -hottext (ran beside the gate's product
build in the same role, start-up bounds red: not a valid measurement; rerun as -hottext2).

## Open / next

* Counts: base-vs-head bench needs a git checkout plus a base build; head counts only (see report). The front paths
  are unchanged in source.
* INFORMATIONAL: `tools/probe/probe` on keke (`~/bin/probe`) is older than the repo's (no `--niji`); run the repo copy
  `python3 ~/w/<role>/tools/probe/probe` or reinstall (`remote.sh --install`, a tool change).
* Next step: if the gate or plants are red, fix and re-run only those; then `cargo xtask precheck`, report, handback-check.
