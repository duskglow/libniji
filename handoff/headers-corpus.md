# SL-HEADERS-1: header and reserved-name corpus scan

Branch `task/sl-headers-1`. Build host: keke (role n1). Report json: `handoff/reports/SL-HEADERS-1.json`.

## State

Done: the tool (`validation/headers_corpus.py`, README `validation/headers-corpus-README.md`), part (a) and part (b),
results in `results/headers-corpus-2026-11-15.{md,tsv}` and `results/headers-rh7-2026-11-15{.md,.tsv,-skips.txt}`, and
`plan/headers.md` §5.1 updated.

## Results that decide things

* **R-H7 (part b):** all 14 undecided legacy headers have a hit, so all are shipped and none is dropped. SL-HEADERS-15 has no
  R-H7 drop to land (its differences rows and compat record are not needed for this list). Accounting: 37,587 source
  packages in `Sources`; 37,579 scanned; 8 excluded by name (avr-libc, dietlibc, glibc, klibc, musl, newlib, picolibc,
  uclibc); 0 failed after retries. Hit counts: `gnu-versions.h` 174, `a.out.h` 42, `regexp.h` 14, `netrom/netrom.h` 9,
  `nfs/nfs.h` 6, `sys/vlimit.h` 6, `sys/vm86.h` 6, `neteconet/ec.h` 4, `gconv.h` 2, `re_comp.h` 2, and one package each for
  `stab.h`, `netash/ash.h`, `netiucv/iucv.h`, `sys/raw.h`.
* **Part (a) (31 source packages: Essential 19 scanned, build-essential 6, smoke 15; glibc excluded by name), set `all`:**
  * Identity macros: `__GLIBC__` in 14 packages (129 files), `__GLIBC_MINOR__` 8, `__GNU_LIBRARY__` 5, `__GLIBC_PREREQ` 2.
  * `__USE_*` (R-H2): `__USE_GNU` 6 packages, `__USE_MISC` 3, `__USE_LARGEFILE64` 3, `__USE_FILE_OFFSET64` 3,
    `__USE_FORTIFY_LEVEL` 2, `__USE_EXTERN_INLINES` 2, `__USE_XOPEN2K` 2. The ruling's `__USE_MISC` and `__USE_GNU` are
    confirmed; the scan adds `__USE_LARGEFILE64`, `__USE_FILE_OFFSET64`, `__USE_FORTIFY_LEVEL` and `__USE_EXTERN_INLINES` as
    candidates (three or two packages each; mostly tests of the names, not definitions). Names in one package are not
    counted as findings here.
  * `sys/cdefs.h` vocabulary (R-H3), by packages: `__glibc_unlikely` 5, `__glibc_likely` 5, `__attribute_warn_unused_result__` 5,
    `__P` 6, `__THROW` 4, `__nonnull` 4, `__BEGIN_DECLS`/`__END_DECLS` 3, `__REDIRECT` 3, `__ASMNAME` 3, `__NTH` 3, `__wur` 2.
    The `__glibc_*` and `__attribute_*__` helpers appear in the tar and gawk sources too, outside the skipped directories;
    I did not open those files to see why (they may be copies of a library header).
  * Reserved types, by packages: `__locale_t` 4, `__pid_t` 4, `__time64_t` 4, `__wchar_t` 3, `__off_t` 2, `__wint_t` 2,
    `__ino64_t` 2. (`__uint128_t` is the compiler's own.) Full list (581 names) in the TSV.
  * FILE members: `_flags` 5 packages (generic name, over-counts), `_IO_read_ptr`, `_IO_read_end`, `_IO_write_base`,
    `_IO_write_ptr`, `_IO_save_base` 2 packages each, `_fileno` 2. The set is small; R-H4 (complete FILE) stays.
  * Direct includes of `bits/` and `gnu/`: `bits/wordsize.h` and `bits/long-double.h` (2 packages each); every other
    `bits/` name is libstdc++'s own (`gcc-14`). No `gnu/` include. No other glibc-internal header name is needed by
    this corpus, so the "whatever SL-HEADERS-1 finds" clause of R-H2 adds only those two.

## Proof

* Two full runs of part (a) from separate work dirs (`index`, `run`, `report`) gave byte-identical `.md` and `.tsv`; a third
  and fourth (earlier tool state) too. Reports from the two final result sets are identical (`sl-headers-1-report`).
* Three hand counts (sed 4.9, grep over the package unpacked with `tar --exclude=lib --exclude=gnulib
  --exclude=gnulib-tests`): `stdio.h` 10/10, `string.h` 7/7, `sys/stat.h` 2/2 (occurrences/files), plus four identifiers.
  All equal the tool.
* No excluded directory was read: the tool drops members whose path has a `lib`, `gnulib` or `gnulib-tests` component
  before reading them; the report lists each skipped directory.

## Notes for the next step

* Part (b) ran in two tool states: pass 1 (37,563 packages) with the first reader, then the 16 packages that failed with
  a reader that handles multi-stream bz2/xz (cause: Python's tar stream reader does not read concatenated streams;
  now external `gzip`/`bzip2`/`xz`/`zstd`) and one git-bundle package (`xlennart`). A package that scanned fine
  under the first reader is single-stream, so the two readers agree on it; not re-verified by a full rerun (about 76 min).
* The slice order and R-H2/R-H3 lists above are for the owner's review; I did not edit §5 rulings.
* `__USE_MINGW_ANSI_STDIO`, `__USE_INIT_FINI__` etc. are other projects' own names, not glibc's.
* A hit is an include, not a build; no package was built.
* No Aqours source patches: nothing was built.

## Not verified

The Essential and build-essential sets are my reading of the policy (see the README); the build-essential set follows the
compiler stubs two levels. Mode `a` counts mentions in comments and `#ifdef` lines.
