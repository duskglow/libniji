# LOCALEDEF-ARCHIVE: locale archive writer, the #7534 gaps, a freeing arena

**State.** Branch `task/localedef-archive` (from origin/next), role keke `~/w/n2`. Leg 2 ended in a REFRESH at
~150k context (93 calls). Workspace clippy (`--all-targets --all-features -D warnings`) is CLEAN on the leg-2 head.
Reserved ids: D-31.23 (arena), D-31.24 (defaults), D-31.25 (archive), D-31.26 (spare: -c / stdin / big-endian).
No COMPAT reserved. Probe scripts and logs: `~/work/localedef-archive/` on keke. Leg-2 helper (local scratch):
`bash ~/w/.scratch/LOCALEDEF-ARCHIVE/sc.sh '<cmd>'` syncs the worktree to keke n2 and runs `<cmd>` there.

## Done in leg 2

* Commit `125135975` (tested): defaults. `undefined_categories_get_the_references_defaults` in
  `crates/locale/tests/reference.rs` PASSES for UTF-8 and ISO-8859-1 (source with only LC_MEASUREMENT, reference
  `localedef -i <path> -c`). Finding: the reference's undefined-category output is NOT the POSIX locale in places:
  LC_CTYPE without print/graph/cntrl/punct for ASCII (widths -1, no outdigits), LC_TIME wide strings and alt month
  names empty, LC_MONETARY "not available" numbers 0xff (a source's `-1` gives 0x7f) and mon_grouping "\x7f",
  LC_ADDRESS country_ab2/ab3 spaces. Decision taken (D-31.24, to write): libniji writes the POSIX values there;
  the test skips exactly those items and checks ASCII classes/widths against XBD 7.3.1 (`posix_ctype`). Matched:
  `week 7;19971130;7`, NUMERIC grouping empty, collate code-point form. Needs a differences row + RULING NEEDED
  (recommend: no compat flag, a degenerate default has no use). Arena clippy fixes done.
* Commit `c6d6bbbe3` (WIP, clippy clean, NOT tested beyond unit tests): `crates/locale/src/archive.rs` (Archive
  parse/names/contains/remove/add/write; 3 unit tests pass), prog.rs body rewritten (archive default, list/add/
  delete/--replace, `--no-archive` name normalized, stdin via `Host::read_stdin`, `<stdin>` in messages, -c and
  outside-section statuses via `compile_text` + `Output::outside`), `Host::lock` (flock on the archive dir in
  `crates/boundary/src/localedef.rs`), prog test mock gained `stdin`/`locked`.

## Done in leg 1 (now compiled, in the two commits above)

* Freeing tool arena in `crates/boundary/src/ldconfig.rs` (D-31.23): power-of-two classes 16 B..64 KiB with
  intrusive free lists from 1 MiB chunks; bigger blocks are own mappings (munmap/mremap). RSS not yet measured.
* `source.rs`: lines outside a category recorded in `Source::errors` and skipped. `lib.rs`: `Output`,
  `compile_output`, `compile_text`; `compile()` stays strict. `defaults.rs`: the built-in defaults source.

## Measured before (base = origin/next 8e1837914, job localedef-archive-so-base, keke)

Peak RSS of `target/niji/localedef -i <loc> -f UTF-8 <dir>`: en_US 224640 KiB, de_DE 344160, fr_FR 344160,
ja_JP 234240, tr_TR 283680 (wall 1.0-1.45 s). Script: `sh ~/work/localedef-archive/rss.sh <localedef> <outdir>`.

## Probe-settled (reference localedef, keke; transcript material for tests)

Archive (`--prefix=P` uses `P/usr/lib/locale/locale-archive`):
* Missing `P/usr/lib/locale`: `cannot create temporary file: P/usr/lib/locale/locale-archive.XXXXXX: No such file or
  directory`, status 1. Compile into archive: silent, status 0 (1 with warnings), replaces an existing name silently.
* Stored names: NAME's codeset normalized (`de_DE.UTF-8` -> `de_DE.utf8`, `xx_XX.ISO-8859-1` -> `xx_XX.iso88591`
  whatever the charmap). NAME without codeset: stored as given AND with the charmap's normalized codeset inserted
  before `@` (`en_US` + ISO-8859-1 -> `en_US`, `en_US.iso88591`; `zz_ZZ@euro` + UTF-8 -> `zz_ZZ@euro`,
  `zz_ZZ.utf8@euro`), both sharing one record. `--no-archive` with `nn_NN.UTF-8` writes directory `nn_NN.utf8`
  (normalized too: libniji today uses NAME as given; fix). Use `niji::locale::name::{parse, archive_form}`.
* `--list-archive`: names sorted bytewise, one per line, status 0; with FILE operand reads that file (extra operands
  ignored); missing FILE: `cannot open locale archive "F": No such file or directory`, status 1; missing default
  archive (prefix): nothing, status 0.
* `--delete-from-archive N...`: exact names only (no normalization); a missing one: stderr `locale "N" not in
  archive`, status still 0. No operands: status 0.
* `--add-to-archive DIR...`: stdout `Adding DIR` per operand; name = DIR's last component normalized; existing name
  without `--replace`: stderr `locale 'fr_FR.utf8' already exists`, status 255; missing DIR: stderr `stat of "DIR"
  failed: No such file or directory: ignored`, status 0. No operands: status 0.
* Format (D.1.6): header 14 u32 (magic, serial 0, namehash off/used/size, string off/used/size, locrectab off/used/size,
  sumhash off/used/size); reference sizes 907 / 0x1d4c / 420 / 2003 (preallocated, file 3-6.5 MB). Name slot index =
  hash % namehash_size; hash = `h = len(name); for each byte c: h = rotl32(h, 9) + c` (black-box fit, 12 of 12
  observed names; textbook rotate-add family; `/tmp`-free script hashfit.py in scratch). Collision step NOT observed:
  writer should pick a prime table size >= 2n+1 where every name's hash % size is distinct (grow until so), so any
  reader finds each name at its first probe. A deleted name keeps its slot with locrec_offset 0: libniji's reader
  (`crates/core/src/locale/archive.rs` `entry`) must treat locrec 0 as absent (today it builds a Record at 0; fix +
  unit test). Identical category images are stored once (locrec slots share offsets).
Gaps:
* No `-i` (and `-i -`): source from stdin; messages name it `<stdin>` (`<stdin>:4: syntax error: ...`).
* Lines outside a category: `<file>:<line>: syntax error: not inside a locale definition section`, skipped, output
  written WITH OR WITHOUT -c, status 1. Error inside a category (`decimal_point "." junk junk`): `<file>:2:
  LC_NUMERIC: syntax error`; without -c status 4 no output; with -c output written, status 1. libniji plan: a failed
  category gets its default (under -c), status 1; without -c the "no output file produced" line, status 4.
* Undefined category: `[warning] No definition for <CAT> category found`, status 1, file written with defaults.
  Reference default values (`locale -k` of p1/stdin-out): tel_int_fmt="+%c %a%t%l", country_ab2="  " (2 spaces),
  country_ab3="   ", category=";UTF-8;;;;;;;;;;;" (identification), height 297 width 210, measurement 1, all
  monetary numbers -1, crncystr "-", mon_decimal_point ".". File sizes: ADDRESS 131, COLLATE 92, IDENTIFICATION 104,
  MEASUREMENT 23, MONETARY 270, NAME 62, NUMERIC 50, PAPER 34, TELEPHONE 48, TIME 1928, SYS_LC_MESSAGES 48.
* `--big-endian`: the reference writes big-endian files, status 0. Plan: a differences row (libniji is x86-64 only;
  no libniji reads them) + COMPAT-NEW or the stated reason; RULING NEEDED in the hand-back.

## Exact next steps (leg 3)

1. Fix the 2 failing prog tests (expected changes): `directory_form_writes_the_defined_categories` (now 12 files:
   `m.files.len()` 14, the defaults are written) and `name_without_slash` (dir is `mini.mini`, archive default now
   works: assert archive written at `/usr/lib/locale/locale-archive` and `--list-archive` lists `mini.mini`).
   Add prog tests: stdin source (`m.stdin = Some(MINI)`, no -i), outside-section line (status 1, written without -c),
   category syntax error (status 4 without -c, 1 with -c), list (missing default: nothing/0; missing FILE: message/1),
   delete (missing name message, status 0), add (`Adding DIR`, exists -> 255, `--replace`, missing DIR -> stat msg).
   Check the exact error texts our `Error.what` gives vs the probed ones (`syntax error: not inside a locale
   definition section`, `LC_NUMERIC: syntax error`); align if cheap.
2. Core reader fix: `crates/core/src/locale/archive.rs` `entry`: locrec offset 0 = absent (+ unit test).
3. End-to-end on keke: `cargo xtask so`, then target/niji/localedef into a `--prefix` archive for the 5 #14 locales
   and setlocale through libniji reading it (find how the existing prog/conformance tests run setlocale over a
   directory; mei `~/work/RELEASE-SNAPSHOT-1122C/y14l/` has the six-POSIX-category case from the coordinator's
   note: it must now give all 12 files and setlocale(LC_ALL) non-NULL). Reference `localedef --list-archive F` on our
   archive is a cheap interop check.
4. Records: D-31.23..26 rows in `plan/07-decisions/s31-utilities.md`, spec §31.8 lines (archive, stdin, -c, defaults),
   `docs/differences-from-glibc.md` rows (defaults' POSIX values; --big-endian refused), `cargo xtask stubs` if the
   spec changes. RULING NEEDED: big-endian, defaults.
5. RSS after: `sh ~/work/localedef-archive/rss.sh target/niji/localedef <outdir>` (before: en_US 224640 KiB, de_DE
   344160, fr_FR 344160, ja_JP 234240, tr_TR 283680).
6. Plants file `handoff/LOCALEDEF-ARCHIVE.plants` (arena free list, defaults fill, archive hash/locrec, stdin, -c
   status, reader locrec 0), `cargo xtask plants` (brief step 6), direct gate (`cargo xtask affected --run`),
   `cargo xtask static`, precheck, report json, handback-check. Wall deadline 2026-11-23T02:58Z.

