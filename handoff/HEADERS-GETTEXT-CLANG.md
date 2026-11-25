# HEADERS-GETTEXT-CLANG: __GNU_GETTEXT_SUPPORTED_REVISION, _FloatN under clang, utmp/ucontext/ifaddrs names

**State (leg 2 ended in a REFRESH, 2026-11-25):** branch `task/headers-gettext-clang` from origin/next db602e58c,
code commit ab2c1b599 (pushed), role keke:~/w/r3 (synced to ab2c1b599). Scratch (local):
`~/w/.scratch/HEADERS-GETTEXT-CLANG/` (probe.py, mksnip*.py, summ.py, splice.py, insrows.py). On keke:
`~/work/headers-gettext-clang/` (probe.py, summ.py, snip2/, snip3/, snip5/; probe logs `~/logs/headers-gettext-clang-probe*.log`).
Decision in force: **D-26.4** (records/decisions/D-26/D-26.4.toml, the _FloatN guard design).

## Done (do not redo)
* Leg 1: black-box probes (results summarised in D-26.4 and the commit; details in probe2/probe3 logs on keke).
* features.h.in: `__NIJI_FLOATN`, `__NIJI_FLOAT128`, typedefs of `_Float32/64/32x/64x` where no keyword.
  `values::floatn_guards` (xtask/src/headers/values.rs) post-pass in `generate()`; unit test
  `floatn_lines_are_guarded_by_compiler_support` (xtask/src/headers.rs).
* Templates: libintl.h `__GNU_GETTEXT_SUPPORTED_REVISION(major) ((major) == 0 ? 1 : -1)` (+ reserved.tsv emit row);
  utmp.h ut_name/ut_time/ut_xtime/ut_addr; ifaddrs.h and net/if.h (struct ifaddr, MISC) ifa_broadaddr/ifa_dstaddr;
  stdbit.h `bool` typedef in C++ (clang -std=c++17 had no `_Bool`).
* Tables: visibility.tsv +30 rows (header_probe.py `--names`, package_names() extended), macros.tsv UTMP_FILENAME
  WTMP_FILENAME, typedefs.tsv greg_t (type_probe.py on keke).
* header_check: s10.py `gettext_units` (form/value/type, `#if` use, not-shown), s12.py MEMBER_MACROS += utmp.h,
  ifaddrs.h, net/if.h ifa_*; header_check.py `clang_p5` (clang-19: c17, gnu2x, c17+_GNU_SOURCE, c++17, gnu++20;
  resource dir before libc dir, as the driver; skip printed when no clang).
* Runs: `headers-gettext-clang-hc1` (full header_check on the regenerated tree: 34 of 21913 failed = 33 test-side
  `0.0` gettext form, removed, + stdbit.h clang-c++17, fixed); `headers-gettext-clang-hc2` (--only the changed units
  incl. all clang P5: 0 failed of 1899). Both before the final commit's last test edits: SUPERSEDED for the hand-back.
* Gettext revision backing: crates/core/tests/oracle/catalogs_diff.c already compares ngettext/dngettext/dcngettext
  plural forms and a msgfmt revision-1 `<PRIu64>` catalog with the reference (catalogs_run.sh). Still to RUN it.

## Exact next steps
1. Read `~/logs/headers-gettext-clang-lint1.log` on keke (started at end of leg 2 on ab2c1b599): lines
   HEADERS_GEN_CHECK= CLIPPY= STATIC= XTASK_TEST= AFFECTED=; affected list in `~/work/headers-gettext-clang/affected.txt`
   (expect FULL escalation: generated headers -> gate = precheck + header_check job + `cargo test -p xtask`, kind test,
   and "FULL: the train's" line). Fix anything red.
2. Proof per item, compile probes: rerun `python3 probe.py ~/w/r3/libc-include snip3` and `snip2` on keke, then
   `python3 summ.py LOG ""` (`*` = differs from reference); expected remaining `*`: HUGE_VAL_F* (INFORMATIONAL),
   `__HAVE_FLOAT*` (reserved, not defined), strict C++ CMPLXF* (INFORMATIONAL).
3. Package proofs (blocking), sources ONLY from the local mirror (coordinator note 15:51Z: never the internet):
   `/srv/mirrors/debian/pool/main/<x>/<pkg>/*.dsc` on keke, `dpkg-source -x` into `/dev/shm/claude/headers-gettext-clang/src`
   (tar_1.35+dfsg-3.1, postgresql-17_17.12-0+deb13u1, sysvinit_3.14-4, ruby3.3_3.3.8-2, python-psutil_7.0.0-2,
   parted_3.6-5+deb13u1, lvm2_2.03.31-2, openldap_2.6.10+dfsg-1); compile the failing files with `-nostdinc -isystem
   <cc include> -isystem ~/w/r3/libc-include -isystem <uapi>`; gettext.m4 (keke /usr/share/aclocal/gettext.m4 lines
   136-171) test programs; tar configure + `make install` with the system flavour (target/niji after `cargo xtask so`,
   or the stage2 chroot tarball under `unshare -r`): `checking for GNU gettext in libc... yes` and .mo installed.
   Run catalogs oracle (find its test target: `grep -rn catalogs_run crates/core/tests/*.rs`).
4. Plants in docs/plants.md (`cargo xtask plants`, RAM dir /dev/shm/claude/headers-gettext-clang/plants-1): e.g.
   values.rs `Some(true) =>` guard dropped (xtask test msg), libintl.h.in `== 0 ? 1 : -1` -> `>= 0 ? 1 : -1` (header_check
   --only GETTEXT), utmp.h.in `ut_addr ut_addr_v6[0]` -> `[1]` (--only "member names utmp"), features.h.in
   `__NIJI_FLOAT128 1` condition inverted (--only "clang"), stdbit.h.in C++ branch removed (--only "stdbit.h clang").
5. Report: handoff/HEADERS-GETTEXT-CLANG-report.md, handoff/reports/HEADERS-GETTEXT-CLANG.json (with "jobs"),
   precheck --jobs, handback-check.

## INFORMATIONAL (for the report)
* libniji lacks HUGE_VAL_F32/F64/F32X/F64X/F128 (reference: GNU C and C++; not in visibility.tsv at all).
* Strict C++ (-std=c++17, g++): libniji defines CMPLXF32/F64X/F128, the reference not.
* Strict modes: reference names utmpx `__e_termination`/`__e_exit` and ucontext `__gregs`, `__fpregs`, `__uc_flags`;
  libniji the unprefixed names in every mode.
* clang's own <tgmath.h> precedes the libc's in the driver's search order; libniji's tgmath.h needs `__builtin_tgmath`.
