# FCLOSE-COOKIE-SEEK: fclose of a reading cookie stream never fails because the hook cannot seek (GitHub #20)

State: branch `task/fclose-cookie-seek` (from origin/next 5e1903cce); the HEAD is in the report JSON.
Report: `handoff/reports/FCLOSE-COOKIE-SEEK.json`. Plants: `handoff/FCLOSE-COOKIE-SEEK-plants.txt`.

## What changed

- `crates/legacy/src/stdio_compat.rs` `close`: a cookie stream that is reading drops its read-ahead at
  `fclose` and calls no seek hook (any mode, `r` or `r+`). Descriptor streams still give the read-ahead
  back (POSIX `fclose`), memory streams are unchanged.
- Same file, the cookie `Backing::seek`: a failing seek hook reports the hook's own `errno` (`EIO` when it
  leaves `errno` 0), as the read and write hooks already do through `hook_result`; it used to be `EINVAL`.
  So `fflush` tolerates `ESPIPE` from a hook (0), and `ftell`/`fseek` report `ESPIPE` or `EINVAL` as the hook said.
- Tests (`crates/core/tests/stdio_short_io.rs`, new seek hook in the harness):
  `cookie_close_after_read_calls_no_seek_hook` (the issue's reproducer and its rows),
  `cookie_flush_of_read_ahead_reports_the_seek_hooks_errno`, `descriptor_close_after_partial_read_gives_read_ahead_back`.

## Probe table (keke, glibc 2.39 dynamic vs libniji static; probe source in the hand-back scratch, runner below)

Cookie stream over 100 bytes, `fread` 10, then the call. "seeks" = seek-hook calls made by the call.

| row | reference | libniji before | libniji after |
|---|---|---|---|
| fclose, hook works / ESPIPE / EINVAL (partial read) | 0, no seek | 0 / -1 EINVAL / -1 EINVAL, 1 seek | 0, no seek |
| fclose after reading to the end (any hook) | 0, no seek | -1 EINVAL (failing hooks), seek(0, CUR) | 0, no seek |
| fclose, `r+` after a read (any hook) | 0, no seek | -1 EINVAL (failing hooks), 1 seek | 0, no seek |
| fclose after `fflush(NULL)` / after a failed `ftell` | 0, no seek | -1 EINVAL, 1 seek | 0, no seek |
| fflush, hook ESPIPE | 0 (errno ESPIPE), ferror 0, read-ahead kept | -1 EINVAL, ferror 1 | 0, ferror 0, kept |
| fflush, hook EINVAL | -1 EINVAL, ferror **0**, kept | -1 EINVAL, ferror 1, kept | -1 EINVAL, ferror 1, kept (s12_4_f_0; RULING) |
| ftell / fseek, hook ESPIPE | -1 ESPIPE | -1 EINVAL | -1 ESPIPE |
| fflush / ftell / fseek, no seek hook | -1 errno unchanged / -1 EIO / -1 errno unchanged | 0 / -1 ESPIPE / -1 ESPIPE | unchanged (RULING) |
| fseek(5, SET), hook works: hook calls | seek(0,SET) then a 100-byte read | seek(5,SET) | unchanged (INFORMATIONAL) |
| pipe: fclose / fflush / ftell after a partial read | 0 / 0 (errno ESPIPE) / -1 ESPIPE | 0 / 0 (errno 0) / -1 ESPIPE | unchanged |
| regular file `r`/`r+`, dup'ed fd: offset after fclose (partial read) | **100** (end of read-ahead) | 10 (POSIX `fclose`) | 10, unchanged (RULING) |
| regular file: offset after fflush / after read to the end | 10 / 100 | 10 / 100 | unchanged |

The "after" column is from the tests on the final head (the gate's tests); a second probe run on the
final products is in the hand-back if it ran.

## Open questions (RULING NEEDED, in the hand-back)

1. The brief assumed the reference gives a regular file's read-ahead back at `fclose`; the probe shows it
   does not (offset 100 after reading 10 of 100). libniji keeps POSIX `fclose` (offset 10), as the brief
   asked. No differences row records this yet: recommend a row with no compat option (correctness, and
   gnulib's fflush/fclose checks depend on it; row "fflush on a seekable input stream").
2. A give-back failing with an errno other than ESPIPE: the reference returns EOF but leaves `ferror` clear;
   libniji sets it, as s12_4_f_0 and POSIX `fflush` say. Recommend keeping libniji's behaviour and adding a
   differences row, or amend the clause if the owner prefers the reference.
3. A cookie stream with no seek hook: the reference's `fflush` of a reading stream returns -1 with `errno`
   unchanged, `ftell` -1 EIO, `fseek` -1 with `errno` unchanged; libniji returns 0, -1 ESPIPE, -1 ESPIPE.
   Recommend keeping libniji's (a meaningful errno, the same as a pipe) with a differences row.

## Not to redo

- `~/bin/probe` on the build hosts looks for `target/liella/liella-crt1.o`; products are `target/niji/niji-crt1.o`,
  so it rebuilds forever and never runs (two 15-minute losses here). Use a direct compile:
  `gcc -std=gnu17 -O1 -fno-builtin -nostdinc -isystem T/include -isystem $(gcc -print-file-name=include) -isystem T/uapi -static -nostdlib T/niji-crt1.o probe.c T/libc.a -lgcc` with `T=~/w/<role>/target/niji`.

## Next step

Land after the gate and plants named in the hand-back; then the three rulings above.
