# Header and reserved-name corpus scan

`validation/headers_corpus.py` (Python 3, standard library only). Two scans over the project's Devuan
excalibur source archive (SL-HEADERS-1; `plan/headers.md` §4 slice 1 and §5.1):

* **Mode `a`**: the program sources of the Essential set, the build-essential set and the smoke corpus
  (`validation/smoke/programs.sh`, its `*_src=` names). Per header: `#include <...>` counts. Also the uses of
  the identity macros (`__GLIBC__`, `__GLIBC_MINOR__`, `__GLIBC_PREREQ`, `__GNU_LIBRARY__`), `__USE_*`,
  the `sys/cdefs.h` vocabulary, reserved `__*_t` type names, the FILE member names of `spec/abi/layouts.tsv`,
  and direct `bits/` and `gnu/` includes. Output: `results/headers-corpus-<date>.md` and `.tsv`
  (columns `set kind name occurrences files packages`).
* **Mode `b`** (R-H7): for each legacy header that `plan/headers.md` §5.1 lists as undecided (`UNDECIDED` in the
  tool), which source packages of the whole `main/source/Sources.xz` include it. Output:
  `results/headers-rh7-<date>.tsv` (`package<TAB>header`), `.md` and a skip log.

## Method

1. `index`: fetch `dists/excalibur/InRelease`, verify it with `gpgv` and the pinned Devuan keyring
   (`~/keyrings/devuan-archive-keyring.gpg`, authenticated as in `plan/headers.md` §1), then fetch
   `main/source/Sources.xz` and `main/binary-amd64/Packages.xz` and check each against its SHA-256 in
   the verified `InRelease`. Only the LAN mirror is used.
2. `run`: one package at a time per worker. Each source file is downloaded to a RAM directory
   (`--tmp`, default `/dev/shm/claude/SL-HEADERS-1`), checked against its SHA-256 in `Sources`, scanned
   by streaming (tar, zip, nested tarballs, `.diff.*`) and deleted; the package directory is removed
   afterwards whatever happens. A byte budget (`--cap`) bounds the sources in flight. Results are appended to
   `results-<mode>.jsonl`; a package that failed is retried (`--passes`) and the job exits 1 if any is left.
3. `report`: reads the jsonl and writes the sorted, date-stamped outputs. Nothing in the outputs depends on
   run time, so two runs on the same index give byte-identical files.

## What is never read

* Any path component named `lib`, `gnulib` or `gnulib-tests`: the member is skipped and its content dropped
  unread (a compressed tar stream has to be decompressed past it). Each skipped directory is logged in the
  per-package result (`skips`) and in the report.
* The C library source packages, by name (`CLIB_RE`: glibc, musl, dietlibc, klibc, newlib, picolibc, uclibc,
  avr-libc, bionic), and nested archives with such a name (a compiler source tree bundles `newlib-*.tar.xz`),
  are listed in `excluded-<mode>.tsv` and the report and are not scanned.
* Only files with a code extension (`CODE_EXT`) are scanned; `configure` and `.m4` files are not (their test
  programs are not uses by the package). Members over 48 MiB are counted, not scanned.

## Limits

* The Essential set is the packages with `Essential: yes` in `main/binary-amd64`; the build-essential set is
  the source packages of `build-essential` and of its direct `Depends`, followed through the `gcc-defaults`
  stubs to the compiler and binutils sources.
* In mode `b` a hit is discarded when the same package ships a file of that header's name outside the
  skipped directories (a quoted include of the package's own header is not a use of the libc header).
* Nested tarballs that cannot be read as tar (test fixtures) are logged as skipped, not as failures.
* FILE members and `__*_t` names over-count: see the notes at the end of each report.

## Run

    python3 -I validation/headers_corpus.py index  --work W
    python3 -I validation/headers_corpus.py run    --mode a --work W
    python3 -I validation/headers_corpus.py report --mode a --work W --date YYYY-MM-DD --out-dir results
    # whole mirror, one long detached job (about 98 GB of source files):
    python3 -I validation/headers_corpus.py run --mode b --work W --jobs 16 --cap 17000000000 --passes 6

Hand counts: `grep -rE` over the same package unpacked with `tar --exclude=lib --exclude=gnulib
--exclude=gnulib-tests` agreed with the tool for `stdio.h`, `string.h`, `sys/stat.h`, `__glibc_likely`,
`__glibc_unlikely`, `__attribute_maybe_unused__` and `__USE_MINGW_ANSI_STDIO` in sed 4.9 (occurrences and files).
