# HEADERS-STAGE2-GAPS-M: handoff (run 1, leg 1: probes done, no code yet)

Branch task/headers-stage2-gaps-m, cut from origin/next af6f8acaf, no merge. Role shiki:~/w/sh1 (stock glibc 2.39,
Ubuntu 2.39-0ubuntu8.9, so black-box probes of the reference run there directly). Story scratch (local):
~/w/.scratch/HEADERS-STAGE2-GAPS-M/ (probe.py, stubs.py, l12.py, abrt.c, t241.sh). The same scripts are on shiki in
~/work/headers-stage2-gaps-m/; outputs in /dev/shm/claude/headers-stage2-gaps-m/ (p-stock.txt, p-niji0.txt = libniji
at af6f8acaf, s-stock.txt, s-niji0.txt, d0.txt = their diff).

Run a probe: `python3 ~/work/headers-stage2-gaps-m/probe.py OUT [--inc ~/w/sh1/libc-include] [--only REGEX]` (stock
without --inc); `python3 ~/work/headers-stage2-gaps-m/stubs.py ~/w/sh1 OUT [--inc ...]`.

## Coordinator notes in force (22:37Z)
* #8823: B1 (nested abort) DROPPED. 2.39 answers 0 for libsodium's form (the same as libniji); 134 was trixie's
  2.41. No library change, no differences row. In the cdiff rerun classify B1 as 2.41-vs-2.39 (harness and scope).
* Every row: confirm stock 2.39 differs from libniji before changing it. A row where 2.39 already agrees with libniji
  is 2.41-only: list it (header, name, package, probe), change nothing.

## Probe-settled stock 2.39 behaviour (all differ from libniji unless noted)
* L2: `-Wunused-result` is only emitted by `gcc -c` (never -fsyntax-only: probe.py's L2 rows must compile with -c,
  fix that first). Stock: GNU strerror_r carries warn_unused_result only under fortify (-O2 with _FORTIFY_SOURCE
  >= 1); without -O, at -O2 -U_FORTIFY_SOURCE, and -O0 -D_FORTIFY_SOURCE=2: no warning. strdup and realloc warn in
  every configuration. libniji: strerror_r warns everywhere. Plan: attr_probe.py table runs the `r` tests again in
  config `gnu` (fortify off; corpus(..., every=True), keep only kind r) and writes wur `1` (both) or `fortify`
  (fortify build only; exit if gnu-only); check mode: `--kinds r` outside base compares `-Wunused-result` alone
  (a FOCUS dict = MEASURES + {"r": {"-Wunused-result"}}, used only for focus). Generator: xtask/src/headers.rs
  ~line 598 and functions.rs redirect_line (line 16): wur `fortify` appends ` __NIJI_FWUR`, a macro defined in
  headers/features.h.in beside __NIJI_NOTHROW: `__attribute__ ((__warn_unused_result__))` when __NIJI_FORTIFY > 0,
  else empty. Also probe stock's `__wur` / `__attribute_warn_unused_result__` on a program's own function (probe.py
  "L2 own-wur", needs -c) and match in headers/sys/cdefs.h.in (libniji defines __wur unconditionally today).
* L3: `__GNUC_PREREQ(maj, min)` defined after features.h, sys/cdefs.h, string.h, limits.h, C89..C++17, value
  (__GNUC__ << 16) + __GNUC_MINOR__ >= ((maj) << 16) + (min) (gcc 13 there: (13,0)=1, (13,1)=0, (4,0)=1, (99,0)=0).
  `__glibc_clang_prereq` also defined, 0 under gcc. libniji: neither. RISK to check before landing: gnulib's
  gl_cv_header_sys_cdefs_h then says yes, so the 15 gnulib packages use the system <sys/cdefs.h> (libniji's has 20
  macros only); a black-box build of one gnulib package's lib/ against the changed headers is the test (never open
  gnulib lib/ sources). Report, do not chase, if it fails.
* L4: <gnu-versions.h> (C and C++, with and without _GNU_SOURCE): _GNU_GLOB_INTERFACE_VERSION 2,
  _GNU_OBSTACK_INTERFACE_VERSION 1, _GNU_REGEX_INTERFACE_VERSION 1, _GNU_GETOPT_INTERFACE_VERSION 2;
  _GNU_NSS_INTERFACE_VERSION undefined. <glob.h>, <features.h>, <getopt.h>, <obstack.h>, <regex.h> show none (same on
  both sides). The make-style glob program (probe.py GLOBTEST, my reconstruction: version == 1) exits 1 on 2.39 AND on
  libniji: make's row is 2.41-only (report it). gnulib's `_GNU_GLOB_INTERFACE_VERSION == 1 || == 2` needs
  <gnu-versions.h> (fails after <glob.h> alone on stock too). Before adding OBSTACK/REGEX/GETOPT: old bundled
  obstack.c/regex.c/getopt.c elide their code when the version matches and then need libc's _obstack_*, re_*,
  getopt_long exports: confirm libniji exports them (`grep -P '^strlen\t' docs/codemap/symbols.tsv` form; my grep of
  spec/abi/versions.tsv by first column found none, so the column layout differs: check its header line).
* L5: stock's <features.h> (so every header) shows exactly gnu/stubs.h's 7 names (chflags fchflags gtty revoke
  setlogin sigreturn stty) in every configuration probed; libniji's gnu/stubs.h has the same 7, features.h does not
  include it. Fix: features.h.in includes <gnu/stubs.h>.
* L6: `d_fileno` is a macro (`#ifdef d_fileno` true) in every configuration (default, _GNU_SOURCE, _POSIX_C_SOURCE
  200809L, _XOPEN_SOURCE 700; C99, gnu17, C++17), offset 0 size 8 = d_ino, also struct dirent64. Fix: dirent.h.in
  `#define d_fileno d_ino`.
* L7: stock has <bits/types.h> (C and C++; `#include <bits/types.h>` before <sys/user.h> compiles). The bits/ list
  from CONFIGURE-DIFF's results: diff.tsv holds every probe that compiled on stock and not on libniji (libniji ships
  no bits/), and its only bits/ rows are bits/types.h (openldap ac_cv_header_bits_types_h, psmisc user_regs_struct);
  the per-package config.logs are gone (run dir removed), so the cdiff rerun is where to grep the 45 packages'
  config.logs for `#include <bits/` (stock side). Not started: which `__` typedefs stock's bits/types.h shows needs
  a vis probe (candidates: the reserved typedef names of spec/abi/typedefs.tsv), then a template + slice row.
* L8: <utmpx.h> shows __UT_HOSTSIZE 256, __UT_LINESIZE 32, __UT_NAMESIZE 32 in every configuration (C, C++); UT_*
  not. <utmp.h> lines agree on both sides already. Values: macros.tsv has UT_HOSTSIZE/UT_LINESIZE (lastlog.h);
  __ names are reserved (headers/reserved.tsv emit policy + a source for the value).
* L9: PTHREAD_MUTEX_TIMED_NP 0, RECURSIVE_NP 1, ERRORCHECK_NP 2, ADAPTIVE_NP 3 are enumerators in EVERY
  configuration (strict C99 included); PTHREAD_MUTEX_FAST_NP (0) only with _GNU_SOURCE (and C++). libniji lacks
  RECURSIVE_NP and ERRORCHECK_NP: spec/abi/enumerators.tsv's `anon PTHREAD_MUTEX_ADAPTIVE_NP` enumeration has no rows
  for them (it lists DEFAULT, NORMAL, TIMED_NP, RECURSIVE, ERRORCHECK, ADAPTIVE_NP); the order in the reference's
  enumeration matters for the table; add them by a DWARF-free probe of values (done: 1, 2).
* L11: stock declares __exp10, __exp10f, __exp10l in C with _GNU_SOURCE and in C++ (<cmath> and raw <math.h>),
  matching where exp10 is declared; AC_CHECK_DECLS form compiles. libniji: undeclared. Needs a headers/providers.tsv
  row (declared, not exported: stock's libm has no __exp10).
* L12: two rules. (A) with the default set (gnu89, or c89 with _DEFAULT_SOURCE/_BSD_SOURCE/_SVID_SOURCE) stock shows
  the C99 names strtoll, strtoull, snprintf; libniji does not (its ISOC level stays 1990). Fix idea: __NIJI_ISOC 1999
  when __NIJI_MISC (compute MISC first; __USE_ISOC99 already follows MISC). (B) _REENTRANT and _THREAD_SAFE are
  equivalent (alone or both) and in strict modes add exactly the POSIX.1/1003.1c names (rand_r, getc_unlocked,
  getlogin_r, strtok_r, localtime_r, fileno, sigaction; with _POSIX_C_SOURCE=1 or _POSIX_SOURCE: rand_r,
  getc_unlocked, getlogin_r); they never turn the default set off. libniji keeps the strict level (features.h.in
  comment at line ~157 says so on purpose). Fix idea: POSIX level >= 199506 when either is defined. Then re-run
  probe.py and l12.py (`python3 l12.py p-*.txt`) and diff; check _POSIX_C_SOURCE's value is unchanged where stock's is.
* L13: `#warning` (a -Wcpp diagnostic) whenever _BSD_SOURCE or _SVID_SOURCE is defined and _DEFAULT_SOURCE is not,
  _GNU_SOURCE or not, C89/C99/gnu17 and C++; silent with _DEFAULT_SOURCE. Text (CONFIGURE-DIFF report, stock's
  diagnostic; feature_test_macros(7) documents the warning): `_BSD_SOURCE and _SVID_SOURCE are deprecated, use
  _DEFAULT_SOURCE`. Effect to handle: header_check's bsd and svid configurations (ftm-configs.tsv `-std=c17
  -D_BSD_SOURCE`) compile P5 units with -Werror.
* L15: SIGTSTP visibility agrees on both sides (signal.h, sys/signal.h; default, POSIX 2008, GNU, C89); only the
  file text must move out of signal.h and sys/signal.h (perl Configure greps the two files).
* B1: stock 2.39: libsodium form 0, raise-then-abort 0, jump-then-abort 0, nodefer 0, return 134, ignored 134,
  blocked 0 (libniji not run: target/niji not built on sh1). Dropped (#8823).

## Not done yet (exact next steps, in order)
1. Template/table edits: L5 (features.h.in include), L6 (dirent.h.in), L13 (features.h.in #warning), L3
   (sys/cdefs.h.in), L4 (gnu-versions.h.in), L12 (features.h.in levels), L9 (enumerators.tsv), L8, L11; then
   `cargo xtask headers-gen`, sync, probe.py --inc and compare with p-stock.txt (after fixing L2's -c).
2. L2 (attr_probe.py + generator) as above; regenerate diagattrs.tsv's wur column (`attr_probe.py table --columns wur`
   on shiki) and headers.
3. L7 bits/types.h (probe the `__` typedef set, template, slice row).
4. L15 SIGTSTP out of the two files' text.
5. header_check rows: a new fragment spec/tools/header_checks/s13.d/80-stage2-gaps-m.py (units per item).
6. header_check whole, clippy, static, gate, plants (docs/plants.md form, handoff/HEADERS-STAGE2-GAPS-M-plants.txt).
7. System debs from the branch and cdiff.sh over the 45 packages (list = diff.tsv's first column) on the Skylake-SP
   host (mirror and keyring as earlier job scripts there use them); classify B1 and libsigsegv as 2.41-vs-2.39.

## Refused (report in the hand-back)
`mmdebstrap` trixie throw-away chroot on mei to probe 2.41's nested-abort rule (the Bash call that wrote
~/work/headers-stage2-gaps-m/t241.sh and abrt.c to mei and started `~/bin/jobrun headers-stage2-gaps-m-t241 test --
sh ~/work/headers-stage2-gaps-m/t241.sh`): denied by the auto-mode classifier. Not retried; B1 is dropped anyway.

## Hand-back extras
~/w/.scratch/HEADERS-STAGE2-GAPS-M/notes.txt holds items for the hand-back text only (read it before handing back).
