# HEADERS-GETTEXT-CLANG: __GNU_GETTEXT_SUPPORTED_REVISION, _FloatN under clang, utmp/ucontext/ifaddrs names

**State (leg 1 ended in a REFRESH, 2026-11-25):** branch `task/headers-gettext-clang` from origin/next db602e58c, role
keke:~/w/r3 (synced, clean next). No code changed yet: leg 1 was the black-box probes below. Scratch (local):
`~/w/.scratch/HEADERS-GETTEXT-CLANG/` (probe.py: compile matrix gcc/g++/clang-19/clang++-19 x stds, reference vs
`-nostdinc -isystem <tree>`; mksnip.py, mksnip2.py: the snippets; summ.py LOG PREFIX: one line per snippet, `*` = the
two sides differ; probe2.log). On keke: `~/work/headers-gettext-clang/` (same scripts, snip/, snip2/), logs
`~/logs/headers-gettext-clang-probe1.log`, `-probe2.log`. clang-19 IS installed on keke (`/usr/bin/clang-19`,
`clang++-19`). Decisions: none yet.

## What Yohane's stage-2 logs say (keke `/srv/aqours/logs/stage2/*.log.zst`), and next's state
* parted (`INQUIRY`, `SCSI_IOCTL_GET_IDLUN` undeclared) and lvm2 (`struct seminfo` incomplete): **already fixed on
  next** (snapshot 25.2's chroot had a 13-line scsi/scsi.h and no sys/sem.h). Probe1: scsi/scsi.h, scsi/scsi_ioctl.h
  codes, `struct seminfo`, SEM_INFO, SEM_STAT, SEM_STAT_ANY, IPC_INFO all equal to the reference in C/C++ gcc/clang.
  Still owed: compile parted's linux.c and lvm2's libdm-common.c as proof (sources below).
* procps/sysvinit `ut_name`, psutil `ifa_broadaddr`/`ifa_dstaddr`, ruby `greg_t`, postgresql-17 `_Float32x` etc.
  under clang-19 (stdlib.h): real gaps on next.

## Probe-settled reference behaviour (black box; keke = Ubuntu GLIBC 2.39, GCC 13.3, clang 19)
* `<libintl.h>`: `__GNU_GETTEXT_SUPPORTED_REVISION(m)` defined in every config (C and C++); value 1 for m == 0,
  -1 for -1, 1, 2, 3, 100000; argument parenthesised (`(1 - 1)` gives 1, `(0 ? 1 : 0)` gives 1); result type int
  (also for a long, unsigned or double argument; `(1u) < 0` is true); argument evaluated once; non-constant argument
  is not an integer constant expression; usable in `#if`. Every result fits `((major) == 0 ? 1 : -1)`.
  gettext.m4 (serial 71, /usr/share/aclocal/gettext.m4 on keke, lines 136-171) needs
  `__GNU_GETTEXT_SUPPORTED_REVISION(0) >= 1` for need-formatstring-macros, plus `ngettext` (need-ngettext), and links
  `bindtextdomain`, `gettext`. libniji's mo.rs already expands system-dependent strings (minor revision 1,
  crates/core/src/catalog/mo.rs header doc): still owed, a real .mo with `<PRId64>` segments and plural forms, stock vs
  libniji (dcngettext/dngettext/ngettext).
* utmp.h: `ut_name`, `ut_time`, `ut_xtime`, `ut_addr` are macros in every config (C, C++), forms by the fake-structure
  test of header_checks/s12.py: `ut_name` = `ut_user`, `ut_time` = `ut_xtime` = `ut_tv.tv_sec`, `ut_addr` =
  `ut_addr_v6[0]`. utmpx.h shows none of them (equal on both sides). utmp.h also shows `UTMP_FILENAME`,
  `WTMP_FILENAME` (libniji: no). utmpx.h shows `__UT_LINESIZE/NAMESIZE/HOSTSIZE` (reserved: R-H3 policy omits; leave).
* utmpx.h `struct __exit_status` (ut_exit's tag in every config): members `e_termination`/`e_exit` with _GNU_SOURCE
  (and C++), `__e_termination`/`__e_exit` without; libniji always `e_*` (strict-mode naming difference, minor).
* ifaddrs.h: `ifa_broadaddr` = `ifa_ifu.ifu_broadaddr`, `ifa_dstaddr` = `ifa_ifu.ifu_dstaddr`, macros, every config.
* sys/ucontext.h: `greg_t` typedef (every config) missing in libniji; `__NGREG` defined by the reference (reserved,
  omit unless needed). In strict C (`-std=c17`, no MISC) the reference names members `__gregs`, `__fpregs`,
  `__uc_flags`, `__cwd`, `__mxcsr` (the `__` spelling); libniji uses `gregs`, `uc_flags`, ... in every config
  (minor, same class as utmpx's exit_status).
* _FloatN, reference: gcc and g++ 13 have the keywords `_Float32 _Float64 _Float32x _Float64x _Float128` and predefine
  `__FLT{32,64,32X,64X,128}_MANT_DIG__`; clang-19 has neither (only `_Float16`, `__float128`). With the reference's
  stdlib.h included, clang C and C++ CAN use `_Float32 _Float64 _Float32x _Float64x` (made usable by the headers) but
  not `_Float128`. Functions: clang C (+_GNU_SOURCE) declares `strtof32/64/32x/64x`, `sinf32`, `sinf64x`,
  `fabsf64`, `cexpf32`, `wcstof32`; no `*f128` under clang; clang C++ declares none of the _FloatN functions; g++
  declares all. `__HAVE_FLOAT32/64/32X/64X` = 1 under both compilers, `__HAVE_FLOAT128` and
  `__HAVE_DISTINCT_FLOAT128` = 1 gcc, 0 clang; `__HAVE_FLOAT16`, `__HAVE_FLOAT128X`, other DISTINCT = 0; all
  defined (libniji defines none: reserved names; internal guard macros are enough unless a package needs them).
  `HUGE_VAL_F128` shown by the reference under gcc (+GNU, C++) not clang; libniji lacks it (INFORMATIONAL, check
  others of that family). libniji under clang: stdlib.h/math.h/wchar.h/complex.h fail in C with _GNU_SOURCE and in
  every C++ mode; tgmath.h fails under clang C and under both C++ compilers (check why: g++ too).

## Plan for leg 2 (exact next step first)
1. Guards: in the generator (xtask/src/headers/functions.rs or where prototypes are emitted), wrap each declaration
   whose text names `_Float128` in `#if __NIJI_HAVE_FLOAT128` and `_Float32|64|32x|64x` in `#if
   __NIJI_HAVE_FLOATN`; define both in features.h.in: keyword support by `defined __FLT128_MANT_DIG__` (resp.
   `__FLT32_MANT_DIG__`); where the 32/64/32x/64x keywords are absent, typedef them (float, double, double, long
   double) so the names are usable as in the reference, and declare the functions in C only. Same for macros
   (M_PIf128, ...) whose literal suffix needs the keyword. Then fix tgmath.h.
2. Tables: header_probe.py `package_names()` += ut_name ut_time ut_xtime ut_addr ifa_broadaddr ifa_dstaddr greg_t
   UTMP_FILENAME WTMP_FILENAME __GNU_GETTEXT_SUPPORTED_REVISION; run it on keke for those names (`--names FILE --out
   DIR`, all headers), splice the rows into visibility.tsv (sort key of header_probe main()); typedefs.tsv greg_t
   (long long, 8, 8) for each header showing it.
3. Templates: utmp.h.in and ifaddrs.h.in `@@owns` + `@@when utmp` / `@@when ifaddrs` defines (net/if.h.in pattern);
   libintl.h.in `# define __GNU_GETTEXT_SUPPORTED_REVISION(major) ((major) == 0 ? 1 : -1)` and reserved.tsv `emit`
   row (reason: gettext.m4 AM_GNU_GETTEXT). `cargo xtask headers-gen`.
4. header_check: s12.py MEMBER_MACROS += utmp.h, ifaddrs.h rows; a unit for the gettext macro value/form; clang:
   header_check.py runs P5 also with clang-19/clang++-19 (-std=c17, gnu2x, c++17, gnu++20) when installed (skip with
   a printed reason otherwise).
5. Proof: sources in `/srv/mirrors/debian/pool/main/<x>/<pkg>/*.dsc` (tar_1.35+dfsg-3.1, postgresql-17_17.12-0+deb13u1,
   sysvinit_3.14-4, ruby3.3_3.3.8-2 (Yohane built 3.3.14 from devuan), python-psutil_7.0.0-2, parted_3.6-5+deb13u1,
   lvm2_2.03.31-2, openldap_2.6.10+dfsg-1); `dpkg-source -x` into /dev/shm/claude/headers-gettext-clang/src; compile
   the failing files with `-nostdinc -isystem ~/w/r3/libc-include -isystem <cc include> -isystem <kernel uapi>`; tar
   configure with CC as such a wrapper linking target/niji (or the stage2 chroot tarball
   /mnt/nvme/aqours/chroots/stage2-niji-20261125.2.tar.zst under `unshare -r`). 6. plants, lint, gate (affected
   escalates to FULL for generated headers: precheck + header_check + xtask tests), report.

Not to redo: the probes above. Leftover processes: none (two stray `find` on keke from leg 1 were killed by PID).
