# CONFIGURE-DIFF report: configure probes that answer differently on libniji than on stock

Run 2026-11-27, Skylake-SP host (48 threads), tool `handoff/configure-diff/` (README there), job
`configure-diff-full1` (+ `configure-diff-retry1` for 15 packages whose chroot never started under load; all 15
finished on the retry's first try). Result files: `handoff/configure-diff/results-20261127.2/`.

* **libniji side:** snapshot-2026-11-27.2's system flavour (`libc6`, `libc-bin`, `libc6-dev`, `libc-dev-bin`
  `2.39+niji20261127.2-1`, next 16e03637e, RELEASE-SNAPSHOT-1127B's debs), installed over trixie's libc in the same
  chroot after the two stock runs.
* **Stock side:** Debian trixie, `libc6 2.41-12+deb13u5`, the same glibc version Aqours builds on. Differences that
  come from 2.41 versus libniji's 2.39 base are marked as such below (one row only: `libsigsegv`'s host string).
* **Package set:** the 438 source packages of Aqours' stage-2 logs (`*.result`), which include the 1127B ybuild
  set (util-linux, inetutils, npth, gnupg2, libev, keyutils), perl and apt. Debian sources at Aqours' versions;
  Devuan-versioned ones (`...devuan1`, e.g. apt, util-linux, coreutils) ran trixie's version of the same package.
  gcc-14 runs libstdc++-v3's own configure (standalone, `g++-14 -nostdinc++`, upstream tree), not Debian's build.
* **Configure step:** the package's dh `build` sequence up to and including `dh_auto_configure` (or its overrides)
  under `dpkg-buildpackage -T` (Debian's flags and overrides); non-dh packages run their rules' configure target.
* GAPS-L had not landed on next during the run, so there is no run against next.

## Totals

| | |
|---|---|
| packages | 438 listed, 437 compared (`eudev`: Devuan-only, not on the Debian mirror) |
| packages with probe files | 297 (140 have none: perl and python modules, plain Makefile packages) |
| probes compared | 183,040 |
| noisy (stockA != stockB, never counted) | 4 rows = 1 probe in 2 packages: gnulib's `pthread_rwlock` wait-queue runtime test (gettext, libunistring) |
| **stock vs stock** (`diff.py --sides stockA,stockB`) | 4 rows, the same probe; **0** without that probe, which is non-deterministic on stock itself |
| differing (stock agrees with itself, libniji differs) | 220 rows in 45 packages: 15 libniji header/name/configuration causes (L1-L15), 7 behaviour (B1 unrecorded, R1-R6 recorded), 3 harness/scope |

Per package (probes, noisy, differing, primary probes, each run's rc and files): `results-20261127.2/summary.txt`;
every differing row: `diff.tsv`. Primary = an autoconf cache variable, meson check, CMake cache entry or perl answer;
the other rows are `config.h` defines and autoconf output variables that repeat a primary answer.

## libniji class: header, name, configuration (rows for a header story)

What the probe compiles is quoted from the package's `config.log` (`why.py <run> <pkg> <regex>` prints it).

| # | Header | Name | Configuration | Packages (probe) | What the probe compiles; stock / libniji |
|---|---|---|---|---|---|
| L1 | `<math.h>` | `fpclassify`, `isfinite`, `isnormal`, `signbit` (the first errors; the program also uses the other C99 classification and comparison macros) | C++98 and C++11 (`g++ -nostdinc++ -std=c++98 -fno-exceptions`), no feature macro | gcc-14 libstdc++-v3 (`glibcxx_cv_c99_math_cxx98`, `_cxx11`; then `_GLIBCXX98_USE_C99_MATH`, `_GLIBCXX11_USE_C99_MATH`, `_GLIBCXX_USE_C99` off) | `#include <math.h>`, `i = fpclassify(d1); i = isfinite(d1); ...`; stock compiles, libniji: "'fpclassify' was not declared". **Yohane's 27.2 gap, found at its source.** |
| L2 | `<string.h>` | GNU `strerror_r` declared `warn_unused_result` | `gcc -c -Wimplicit-function-declaration -Werror`, no `-O`, no `_FORTIFY_SOURCE` | gcc-14 libstdc++-v3 (`HAVE_STRERROR_R`, `_GLIBCXX_HAVE_STRERROR_R` off) | `char s[128]; strerror_r(5, s, 128);` (result ignored); stock compiles, libniji: "ignoring return value ... [-Werror=unused-result]" |
| L3 | `<sys/cdefs.h>` | `__GNUC_PREREQ` | any | gnulib "glibc-compatible sys/cdefs.h": coreutils, findutils, gettext, gnutls28, inetutils, libidn, libidn2, libpipeline, libtasn1-6, libunistring, man-db, nano, patch, texinfo, wget | `#include <sys/cdefs.h>` / `enum { foo = __GNUC_PREREQ (14, 1) } bar;`; libniji: implicit declaration of `__GNUC_PREREQ` |
| L4 | `<glob.h>` / `<gnu-versions.h>` | `_GNU_GLOB_INTERFACE_VERSION` | `_GNU_SOURCE` | gnulib glob: inetutils, man-db, nano (`gl_cv_gnu_glob_interface_version_1_2`; derived: `gl_cv_glob_*`, `getpwnam_r`, `gl_cv_onwards_func_getpwnam_r`); GNU make "system libc has working GNU glob" (make-dfsg, both flavours) | `char a[_GNU_GLOB_INTERFACE_VERSION == 1 \|\| _GNU_GLOB_INTERFACE_VERSION == 2 ? 1 : -1];`; libniji: undeclared. make's program (`<features.h>`, `<gnu-versions.h>`, `<glob.h>`, `<fnmatch.h>`) exits 1 on libniji |
| L5 | `<features.h>` (so `<limits.h>`) | the `__stub_*` macros of `<gnu/stubs.h>` | any | AC_CHECK_FUNC stub test: setlogin (heimdal, inetutils, openssh), chflags (e2fsprogs, libarchive, tcl8.6), fchflags (libarchive), revoke (fakechroot, heimdal); derived heimdal `ac_cv_func_getlogin_posix` | `#include <limits.h>` / `#if defined __stub_setlogin \|\| defined __stub___setlogin` / `choke me`; stock: the macro is visible (fails, "no"), libniji: `<gnu/stubs.h>` defines it but `<features.h>` does not include that header, so the check links and says "yes" |
| L6 | `<dirent.h>` | `struct dirent` member `d_fileno` (the GNU C Library manual documents it beside `d_ino`) | default | bash, readline (`ac_cv_member_struct_dirent_d_fileno`, `bash_cv_dirent_has_d_fileno`), apr (`apr_cv_dirent_inode` picks `d_fileno` first) | `static struct dirent ac_aggr; if (ac_aggr.d_fileno) return 0;`; libniji: "no member named 'd_fileno'" |
| L7 | `<bits/types.h>` | the header (libniji's include tree has no `bits/`) | default | openldap (`ac_cv_header_bits_types_h`), psmisc (`struct user_regs_struct.rax`, `.rdi`, `.rsi`, `.rdx`, `.orig_rax`: the test includes `<bits/types.h>` before `<sys/user.h>`) | `#include <stddef.h>` / `#include <bits/types.h>`; libniji: "No such file or directory" |
| L8 | `<utmpx.h>` | `__UT_HOSTSIZE` | default | procps (`ac_cv_have_decl___UT_HOSTSIZE`, `HAVE_UT_HOSTSIZE_IN_UTMPX`) | AC_CHECK_DECLS after `#include <utmpx.h>`; libniji: undeclared |
| L9 | `<pthread.h>` | `PTHREAD_MUTEX_ERRORCHECK_NP`, `PTHREAD_MUTEX_RECURSIVE_NP` (`PTHREAD_MUTEX_ADAPTIVE_NP` is there) | `_GNU_SOURCE` | valgrind (`HAVE_PTHREAD_MUTEX_ERRORCHECK_NP`, `HAVE_PTHREAD_MUTEX_RECURSIVE_NP`) | `#include <pthread.h>`, the name as an expression; libniji: undeclared |
| L10 | `<unistd.h>` | `__environ` | C++ (`g++ -std=gnu++17`, so `_GNU_SOURCE`) | google-perftools (`HAVE___ENVIRON`) | `#include <unistd.h>` / `char **env = __environ;`; libniji: "not declared". **Yohane's 27.2 `__environ` gap.** |
| L11 | `<math.h>` via `<cmath>` | `__exp10` (declaration only: stock's libm exports `exp10`, no `__exp10`) | C++ | xapian-core (`ac_cv_have_decl___exp10_double_`) | AC_CHECK_DECLS `__exp10(double)` after `#include <cmath>`; low value: `exp10` is found on both sides |
| L12 | `<stdlib.h>` (feature levels) | `strtoll`, `strtoull` | `_REENTRANT` and `_THREAD_SAFE` defined, no other feature macro | expect (tcl's "required early compiler flags": `tcl_cv_flag__isoc99_source` no -> yes, so expect adds `-D_ISOC99_SOURCE`) | `#include <stdlib.h>` / `char *p = (char *)strtoll; char *q = (char *)strtoull;`; stock compiles, libniji: undeclared. `_REENTRANT` alone turns libniji's default feature set off |
| L13 | `<features.h>` | the deprecation `#warning` for `_BSD_SOURCE` / `_SVID_SOURCE` without `_DEFAULT_SOURCE` (feature_test_macros(7) documents it) | `-Werror` | dhcpcd (its own configure: explicit_bzero test, `HAVE_EXPLICIT_BZERO` libniji only) | `#define _BSD_SOURCE` then `<string.h>`; stock: `#warning "_BSD_SOURCE and _SVID_SOURCE are deprecated, use _DEFAULT_SOURCE"` is an error under -Werror, libniji: silent |
| L14 | (export) | `__libc_enable_secure` (stock: GLIBC_PRIVATE data in the loader; stock's libc.so.6 imports it) | link | p11-kit meson (`HAVE___LIBC_ENABLE_SECURE`) | links `extern int __libc_enable_secure`; libniji: undefined. RULING NEEDED below |
| L15 | `<signal.h>`, `<sys/signal.h>` (file text) | the text `SIGTSTP` | perl Configure greps the two files | perl (`d_bsd` undef -> define) | `cat /usr/include/signal.h /usr/include/sys/signal.h \| grep SIGTSTP`; stock's two files do not contain the text, libniji's do |

## Behaviour (runtime probes)

| # | What | Packages (probe) | Record |
|---|---|---|---|
| B1 | `abort()` called again from inside a SIGABRT handler: libniji runs the next handler (the probe's third handler `exit(0)`s); stock ends the process with SIGABRT (status 134) | libsodium (`ax_cv_check_cCATCHABLE_ABRT` no -> yes) | **not recorded**: RULING NEEDED below |
| R1 | `fflush` of a seekable input stream after `ungetc` | gnulib `gl_cv_func_fflush_stdin` (coreutils, diffutils, findutils, gettext, gnutls28, grep, gzip, inetutils, libtasn1-6, m4, sed, sharutils, tar, wget); derived `gl_cv_func_fclose_stdin`, `_fseeki64`, `HAVE_FSEEKO` | recorded (differences row "fflush on a seekable input stream after ungetc") |
| R2 | `SIGRTMIN` 33 (stock 34) | perl `sig_name`, `sig_name_init` | recorded (D-20.1, D-23.3) |
| R3 | `gcvt` with more than 17 digits | perl `d_Gconvert` (gcvt passes perl's 53-digit test, so perl uses gcvt; stock uses sprintf) | recorded (D-8.10) |
| R4 | `pthread_rwlock` waiting order | gnutls28 `gl_cv_func_pthread_rwlock_good_waitqueue` (and noisy on stock in gettext, libunistring) | recorded |
| R5 | iconv EUC-CN not open | texinfo `texinfo_cv_sys_iconv_converts_euc_cn` | recorded ("Not open yet: ... EUC-CN") |
| R6 | no `dlmopen` | fakechroot `ac_cv_func_dlmopen` | owner deferral (DL-22) |

## Harness and scope (not libniji header gaps)

* **2.41 vs 2.39:** libsigsegv `sv_cv_host` embeds the C library version (`...-gnu-glibc2.41` vs `glibc2.39`).
* **x86-64 only:** readline's `build32` flavour (`gcc -m32`, 20 primary rows: every header "no", `sizeof wchar_t`
  0, sigsetjmp "missing", file offset bits unknown). libniji's `<stdio.h>` declares `size_t` as `unsigned long`
  ("conflicting types for 'size_t'"); libniji ships no i386 multilib. INFORMATIONAL.
* **libniji's answer is the one the probe wants:** coreutils `gl_cv_clean_version_stddef` (stock's `<time.h>`
  leaves `__STDC_VERSION_STDDEF_H__` defined after an `#undef`, gcc bug 114870; libniji's does not), so gnulib
  replaces `<stddef.h>` on stock only.

## Sanity against Yohane's 27.2 report

* `<math.h>` C99 macros in C++: **present** (L1, gcc-14 libstdc++-v3, the probe that said "no" silently).
* `__environ`: **present** (L10, google-perftools).
* `<printf.h>` `PA_*`: **not seen.** No configure step among the 437 packages probes `<printf.h>` (no `printf`/`PA_`
  key in any probe file); that gap stops a compile, not a configure check, so this method cannot see it.
* Pilot (10 packages, job `configure-diff-pilot2`): the same rows for its packages; the pilot's gcc-14 ran without
  `-nostdinc++` (libstdc++'s own `<math.h>` wrapper hid L1), which is why the driver now passes it.

## RULING NEEDED

* **B1 nested `abort()`**: record a differences row (libniji's behaviour lets libsodium's configure decide SIGABRT
  is catchable) or match stock. Recommend: header/behaviour story checks what POSIX `abort` requires for a handler
  that calls `abort` again, then records or matches; until then libsodium builds with `HAVE_CATCHABLE_ABRT` on
  libniji only.
* **L14 `__libc_enable_secure`** (a GLIBC_PRIVATE loader symbol): recommend not exporting it (private ABI);
  p11-kit's configure takes its "no" branch on libniji. Revisit only if a package fails without it.
* **L13 `_BSD_SOURCE` warning** and **L15 perl's text grep**: matching them means emitting a deprecation warning and
  keeping `SIGTSTP`'s text out of `<signal.h>`/`<sys/signal.h>`. Recommend matching both (cheap, and each changes a
  package's configuration today).

## Exact next step

A header story (GAPS-L or the next) takes L1-L13 as rows; L14 and B1 need the rulings above. Later snapshot stories
rerun `cdiff.sh` with their debs (one job, about 95 minutes for 438 packages at 20 at a time on the Skylake-SP
host) and expect only the rows of "Behaviour" (recorded) and "Harness and scope".
