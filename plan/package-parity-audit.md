# Package parity audit: what Debian's C library packages ship that ours do not (SYS-PACKAGE-PARITY-AUDIT)

A ranked list of the gaps between the binary packages Debian trixie builds from its C library source
(the reference, version 2.41-12+deb13u4, 16 binary packages) and libniji's system-flavour packages
(`tools/deb/build.sh --flavour system`, built from `next` 28740dd5e, version 2.39+niji20261116.1-1,
6 packages). This document only lists and ranks; it fixes nothing. The ranking says how many Debian
trixie packages would notice each gap.

## Method and data

`tools/deb/parity.sh WORK OURS-DEBS results/package-parity` reruns the whole audit (`tools/deb/parity.py`).
All data is in `results/package-parity/`:

| file | what | lines |
|---|---|---|
| `index-meta.json` | trixie, -updates and -security indices: dates; SHA256 checked against InRelease (signature not checked: no Debian keyring on the build host) | - |
| `ref-control.tsv`, `ours-control.tsv` | control fields; control members (name, size); conffiles, triggers, shlibs, the header lines of `symbols` | 363 / 95 |
| `ref-files.tsv`, `ours-files.tsv` | every file: package, type, size, path, link target | 10563 / 93 |
| `ref-elf.tsv`, `ours-elf.tsv` | every ELF under a lib/bin directory: SONAME, NEEDED (i386/x32 marked) | 860 / 22 |
| `ref-dynsyms.tsv`, `ours-dynsyms.tsv` | defined dynamic symbols of the x86-64 shared objects: soname, symbol, version, `@@`/`@`, type | 5148 / 4216 |
| `gaps.tsv` | every gap, with the inventory line it comes from (`parity.py compare`) | 3028 |
| `consumers-packages.tsv` | exact counts: binary packages of other sources that (Pre-)Depend/Recommend/Suggest each reference package; source packages that Build-Depend on it | 80 |
| `consumers-sonames.tsv`, `consumers-imports.tsv` | ELF scan: NEEDED sonames and `GLIBC_*` imports (library, symbol, version), packages per item | 4396 / ~1900 |
| `consumers-users.tsv` | each gap item with users in the scan: count, packages that depend on a user (one level), the users' names | 19 |
| `consumers-meta.json` | population 23193 (trixie amd64 packages of other sources that depend on libc6), sample 4000 drawn at random (seed 20261116), 4000 scanned, 0 failed | - |

Clean room: package metadata, file names/types/sizes, ELF dynamic tables only. Maintainer-script
bodies were not read (names and sizes only); `glibc-source` was listed, never unpacked; data files
(locales, charmaps, translations) were listed by name only.

**Counts.** "Scan" counts are packages of the 4000-package random sample; ×5.8 estimates the archive
(23193 / 4000). A gap with 0 users in the sample has fewer than about 17 users archive-wide at 95 %
(rule of three). "Exact" counts come from the full Packages/Sources indices.

**Gap classes** (brief step 3): (a) files 1606 rows; (b) sonames 579 rows, of which 10 are x86-64
libraries, 6 gconv helper libraries, the rest i386/x32 multilib; (c) (symbol, version) pairs 627 rows:
308 GLIBC_PRIVATE, 96 after 2.39, 58 multi-version, 125 other missing (94 libc.so.6, 26
libc_malloc_debug.so.0, 3 loader, 2 libm); (d) 10 packages; (e) 268 control differences.

**Impact kinds:** RUN = blocks running Debian binaries; BUILD = blocks building; DEGRADE = silently
degrades a build or a system; COSMETIC.

## Ranked table

| # | gap (class) | inventory line | users | impact | story |
|---|---|---|---|---|---|
| 1 | libm.so.6 multi-version pairs at GLIBC_2.2.5: 16 of the 35 have users (`fmod` 25, `pow` 12, `fmodf` 9, `exp` 8, `log` 7, `exp2`/`log2` 4, `hypot`/`hypotf`/`log2f`/`logf` 3, `expf` 2, `exp10`/`exp2f`/`lgamma`/`powf` 1); libc.so.6's 23 multi-version pairs: 0 (c) | `ref-dynsyms.tsv:4067` (fmod) …; `consumers-users.tsv` | scan 25 for `fmod` alone (~145 archive), 13 depend on them | RUN | covered: **SYS-COMPAT-VERSIONS** (its list; not redone here) |
| 2 | `libmvec.so.1` (b) and its dev files `libmvec.so`, `libmvec.a` (a) | `ref-elf.tsv:275`; `ref-files.tsv:1981` | scan 18 (~104), 12 depend on them (guitarix, splash, xfoil, raster3d …) | RUN | ledger **M-0/M-1**: re-prioritise (evidence here). Proposed brief: `handoff/SYS-PACKAGE-PARITY-AUDIT/SYS-LIBMVEC.md` (M) |
| 3 | `libresolv.so.2` (b), `libresolv.so` dev link (a) | `ref-elf.tsv:283` | scan 13 (~75: msmtp, netcat-openbsd, mailutils, connman, cifs-utils, opensmtpd …), 41 depend on them; libkrb5's dependents on top | RUN, DEGRADE | covered: **SYS-LIBRESOLV-2** |
| 4 | 478 headers, `usr/include` (a) | `ref-files.tsv:1801` (stdio.h), 1467 | every C build (libc6-dev: Depends 204, build-essential) | BUILD | covered: **SL-HEADERS** (slices) |
| 5 | `locales-all` package (d) | `ref-control.tsv:336` | exact: Build-Depends 108 sources, Depends 98, Recommends 10 | BUILD (108 sources), RUN (98 installs) | new: `SYS-LOCALES-ALL.md` (M; after SL-LOCALEDEF, ruling R5) |
| 6 | `locales` content: `locale-gen`, `update-locale`, `validlocale`, `/etc/locale.alias` (conffile), 605 files in `usr/share/i18n` (charmaps, locale sources) (a, e) | `ref-files.tsv:2828`, 2777; `ref-control.tsv:324` | exact: Depends 114, Pre-Depends 1, Build-Depends 65, Recommends 15; our placeholder satisfies the relation but ships nothing | DEGRADE (maintainer scripts that run `locale-gen` fail) | covered: **SL-LOCALEDEF** (programs), **SYS-ICONV-CHARMAPS** (charmaps), R5 (data) |
| 7 | libc6-dev `Depends: linux-libc-dev, libcrypt-dev, rpcsvc-proto` (e) | `ref-control.tsv:165` | not counted: every build that includes `<linux/…>`, `<crypt.h>` or runs `rpcgen` without naming the package (libcrypt.so.1: scan 33) | BUILD | new: `SYS-CONTROL-PARITY.md` (S) |
| 8 | libc6-dev `Provides: libc-dev (= <version>)`: ours is unversioned (e) | `ref-control.tsv:169`, `ours-control.tsv:81` | ≤ 2 sources with `libc-dev (>= 2.38)` (the count does not separate `linux-libc-dev`); an unversioned Provides satisfies no versioned relation | BUILD | new: `SYS-CONTROL-PARITY.md` |
| 9 | libc-bin programs `locale`, `iconv`, `pldd`, `iconvconfig`, `/usr/bin/ld.so` link; `localedef`; `zic`, `zdump`, `tzselect` (a) | `ref-files.tsv:506`, 515, 488, 518, 496, 514, 493 | not counted (maintainer-script bodies are not read); libc-bin is Essential | RUN (scripts), BUILD (tzdata needs `zic`) | covered: **SL-BIN-CORE**, **SL-LOCALEDEF**, **SL-TZ** (R7) |
| 10 | `gencat` (libc-dev-bin) (a) | `ref-files.tsv:529` | exact: libc-dev-bin Build-Depends 5 sources | BUILD | covered: **UTIL-14** |
| 11 | `libthread_db.so.1` (b), `libthread_db.so` (a) | `ref-elf.tsv:285` | scan 1 (~6); debuggers load it by name (not counted by NEEDED) | RUN (1), DEGRADE (thread debugging) | new: `SYS-THREAD-DB.md` (M) |
| 12 | `libc6-dbg` package (d) | `ref-control.tsv:154` | exact: Depends 1, Build-Depends 4 | DEGRADE (debuggers, valgrind) | new: `SYS-DBG-PACKAGE.md` (S) |
| 13 | `libBrokenLocale.so.1` (b); dev links `libanl.so`, `libBrokenLocale.so`, `libc_malloc_debug.so`, `libnss_compat.so`, `libnss_hesiod.so`; archives `libg.a`, `libmcheck.a`, `libpthread_nonshared.a`, `libBrokenLocale.a`, `libm-2.41.a` (a) | `ref-elf.tsv:268`; `ref-files.tsv:1979` (libanl.so), 1493, 1946 | scan 0 | DEGRADE (`-lanl` falls back to the static archive; `-lBrokenLocale` fails to link) | new: `SYS-DEV-LINKS.md` (S). SL-SONAMES-2 landed, yet the system flavour ships no `libBrokenLocale.so.1` |
| 14 | `libc_malloc_debug.so.0` exports none of its 26 pairs (`malloc`, `mtrace`, `mcheck`, the hooks) (c) | `ref-dynsyms.tsv:3266` | scan 0 (it is preloaded, not linked) | DEGRADE (`LD_PRELOAD` does nothing) | SL-DEVTOOLS scope; RULING NEEDED (placeholder per D-27.9, or implement) |
| 15 | loader `_r_debug@@GLIBC_2.2.5`, `_dl_mcount@@GLIBC_2.2.5`; libc `_dl_mcount_wrapper`, `_dl_mcount_wrapper_check` (c) | `ref-dynsyms.tsv:37`, 32, 931 | scan 0 | DEGRADE (debuggers, profilers) | DL-23 (debugger support): add to its scope |
| 16 | `libnss_compat.so.2`, `libnss_hesiod.so.2`, `libnss_files.so.2`, `libnss_dns.so.2` (b) | `ref-elf.tsv:277`–280 | scan 0 (loaded by name from nsswitch.conf) | DEGRADE (`passwd: compat`) | SL-NSS (compat); files/dns are built in; hesiod not needed |
| 17 | 308 GLIBC_PRIVATE pairs (c) | `ref-dynsyms.tsv:1082` … | scan 0; archive 4 packages (LIELLA-SYSTEM-LIBC-PLAN) | RUN (4) | covered: D-27.9, **SL-GLIBC-PRIVATE-CONSUMERS** |
| 18 | 96 libm pairs at GLIBC_2.40/2.41 (`exp2m1` … `atan2pi`) (c) | `gaps.tsv` "after 2.39" | scan 0; archive: r-base (plan/system-libc.md §1) | RUN (r-base) | covered: ruling R3, **LIBM-C23-*** |
| 19 | 94 other libc.so.6 pairs, all at GLIBC_2.2.5 and mostly non-default (`__ctype_b`, `_IO_vfscanf`, `__res_*`, `__sigaddset` …), `__libc_sa_len@@`; libm `matherr`, `_LIB_VERSION` (c) | `ref-dynsyms.tsv:318`, 732, 4295 | scan 0: binaries built against these libraries cannot bind the non-default ones; only older foreign binaries can | COSMETIC for trixie | none proposed; add one when a user appears (list in `gaps.tsv`) |
| 20 | multilib `libc6-i386`, `libc6-x32`, `libc6-dev-i386`, `libc6-dev-x32` (d) | `ref-control.tsv:203`, 261, 180, 192 | exact: Depends 32 / 19 / 4 / 3; Build-Depends 1 / 0 / 9 / 3 | RUN (installs of those packages) | none: x86-64 only (plan/system-libc.md §2) |
| 21 | `libc-devtools` (d): `memusage`, `mtrace`, `sotruss`, `sprof`, `memusagestat`; `libmemusage.so`, `libpcprofile.so` (b) | `ref-control.tsv:59`; `ref-elf.tsv:274`, 281 | exact: no relation from another source | COSMETIC | covered: **SL-DEVTOOLS** |
| 22 | `nscd` (d) | `ref-control.tsv:349` | exact: Recommends 5 | COSMETIC | none: R8 (drop) |
| 23 | `libc-l10n` translations, 39 `.mo` files (a) | `ref-files.tsv:636` | exact: Depends 1 (other than locales) | DEGRADE (messages in English) | R5 |
| 24 | gconv: 256 modules, `gconv-modules` configuration, 6 helper sonames (`libJIS.so` …) (a, b) | `ref-files.tsv:954`; `ref-elf.tsv:261`–266 | scan 0 | COSMETIC (iconv builds converters from charmaps, D-6.8) | converter coverage: **SYS-ICONV-CHARMAPS** |
| 25 | control and small files: libc6 `Depends: libgcc-s1` (no use of libgcc_s found in the product code), `Recommends: libidn2-0`, `Suggests`; libc6 and locales debconf `templates`/`config`/`postinst`/`preinst` (service restarts on upgrade); conffiles `/etc/bindresvport.blacklist`, `/etc/default/nss`; `Multi-Arch: foreign` on libc-l10n; `Priority` values; `Breaks` (libc6 29, libc6-dev 105: upgrades from older releases); `usr/lib/locale/C.utf8` (built in); `usr/share/gdb` | `ref-control.tsv:79`, 98, 32, 69, 78, 163; `ref-files.tsv:486`, 502, 468, 1461 | not applicable | COSMETIC, except the conffiles if the functions read them (to check) | `SYS-CONTROL-PARITY.md` (the conffiles, Multi-Arch, Priority); the rest none |
| 26 | `glibc-doc`, `glibc-source` (d) | `ref-control.tsv:3`, 13 | exact: Suggests 14; Build-Depends 3 (cross-toolchain sources) | COSMETIC; BUILD for the 3 | none (a libniji source package is a separate question) |

## Proposed stories (briefs under `handoff/SYS-PACKAGE-PARITY-AUDIT/`)

| story | gaps | size | why first |
|---|---|---|---|
| SYS-CONTROL-PARITY | 7, 8, 25 (conffiles, Multi-Arch, Priority) | S | blocks building, package metadata only |
| SYS-LIBMVEC (ledger M-0/M-1) | 2 | M | the second-largest RUN gap after SYS-COMPAT-VERSIONS |
| SYS-DEV-LINKS | 13 | S | link-time names that silently pick the wrong object |
| SYS-DBG-PACKAGE | 12 | S | the libc6-dbg gap Yohane named |
| SYS-THREAD-DB | 11 | M | debugger support for threaded programs |
| SYS-LOCALES-ALL | 5 | M | 108 sources cannot build; waits on SL-LOCALEDEF and R5 |

## Not covered by this audit

* Which packages' maintainer scripts call libc-bin's programs (bodies not read): rows 6 and 9 are not counted.
* Header contents (SL-HEADERS owns them), and behaviour differences of symbols both sides export.
* Transitive users beyond one level of Depends (row 3: libkrb5's dependents).
