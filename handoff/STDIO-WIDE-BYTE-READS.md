# STDIO-WIDE-BYTE-READS (GitHub #23): byte operations on a wide-oriented stream

State: branch `task/stdio-wide-byte-reads-2` (continues `task/stdio-wide-byte-reads`; base origin/next 50abc44ed; code
head 18bbf0107, unchanged by the -2 run, which only re-measured the counts and updated this page and the report).
Role keke:~/w/r1 (-2 run); keke:~/w/n3 (first run).
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

Code 18bbf0107 (first run, all rc 0): -check4 (records check, clippy, static, the new conformance program on both
libraries); -plants2 5/5 CAUGHT; -gate3 (direct tier); -hottext3 (NIJI_FULL_PROFILE hot_text_products).
-2 run (same code, role r1): stdio-wide-byte-reads-2-check0 (clippy --all-features -D warnings, static: green),
-2-percall (base vs head vs reference, below), -2-benchrows (`bench --compare --counters cbench_hot cbench_stdio
--rows cbench_hot/getc,cbench_hot/getc_unlocked,cbench_hot/putc_unlocked,cbench_stdio/putc`), and on r1 again
because precheck cannot match jobs from another role to this one's files: -2-plants (5/5 CAUGHT, no WARN),
-2-gate (direct tier), -2-hottext (NIJI_FULL_PROFILE hot_text_products), -2-precheck2.
Superseded: -check/-check2 (test bug: fwrite oriented the tmpfile to bytes), -check3 (record key order), -gate (red:
three niji-boundary assertions of the old rule, fixed in 18bbf0107; stopped), -hottext (ran beside the gate's product
build in the same role, start-up bounds red: not a valid measurement; rerun as -hottext3), -gate2/-counts (a chain
started on a role the sync had not updated; stopped), -counts3 (rc 1: it passed function names to `bench --compare`,
which takes bench names, so nothing was measured; replaced by -2-percall and -2-benchrows).

## Counts (instructions:u per call, deterministic; Zen 1 host)

`validation/wl_percall.sh --base 50abc44ed --spec stdio_char.c` (spec outside the tree, keke
~/work/STDIO-WIDE-BYTE-READS-2/stdio_char.c: reads from /dev/zero, writes to /dev/null, 8192-byte full buffers,
putchar with stdout on /dev/null; every self-check `ok`):

| row | reference | base 50abc44ed | head | head/reference |
|---|---:|---:|---:|---:|
| fgetc, getc | 31.0 | 23.0 | 23.0 | 0.74 |
| getc_unlocked | 13.0 | 13.0 | 13.0 | 1.00 |
| fputc, putc, putchar | 36.0 | 25.1 | 25.1 | 0.70 |
| fputc_unlocked, putc_unlocked | 13.0 | 13.1 | 13.1 | 1.00 |
| putchar_unlocked | 14.0 | 14.1 | 14.1 | 1.00 |

No row changes between base and head (the byte-oriented front paths are untouched). Bench rows (head vs reference,
instr/op): cbench_hot getc 21.4 vs 27.4, getc_unlocked 15.4 vs 15.4, putc_unlocked 17.2 vs 19.1; cbench_stdio putc
23.2 vs 32.2.

## Open / next

* Not run: `cargo xtask test` (the full two-pass suite, i.e. FULL tier; the train's per the owner's 11-19 rule).
* INFORMATIONAL: `tools/probe/probe` on keke (`~/bin/probe`) is older than the repo's (no `--niji`); run the repo copy
  `python3 ~/w/<role>/tools/probe/probe` or reinstall (`remote.sh --install`, a tool change).
* Owed after landing (#6945): the compat-flag registration for the differences row.
* Next step: none on this branch; land.
