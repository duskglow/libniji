# NL-LOCALE-NAME-ABI(-2): _NL_LOCALE_NAME in <langinfo.h>, static_assert in <assert.h>, _STDIO_H, layout-macro sweep

**State:** branch `task/nl-locale-name-abi-2` (continues `task/nl-locale-name-abi`, from origin/next ecc8c95e3), role
keke:~/w/r4. Head: see `handoff/reports/NL-LOCALE-NAME-ABI-2.json`. Decisions: none new. `_STDIO_H` is an R-H2 row of
`headers/reserved.tsv` (owner ruling R-H2: reserved names other software tests, same meaning); the rest follows
reference probes and ISO C 7.2. No differences row, no compat option.

## Commits
* Run 1: `93027447b` _NL_LOCALE_NAME (template, visibility.tsv row, header_probe candidate, s10 form rows,
  sym_nl_langinfo(_l) cases); `3770f6ab5` static_assert; `e499daade` s10 static_assert unit per configuration;
  plants `handoff/NL-LOCALE-NAME-ABI-plants.txt` (6 CAUGHT).
* Run 2: `f55e4b249` emit `_STDIO_H` (reserved.tsv row; generated stdio.h only: argp.h, malloc.h, resolv.h, stdio_ext.h
  show it by including stdio.h) and header_check **P1G** units: every include guard reserved.tsv emits (`_NET_IF_H`,
  `_NETINET_IN_H`, `_SYS_XATTR_H`, `_STDIO_H`), in every p1 header visibility.tsv shows it in, defined with its
  macros.tsv value where shown and undefined where not (P1 skipped all guards: they were tested nowhere). Plants
  `handoff/NL-LOCALE-NAME-ABI-2-plants.txt` (3; results at its foot); release note
  `docs/release-notes.d/NL-LOCALE-NAME-ABI.md`.
* Run 2 jobs (keke): lint0 (inherited head clean), p1g1 (P1G 0 failed of 507), test3 (full header_check and
  `cargo test -p xtask`), test2 (`cargo xtask so` + s07_locale_symbols 43 passed), plants3 (3 CAUGHT), lint and
  precheck on the final head. Superseded: test1 (header_check 0 failed of 20622 and xtask green, but its s07 part
  failed only on "the products are missing": no `cargo xtask so` in the role yet; split into test2 and test3);
  plants (the policy-row plant survived, see the plants file); plants2 (field typo in the plants file).

## Probe-settled behaviour (keke, black-box, both libraries)
* `_NL_LOCALE_NAME(c)` = `_NL_ITEM(c, 0xffff)`; type follows the argument; not usable in `#if`; langinfo.h in every
  configuration but time64only. nl_langinfo(_l) of it: setlocale(c, NULL)'s name, "POSIX" read back as "C", alias
  spellings kept, "" for LC_ALL; libniji already served it (C, POSIX, en_US.UTF-8, en_US.utf8, environment, mixed
  newlocale, duplocale, uselocale, categories 0..12); "" for -1, 13, 100; LC_GLOBAL_LOCALE = the global locale.
* `static_assert`: assert.h, every C configuration except c89, c99, isoc99, c2x, gnu2x; not C++.
* `_STDIO_H`: value 1 (macros.tsv), stdio.h and its includers, every configuration but time64only; complex.h and
  tgmath.h show it in the reference's C++ modes too; libniji's own complex.h does not include stdio.h, and whether
  the C++ library's <complex.h> reaches stdio.h there is NOT VERIFIED (P1G covers p1 headers; both are p1 0).

## Proof (perl, run 1)
* trixie perl 5.40.1 headers: reference HS_KEY 0xeb80487, sizeof(PerlInterpreter) 3768; libniji before 0xf280487,
  3880 (Yohane's "got 0xf280080"); after 0xeb80487, 3768. A libniji-built XS module loads into trixie's perl on
  libniji (job `nl-locale-name-abi-xsload3`).

## Sweep (run 1; names tested in #if lines of perl 5.40, python3.13, openssl 3.5, glib/gio 2.84 headers)
* Layout-changing: only `_NL_LOCALE_NAME`. Fixed. `static_assert`, `_STDIO_H`: fixed.
* Reference-only, left (no layout change; who tests them, run 2): `va_copy`/`__va_copy` in printf.h (perl handy.h
  tests `va_copy` after including <stdarg.h> itself); `_IOC_SIZE` in sys/soundcard.h (perl.h, with `__GLIBC__`);
  `CHARBITS` in values.h (perl's utf8.h uses its own); `_PATH_RESCONF` in resolv.h (gio's gnetworking.h defines the
  same "/etc/resolv.conf" when absent); S_IS* in ftw.h; `SPACE` in scsi/scsi.h.
* libniji-only: `__GLIBC__`, `__THROW`, feature macros in 8 headers including features.h; CMSG_LEN in 5 net headers;
  S_ISSOCK in sys/stat.h under c11. Include differences: a.out.h, stab.h, sys/vm86.h.

## Open
* gnulib localename / liblocale-gettext-perl stage-2 rebuild: NOT VERIFIED. Run 1's source fetch was refused; run 2's
  first step on mei (listing `~/work/RELEASE-SNAPSHOT-1125B/`, the recipe the coordinator named) was refused by the
  auto-mode classifier, so the step stopped (hand-back REFUSED line). Needs the coordinator to grant or run it.
* Next step: the train (FULL: generated tables); then the stage-2 rebuild above once permitted.
