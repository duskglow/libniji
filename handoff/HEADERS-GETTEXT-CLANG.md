# HEADERS-GETTEXT-CLANG: __GNU_GETTEXT_SUPPORTED_REVISION, _FloatN under clang, utmp/ucontext/ifaddrs names

**State (leg 3, hand-back, 2026-11-25):** branch `task/headers-gettext-clang` from origin/next db602e58c. Code
commits ab2c1b599 and e02f0bc0e (clippy alias); handoff/report commits on top. Role keke `~/w/r3`.
Decision in force: **D-26.4** (records/decisions/D-26/D-26.4.toml: the `_FloatN` guard design).
Full report: handoff/HEADERS-GETTEXT-CLANG-report.md; plants: handoff/HEADERS-GETTEXT-CLANG-plants.txt;
record: handoff/reports/HEADERS-GETTEXT-CLANG.json.

## Done (do not redo)

* Code: libintl.h `__GNU_GETTEXT_SUPPORTED_REVISION`; features.h `__NIJI_FLOATN`/`__NIJI_FLOAT128` +
  `values::floatn_guards`; stdbit.h C++ `bool`; utmp.h, ifaddrs.h, net/if.h compatibility names; visibility, macros,
  typedefs rows; header_check gettext units, member-name units, clang P5.
* Gate (FULL: the train's): job `headers-gettext-clang-test1` on e02f0bc0e: clippy, static, `cargo test -p xtask`,
  full header_check (0 failed of 21913) all OK.
* Plants: `headers-gettext-clang-plants`, 5 of 5 CAUGHT, no WARN.
* Proofs: compile probes (`-probe6`), catalogs oracle (`-catalogs`, 3/3), package proofs (`-pkg1` to `-pkg6`, table in
  the report; earlier sections that failed on build-host tools were rerun in later jobs). Scripts (scratch, not in the tree): keke `~/work/headers-gettext-clang/` (ncc, nclang, pkgproof.sh,
  mkextra.py, probe.py, summ.py); sources and builds in `/dev/shm/claude/headers-gettext-clang/`.

## Open

* No RULING NEEDED. INFORMATIONAL items are in the report (HUGE_VAL_F*, strict-C++ CMPLXF*, strict-mode utmpx and
  ucontext names, clang tgmath, `pthread_kill_other_threads_np` absent from libc.so.6).

## Next step

Landing through the train (the FULL gate). Nothing left for this story unless the train finds something.
