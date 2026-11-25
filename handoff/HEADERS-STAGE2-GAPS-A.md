# HEADERS-STAGE2-GAPS-A: max_align_t clash (sys/un.h, sys/rseq.h), langinfo.h nl_item constants and _NL_ITEM macros

**State:** branch `task/headers-stage2-gaps-a` (from origin/next de74d9286), role keke:~/w/r1. Two legs (leg 1 ended in a
REFRESH). Full report: this page; json: `handoff/reports/HEADERS-STAGE2-GAPS-A.json`.
Decisions: none new. Every change follows black-box probes of the reference; no differences row, no compat option.

## Commits (all mine)
* `2c0104702` templates, tables and checks (sys/un.h, sys/rseq.h, langinfo.h; visibility.tsv +238, macros.tsv +235;
  header_probe.py names; header_check.py P9; header_checks/s10.py `_NL_ITEM` form rows).
* `bb1407022` Regenerate libc-include (`cargo xtask headers-gen` changed exactly langinfo.h, sys/un.h, sys/rseq.h).
* `42eb8de7c` plants file; `3a4803cdb` `header_check.py --only REGEX` (plant commands; see below); then this report.

## Cause and fix
* (1)(2) exim4/openssl: headers-gen emitted its own anonymous-structure `max_align_t` in sys/un.h and sys/rseq.h. An
  anonymous structure is a type of its own, so it conflicts with the compiler's `<stddef.h>` in either order, whatever
  guard macros the compiler uses. Probe of the reference (gcc 13): sys/un.h exposes the compiler's whole `<stddef.h>` in
  its misc configurations (gnu17, c99/c11 + _DEFAULT_SOURCE, c2x gnu, C++), sys/rseq.h and malloc.h in all of them.
  Fix: both templates include the compiler's `<stddef.h>` there and `@@owns max_align_t`, so the one type is the
  compiler's own definition. No dependence on gcc 13's or gcc 14's guard macros.
* (3)(4) perl/gnutls: every spec/abi/langinfo.tsv item name is now probed and emitted. Values are equal to
  (category << 16) | index for all 319 names. `_NL_ITEM`, `_NL_ITEM_CATEGORY` and `_NL_ITEM_INDEX` are in the reference's
  probed form (comment in headers/langinfo.h.in).

## Proof (keke, Zen 1 host; all on the final code head 3a4803cdb unless noted)
* header_check full: `headers-stage2-gaps-a-hc2`: 0 failed of 19083 (also 0 of 19083 on bb1407022, `headers-stage2-gaps-a-hc`). No form note on
  any new item. P9 is the sweep for every other compiler typedef (max_align_t, size_t, ptrdiff_t, wchar_t, wint_t,
  nullptr_t, va_list): `<stddef.h>` + `<stdarg.h>` before and after each header showing one, in c89 c11 gnu2x cxx98 cxx11
  cxx23. 0 failures, so no other header needed a fix.
* Include order, by hand (scratch probe, both libraries): `<stddef.h>` before and after `<sys/un.h>` + `<sys/rseq.h>`,
  using max_align_t, size_t and ptrdiff_t. In gnu17, c11, gnu2x, gnu++17 and c++23 it compiles on both sides. In c99 and
  c++98, max_align_t is undeclared on both sides (the reference does not show it there either).
* nl_langinfo serving: a probe that names all 277 public langinfo.tsv items by their header names (this covers perl's 36
  undeclared names and every new item) calls `setlocale(LC_ALL, L)` + `nl_langinfo`. It was built against each library's
  headers and run under each library (libniji through its ld.so). en_US.UTF-8: 277 of 277 equal. C.UTF-8: 277 of 277
  equal (item bytes). C: equal apart from already-recorded differences: the five LC_IDENTIFICATION contact fields
  (D-7.10, D-7.22, differences row "LC_IDENTIFICATION of the C locale") and `_NL_CTYPE_TRANSLIT_IGNORE` ("" where the
  reference returns NULL; differences row "nl_langinfo of the ctype and collation table items", D-7.10). The
  reference-compiled binary run under libniji gives the same lines, so the header constants are identical; what differs
  is libc serving, and it is recorded. No item was dropped.
* xtask tests (`cargo test -p xtask`, the readers of the changed tables): `headers-stage2-gaps-a-xtask2`: 6 suites, 0 failed. Lint: `headers-stage2-gaps-a-lint2` (clippy -D warnings + static).
* Plants: `headers-stage2-gaps-a-plants2` (6 plants: un.h and rseq.h spelling max_align_t themselves, the three item
  macros without their parentheses or int cast, the `--only` empty-selection guard): 6 CAUGHT, 0 WARN.
* Gate: `cargo xtask affected` says `FULL: the train's` (generated tables and their inputs), so per the brief: precheck +
  the readers' tests (xtask) + header_check. Superseded: `headers-stage2-gaps-a-plants` (its baseline timed out: the
  full header_check is about 520 s of serial compiles, over the 600 s limit beside a second copy; hence `--only`).
* perl/exim4 source build: not run (did not fit this budget; the dpkg-buildpackage of perl is the finish line's own).

## Informational
* The 41 `__`-prefixed enumerator names of langinfo.tsv (`__DECIMAL_POINT`, `__ALTMON_*`, ...) are not in libniji's
  langinfo.h, though 37 of them gained table rows here. headers-gen leaves them out by R-H2/R-H3 (reserved identifiers, `headers/reserved.tsv` not "emit").
  perl and gnutls need none of them. The public aliases (RADIXCHAR, ALTMON_*, ...) are emitted.
* langinfo.h items are `#define`d int constants. The reference's are also enumerators, so C++ sees type `enum` there
  (header_check form report: 95 `type differs` notes on langinfo.h, all on the older items such as ABDAY_*, none on new
  ones). This is report mode and existed before this story.
* `spec/tools/locale_inspect.py` `item_names()` parses the reference system's langinfo.h text (the langinfo.tsv names
  came from it); plan/headers.md 2.2 has probes read no header text. Not used by this story.
* No `_NL_NUM_*` category-count names were added: no spec table names them.

## Exact next step
None for this story: it lands through the train. A follow-up worth having is header_check running its units in
parallel (about 520 s serial; the plants needed `--only` because of it).
