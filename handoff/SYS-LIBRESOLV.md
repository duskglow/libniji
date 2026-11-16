# SYS-LIBRESOLV (issue #5): libresolv.so.2 in the system packages

State: branch `task/sys-libresolv-3`, code head 66e010524 (SYS-LIBRESOLV-3; later commits only touch
handoff/). Build role keke ~/w/n2; job scripts in keke ~/work/SYS-LIBRESOLV-3 (deb.sh) and
~/work/SYS-LIBRESOLV-2 (lintbase/sys = origin/next 28740dd5e's system packages, used as the swap base).

## Decisions in force

* D-16.6 (ruling #6096): libresolv.a stays empty, libc.a defines every libresolv function;
  check-system.sh proves a static `-lresolv` link runs (green on the final head).
* No new decision record: every behaviour change matches the reference (probe-settled by
  tests/ns_parse_products.rs, which now also compares the ns_msg handle's section, record and
  parse position after every call).

## Done (do not redo)

* Run 1: system.sh ships libresolv.so.2 (libc6) and libresolv.so (libc6-dev); check-system.sh,
  check-symbols.sh, smoke/resolv.c, smoke/sym-resolv.c; differential probe
  `crates/core/tests/oracle/ns_parse_diff.c` + `tests/ns_parse_products.rs`.
* Run 2: ns_parserr rrnum -1 / handle moves, EMSGSIZE for every name-function failure,
  ns_name_uncompress root "."; root-system.sh resolv + krb5 in the empty root; deb_symbols_products
  guard for libc6's libresolv.so.2 block.
* Run 3 (this):
  - Lintian (#6116): `undeclared-elf-prerequisites (libc.so.6)` is raised for EVERY library in
    libc6 with DT_NEEDED libc.so.6 (8 on next: libm, libpthread, libdl, librt, libutil, libanl,
    libnsl, libc_malloc_debug; libresolv.so.2 made 9). lintian 2.122.0's check
    (Binaries/Prerequisites.pm) fires when a file has NEEDED entries and the package's strong
    Depends is empty; libc6 cannot depend on itself. libresolv.so.2 already had NEEDED libc.so.6,
    as the reference's does (readelf -d). Fix: tools/deb/system/libc6.lintian-overrides installed
    as usr/share/lintian/overrides/libc6, covering exactly the `(libc.so.6)` context under
    usr/lib/x86_64-linux-gnu (a library needing anything more still raises it); check-system.sh
    allows the path. Result 7 errors / 42 warnings (base 7/50).
  - Plants file handoff/SYS-LIBRESOLV-3-plants.txt (9 plants; replaces -2's, whose two parserr
    sites no longer exist). plants-1 showed two survivors, both now covered:
    the handle's parse position after a section move (probe now prints `handle=s# #n @off`) and
    rr_at's RDATA bound (masked by ns_initparse in the probe; s16_4_c_3 asserts it directly).
  - deb.sh: the swap step compares the branch's swap output with the base packages' (same
    RESULT and same FAIL lines after removing versions) instead of failing on the known ldd-1..7.

## Results (keke)

* lint0 (inherited head): rc 0. lint1 (e9c491678: workspace clippy -D warnings, xtask clippy,
  static): rc 0.
* gate (e9c491678): 6 suites, 1494 passed / 0 failed / 10 ignored, peak 1.7G.
* deb (e9c491678): build/check/check-symbols/root-system (18 pass / 0 fail, includes resolv and
  krb5) / swap same=54 differ=7 on branch AND base with identical FAIL lines / lint 7/42 /
  coexist build + check: all rc 0.
* ns5 (66e010524): ns_parse_products (handle state included) and s16_4_c_3 pass.
* Final head 66e010524 (all rc 0): gate2 6 suites 1494 passed / 0 failed / 10 ignored (its
  clippy covered the changed crates), peak 1.3G; plants2 9 CAUGHT / 0 SURVIVED / 0 WARN
  (results in handoff/SYS-LIBRESOLV-3-plants.txt); deb2 every step rc 0 (check-system with the
  static -lresolv program, root-system 18/0 with dynamic -lresolv and krb5, swap 54/7 identical
  to base, lint 7/42, coexist build + check); lint2 (workspace clippy -D warnings, xtask clippy,
  static) rc 0.
* Superseded: plants-1 (2 survivors, stopped after 8 plants), gate/deb/lint1 (head e9c491678,
  before the test additions).

## Next step

None for issue #5 once the final-head jobs are green. Optional: see open questions.

## Open questions

* INFORMATIONAL: the swap's ldd-1..7 differences (/usr/lib vs /lib paths, -v version lines) are
  identical on origin/next's packages: loader trace output, DL-TRACE-INTERP's.
* INFORMATIONAL: the system flavour lacks libBrokenLocale.so.1/.so/.a, libg.a, libmcheck.a,
  libpthread_nonshared.a, which the coexist flavour ships (not this story).
