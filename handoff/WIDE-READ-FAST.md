# WIDE-READ-FAST: wide-character reads (D-12.30)

State: branch `task/wide-read-fast`, base origin/next 43987d4dc, code head faa4cd785 (this page and the report after
it). Role mei:~/w/s2 (Skylake-SP). Scratch on the role host: `~/work/WIDE-READ-FAST/` (probe.sh, check.sh, the #23
probe sources and STDIO-WIDE-BYTE-READS-4's head results copied from the Zen 1 host).

## Change

* `sc::getwc_front` (crates/legacy/src/stdio_compat.rs): a stream already wide-oriented on `Codec::Text` (UTF-8,
  ASCII, ISO-8859-1) returns a character given back, else (reading, no partial character, buffer present) decodes the
  whole next character in place from the read window (`text::decode_utf8_initial`), moves `_IO_read_ptr` and
  `_IO_read_end` together (wide window stays closed, D-12.19). Anything else is `None`, nothing touched; the general
  path (`getwc_slow`, the old body) is unchanged. `sc::getwc` = front, then slow (fgetws, `__wunderflow`, wide scanf).
* `stdio::getwc_c` (crates/boundary/src/stdio.rs): the exports' form (fgetwc, getwc, the `_unlocked` forms, getwchar,
  getwchar_unlocked, `__wuflow`): checked `file_words`, sealed record, the lock rule of `with` (unlocked / only thread /
  `__fsetlocking`), then the front path and the `wint_t` straight back; else one out-of-line call to the old path.
  `stdio::getwc` looks the locale up only past the front path.
* Ruling #7212's +3 (fresh-fill note): moot. The note is only on the general path, which a whole character in the
  window never reaches now (2-byte rows 639 -> 119). Deriving it in `end_of_input` from the position is not possible
  as proposed: the empty fill that reports end of input resets `rpos`/`rend` to `RESERVE` first.
* Tests (crates/core/tests/s12_stdio_clauses.rs, clause s12_10_c_0): `wide_read_characters_cut_by_the_window_end`
  (1- to 6-byte buffers, every cut of 2/3/4-byte characters, positions by ftell; stale bytes past a short refill),
  `wide_read_partial_character_is_continued` (EAGAIN mid-character on a non-blocking pipe, then `ungetc` of a byte
  that cannot continue it, and bytes that do). Percall spec `validation/percall/stdio_wide.c` (31 rows).

## Measurements (wl_percall, Skylake-SP, instructions per call: glibc / base 504e620 (= next) / head c7b0887)

| row | glibc | base | head |
|---|---:|---:|---:|
| fgetwc_unlocked ascii / nul / C-locale ascii | 35.6 / 35.6 / 34.5 | 362 / 356 / 347 | 88.1 / 88.1 / 88.1 |
| fgetwc_unlocked 2 / 3 / 4 byte | 58.1 / 84.9 / 98.1 | 639 / 907 / 1172 | 119.2 / 137.4 / 151.6 |
| fgetwc = getwc ascii / nul / C ascii | 67.6 / 67.6 / 66.5 | 372 / 366 / 357 | 94.1 |
| fgetwc = getwc 2 / 3 / 4 byte | 90.2 / 116.9 / 130.1 | 649 / 917 / 1182 | 125.2 / 143.4 / 157.6 |
| getwchar ascii / 2 / 3 byte / C ascii | 71.7 / 95.9 / 122.5 / 69.6 | 373 / 650 / 918 / 358 | 95.1 / 126.2 / 144.4 / 95.1 |
| fgetws 32-char line ascii / 2 / 3 byte / C ascii | 258.2 / 328.8 / 405.3 / 254.8 | 1333.7 / 2213 / 3063.8 / 1284.5 | 684.3 / 805.2 / 862.9 / 684.3 |
| ungetwc + fgetwc (2 byte, C ascii) | 152 | 327 | 252 |
| getc_unlocked / getc / fgetc | 13 / 31 / 31 | 13 / 24 / 24 | 13 / 24 / 24 |

Every wide row 3.2x to 7.7x fewer instructions; byte rows unchanged; no row slower (an intermediate head b68395c had
ungetwc+fgetwc at 343, fixed by the given-back pop in the front path). TARGET NOT REACHED: the wide rows are still
1.2x (fgetwc_4byte) to 2.7x (fgetws ascii) the reference. Zen 1 host not measured (instruction counts; no dispatch
on this path).

Where one call goes (wl_trace, fgetwc_unlocked ascii, head b68395c, 113 instructions: before getwc_c): front path
~51 (mode word, codec and encoding matches, given-back/partial/buffer/role/closed checks, window slice bounds,
decode, 4 stores), file_words 13, with/parts/record ~20, KResult round trip (getwc, branch, wint_or_weof) ~21. getwc_c
removed the last group and most of the second (traced 113 -> about 76 by the percall delta, 125 -> 88).

## Next step (to reach the reference)

1. `fgetws`: still one `sc::getwc` per character (684 per 32-char line, about 21 per character over a fixed part):
   a loop over the window decoding characters until the newline or the room is used, one store at the end.
2. The front path's checks: fold mode/codec/given-back/partial/role/closed into one per-stream "front ready" word
   kept by the general path (set when it leaves the stream wide, reading, nothing pending), so the front path is
   one load + compare before the window bounds.
3. `file_words` (13): the byte path's trusted form (owner #1654, PERF-STDIO-UNLOCKED-TRIM) would apply here too, but
   it changes `fgetwc(NULL)` from `EINVAL` to a fault: RULING NEEDED before using it.

## Jobs (mei)

* Superseded: check1 (static trace red: test names read as clause ids, fixed), gate1 and plants1 (older head, gate
  stopped by me; plants1 3/3 CAUGHT), check2/pc2 (started on an unsynced tree after sync rc 4, stopped), pc1
  (b68395c), trace1 (diagnostic).
* On the final code: check3 (clippy -D warnings all features, static, records, `so`, #23 probe identical: 39 + 23 +
  8 rows, empty diff against STDIO-WIDE-BYTE-READS-4's head), pc3 (table above); check3 and pc3 ran on c7b0887, whose code is faa4cd785's (that commit changed only the D-12.30 record).
* gate2 (faa4cd785, direct tier): 16 suites ok, 9764 passed / 0 failed (1491 ignored), conformance 15 / 0, SKIPPED 0, peak 13G, oom_kill 0.
* plants2 (faa4cd785): 3 plants, 3 CAUGHT, 0 SURVIVED, 0 WARN ().
* hot1:  so +  (stdio is start-up code). Not run:  (FULL tier: the train's, owner 11-19).
