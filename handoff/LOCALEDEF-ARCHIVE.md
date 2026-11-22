# LOCALEDEF-ARCHIVE: locale archive writer, the #7534 gaps, a freeing arena

**State.** Branch `task/localedef-archive` (from origin/next), role keke `~/w/n2`. Leg 1 of the run ended in a
REFRESH at ~135k context. The WIP commit is NOT compiled or tested yet. Reserved ids: D-31.23 (arena),
D-31.24 (defaults), D-31.25 (archive), D-31.26 (spare: -c / stdin / big-endian). No COMPAT reserved.
Probe scripts and logs: `~/work/localedef-archive/` on keke (probe1.sh/.log via p1/, probe2.sh, dump.py, rss.sh).

## Done in leg 1 (WIP commit, uncompiled)

* `crates/boundary/src/ldconfig.rs`: `Arena` replaced by a freeing allocator (D-31.23): power-of-two size
  classes 16 B..64 KiB with intrusive free lists, cut from 1 MiB chunks at their own alignment; bigger blocks are
  their own mapping (`crate::raw::munmap` on free, `crate::raw::mremap` on realloc). Check: clippy, the unsafe audit
  (`// UNSAFE(allocator)` comments sit above each `#[allow(unsafe_code)]` / `unsafe {`), then RSS.
* `crates/locale/src/source.rs`: lines outside any category are recorded in `Source::errors` ("syntax error: not
  inside a locale definition section") and skipped (was: fatal).
* `crates/locale/src/lib.rs`: `Output { files, defined, errors }`, `compile_output` (goes past syntax errors, fills
  every undefined or failed category with its default), `compile_text`, private `category()`; `compile()` stays strict
  (first error is Err; undefined categories None) so `tests/reference.rs` keeps its meaning.
* `crates/locale/src/defaults.rs`: `SOURCE`, a built-in locale source of the defaults (POSIX XBD 7.3 POSIX locale
  for LC_CTYPE classes/maps; C values for TIME/NUMERIC/MONETARY/MESSAGES; reference default values for PAPER 297/210,
  NAME name_fmt, ADDRESS postal_fmt, TELEPHONE tel_int_fmt "+%c %a%t%l", MEASUREMENT 1; IDENTIFICATION and
  COLLATE empty). NOT yet compared with the reference; values from `locale -k` of reference defaults (below).

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

## Exact next steps (leg 2)

1. Sync, `cargo clippy -p niji-locale -p niji-boundary` on keke; fix the WIP compile.
2. Test `undefined_categories_get_the_references_defaults` in `crates/locale/tests/reference.rs`: temp source with only
   `LC_MEASUREMENT measurement 2`, reference `localedef -i <path> -c -f UTF-8`, ours `compile_output` over a Files that
   serves the temp path; reuse `diff_items`/`diff_ctype`/`diff_collate`. Iterate `defaults.rs` until equal.
3. `crates/locale/src/archive.rs` (read: bounds-checked, damaged = error; add/replace/delete; write with dedup and the
   hash above), prog.rs: archive default for NAME without `/`, names per the rules above, `--list-archive`,
   `--add-to-archive`, `--delete-from-archive`, `--replace`, `--no-archive` normalization, stdin (`Host::read_stdin`
   in prog.rs + boundary `Disk`), -c handling via `compile_output`, `<stdin>` naming. Archive write via
   `host.write` (boundary `replace_file` is atomic).
4. Core reader fix (locrec 0). Test: setlocale over an archive libniji wrote, every #14 locale (prog test + core).
5. Records: D-31.23..26 rows in `plan/07-decisions/s31-utilities.md`, spec §31.8 Guarantees lines, differences row.
6. RSS after (rss.sh), plants file `handoff/LOCALEDEF-ARCHIVE.plants`, `cargo xtask plants`, direct gate, precheck.
