# UTIL-12: `niji-locale` and `localedef` (issue #14's blocker)

**State.** Branch `task/util-12` (from origin/next `90390071c`), role mei `~/w/s1`. Head: see `git log -1`.
Run 1 ended after leg 4 (all refreshes used). The directory form of `localedef` is done and shipped in
libc-bin; the locale archive is not.

## Done (do not redo)

* `crates/locale` (niji-locale): source parser, all 12 Appendix D.1 category writers, and the command
  (`prog.rs`: options, search paths, `.gz` charmaps inflated in-process, the default-charmap fallback,
  directory form for a NAME with `/` or `--no-archive` (+ `--prefix`), the reference's messages/statuses).
* Shipped program: `crates/localedef-bin` over boundary feature `localedef` (`crates/boundary/src/localedef.rs`),
  entry `niji_machine_localedef_entry` (`csys/src/machine/entry.S`, `-DNIJI_LOCALEDEF`), built by
  `run_localedef` in `xtask/src/so.rs` into `target/niji/localedef` (static PIE, no interpreter, D-31.2 checks).
  Manifest row `system libc-bin usr/bin/localedef`. Transcript harness knows `localedef`.
* Decisions D-31.19 (crate + freestanding program), D-31.20 (collation weights: order equal, bytes own),
  D-31.21 (shared transliteration lists).
* Tests: `crates/locale/tests/reference.rs` (6 locales vs the reference, by the reader), `tests/prog.rs`
  (7 command tests), `validation/transcripts/localedef/` (10 transcripts, recorded on the Zen 1 host).
* Plants: `handoff/UTIL-12.plants` (7: prog.rs x4, collate.rs x2, ctype.rs x1).

## Probe-settled (reference localedef, Zen 1 host)

* Missing source: one `[error] cannot open locale definition file ...` line, status 4. Missing charmap: its
  line, then the default `ANSI_X3.4-1968` is used; without `-c` the no-output line and status 4, with `-c`
  output and status 0. The output directory is made before any input is read (left empty on failure); a
  parent that is missing is reported at write time (`cannot write output files to ...`).
* Categories the source lacks: `[warning] No definition for <cat> category found`, status 1, but the
  reference still writes all 12 files (POSIX defaults). libniji writes only the defined ones (gap below).
* The reference reads the charmap before the source; `-f path.gz` is not inflated (only the search is).
* Measured: libniji's localedef compiles each of en_US/de_DE/fr_FR/ja_JP/tr_TR.UTF-8 in 0.6 to 0.84 s
  (wall, one run, Zen 1 host), peak RSS 218 to 338 MB (the ldconfig arena never frees non-last blocks);
  10 of 12 files byte-equal to the reference's, LC_COLLATE and LC_CTYPE by design (D-31.20/21); libniji's
  `locale -k` over our de_DE equals the same over the reference's.

## Open (next story: UTIL-12b recommended)

1. The locale archive writer and `--list-archive` / `--add-to-archive` / `--delete-from-archive` (D.1.6;
   `niji::locale::archive` reads). Today a NAME without `/` and without `--no-archive` is status 4.
2. POSIX default files for categories a source does not define (the reference writes them).
3. No `-i`: the reference reads the source from stdin; libniji reports an error.
4. A syntax error stops the compile (the reference continues under `-c`); `--big-endian` is refused.
5. Peak RSS: a freeing allocator (or arena reuse) before UTIL-13 runs compiles in parallel.
6. RULING NEEDED (R5): ship Devuan's locale sources and charmaps in `locales` (they come from glibc's
   source package; not in this tree). Recommendation: package them from the distribution's source.

## Rows to read

D-31.2, D-31.4, D-31.19..21; plan/utilities.md §4 "Locale" and the UTIL-12 row; spec §31.8.
