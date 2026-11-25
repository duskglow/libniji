# HEADERS-STAGE2-GAPS-A: max_align_t clash (sys/un.h, sys/rseq.h), langinfo.h nl_item constants and _NL_ITEM macros

**State:** branch `task/headers-stage2-gaps-a` (from origin/next de74d9286), role keke:~/w/r1. Leg 1 ended in a
REFRESH; HEAD = the WIP commit "WIP: ..." (templates, tables, checks; `libc-include/` NOT yet regenerated).
Decisions: none new (every change follows black-box probes of the reference; no differences row).

## Done (leg 1)
* Cause of (1)(2): headers-gen emitted its own `typedef struct { long long ..; long double ..; } max_align_t;` in
  sys/un.h and sys/rseq.h; an anonymous structure is a type of its own, so it conflicts with the compiler's
  `<stddef.h>` in either order. Probe (keke, reference, gcc 13): sys/un.h exposes the compiler's whole <stddef.h>
  (ptrdiff_t, offsetof, max_align_t, size_t, NULL, wchar_t) in its misc configurations (gnu17, c99/c11 +
  _DEFAULT_SOURCE, c2x gnu, C++), nothing in c99/c11/c11+POSIX; sys/rseq.h and malloc.h in every configuration.
  Both orders with <stddef.h> compile on the reference.
* Fix (templates): `headers/sys/un.h.in` `@@owns max_align_t` + `@@when size_t` `# include <stddef.h>`;
  `headers/sys/rseq.h.in` `#include <stddef.h>` + `@@owns NULL max_align_t` (malloc.h's existing form). No
  dependence on any compiler guard macro: the type is the compiler's own definition.
* langinfo: `spec/tools/header_probe.py` `package_names()` gains every `spec/abi/langinfo.tsv` item name and
  `_NL_ITEM`, `_NL_ITEM_CATEGORY`, `_NL_ITEM_INDEX`. Probe run on keke (`--names`, all 243 headers, 40 configs):
  238 new langinfo.h rows spliced into visibility.tsv (235 enumerators `!time64only`, 3 macros), no existing row
  changed; `type_probe.py --headers langinfo.h`: 235 new macros.tsv rows (int), every value equal to
  (category << 16) | index of langinfo.tsv (cross-checked, 319 names, 0 mismatches). The 46 names of categories
  7-12 are enumerators only (not macros) in gnu17, c99, c11+POSIX, C++17.
* `_NL_ITEM` forms (probe): `(((c) << 16) | (i))` (usable in #if, type follows args: 1L -> long, 1u -> unsigned);
  `((int) (item) >> 16)` and `((int) (item) & 0xffff)` (not in #if, type int, 0xFFFFFFFFu -> -1 / 65535,
  0x100020005L -> 2 / 5, results and arguments parenthesised). Written in `headers/langinfo.h.in` (`@@owns-macro`,
  `@@if _NL_ITEM`).
* Checks: `spec/tools/header_check.py` P9 `compiler_order_units` (headers showing a compiler typedef, <stddef.h> +
  <stdarg.h> before and after, c89 c11 gnu2x cxx98 cxx11 cxx23); `spec/tools/header_checks/s10.py` `_NL_ITEM`
  form rows (c89 gnu17 cxx98 cxx17).
* Scratch (keke ~/work/headers-stage2-gaps-a/): probe_ref.py, ref.out, out1/, out2/, splice.py.

## Exact next step (leg 2)
1. `~/bin/jobwait headers-stage2-gaps-a-so0` on keke (product build of the base, started 05:12), then sync
   (rc 4 = BUSY: wait again, never force).
2. On keke: `cargo xtask headers-gen` (jobrun), copy back `libc-include/` changes (langinfo.h, sys/un.h,
   sys/rseq.h; expect no other header to change), commit "Regenerate libc-include".
3. header_check on the role (`python3 spec/tools/header_check.py`, jobrun kind test): 0 failed; P9 is the sweep
   for other compiler typedefs (size_t etc. are spelled as the compiler's own types; confirm 0 P9 failures).
4. nl_langinfo serving: `spec/tools/probe_langinfo.c` built against the reference, run for `C` and `en_US.UTF-8`
   with every langinfo.tsv CAT:INDEX:LEN, under the reference and under libniji (target/niji loader); every
   differing item is named in the hand-back (drop its rows rather than declare a constant that lies).
5. Probe with <stddef.h> before/after <sys/un.h> in C and C++ against libniji; perl-style probe of the 34 names.
6. Plants file handoff/HEADERS-STAGE2-GAPS-A-plants.txt (template mutants: un.h without the include, rseq.h
   without it, _NL_ITEM_CATEGORY without the int cast, _NL_ITEM unparenthesised), `cargo xtask plants`.
7. clippy, static, affected gate, precheck, report json, handoff-check. perl/exim4 source build: not in budget.

## Informational (for the report)
* `spec/tools/locale_inspect.py` `item_names()` parses the reference system's /usr/include/langinfo.h text (the
  langinfo.tsv names came from it); plan/headers.md 2.2 has probes read no header text. Not used by this story.
* No `_NL_NUM_*` category-count names were added: no spec table names them.
