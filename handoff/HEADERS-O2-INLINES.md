# HEADERS-O2-INLINES handoff (leg 1 of the run; WIP)

**State.** Branch `task/headers-o2-inlines` from origin/next e0110e274; role keke:~/w/n3. Leg 1 committed the template
edits UNREGENERATED and UNTESTED (WIP commit): `libc-include/` is not regenerated yet, nothing compiled.
Probe scripts (not in the tree): scratch `~/w/.scratch/HEADERS-O2-INLINES/` (local) and keke `~/work/headers-o2-inlines/`:
`p3.sh <tag>` (P3 for the 26 names, 5 headers), `o2probe.py ref|niji ~/w/n3 [names] [opts] [macros]` (undefined
symbols of one call per name across -O levels and macros), `cxxprobe.sh ref|refnostdcxx|niji` (C++ overload shapes),
`errnoprobe.py ~/w/n3 OUT "-O2;-Os;-O0"` (errno folding after every pure/const function, ref vs niji).

**Probe results (black box, objects only; before state on keke).**
* P3 before: 55 rows = 37 `call` rows (inline bodies) + 18 C++ `addr` rows (`!error` in the reference).
* Inline condition (all names alike): body when `__OPTIMIZE__` and not `__OPTIMIZE_SIZE__` / `__NO_INLINE__` (-O1, -O2,
  -O3, -Og inline; -O0, -Os, -fno-inline, -D__NO_INLINE__ call; C and C++ alike; -ffreestanding/-fno-builtin still
  inline). getline inlines to `__getdelim` only under _GNU_SOURCE (C++ defines it). atoi/atol -> strtol, or
  `__isoc23_strtol` under GNU/C2X (the strtol redirect). mbrlen with a run-time ps -> `mbrtowc` and `__mbrlen`
  (callsites.tsv shows only mbrtowc because its argument is a known non-null buffer). getc_unlocked family follows
  each name's visibility (no inline in strict ISO modes where the name is undeclared).
* C++: string.h basename memrchr rawmemchr strcasestr strchrnul and strings.h index rindex have const overloads in
  the reference C header (`&F` ambiguous; const in -> const out; const in -> mutable out is an error). abs/div `&F`
  ambiguity comes from libstdc++'s <stdlib.h> wrapper, NOT the C header: with `-nostdinc++` the reference compiles
  `&abs`/`&div`. So those 4 rows are a P3 probe artifact: callsite_probe.py `--include` drops libstdc++'s dirs.
* errno (owed #145): errnoprobe before: libniji folds the errno check where the reference keeps it for **269
  functions**: atof atoi atol atoll (at -O2; reference keeps at -O2 via its inline, folds at -Os too) and ~265 MATH
  functions (acos, asin, atan2, acosh, cosh, exp*, exp2, exp10, erfc, drem, log, sqrt, ... and the f/l/fN variants):
  diagattrs.tsv marks them `const`, libniji's math.h emits `__const__`, the reference does not fold. Cause: attr_probe.py
  infers pure from -Wunused-value on calls with LITERAL arguments (GCC folds the builtin) and then `conflict` makes it
  const. 363 others fold on both sides (fabs, strlen, ...): same as the reference, keep.

**Done in leg 1 (templates, WIP commit):** features.h.in `__NIJI_INLINES` / `__NIJI_INLINE_FN`; stdio.h.in inline
getc_unlocked getchar_unlocked putc_unlocked putchar_unlocked fgetc_unlocked fputc_unlocked feof_unlocked
ferror_unlocked getchar putchar getline (C.9 text); stdlib.h.in atof atoi atol atoll bsearch (bsearch probes in the
library's order: crates/core/src/search.rs binary_search, size 0 -> NULL); wchar.h.in mbrlen; string.h.in / strings.h.in
C++ const overloads (owned names; index/rindex under guard `__NIJI_D_index` in both headers); reserved.tsv emits
__uflow __overflow __getdelim __mbrlen.

**Next steps (exact).**
1. Sync, `cargo xtask headers-gen` on keke (in a jobrun job), fix generator errors (owned names now declared by the
   template: check `@@if strchrnul`/`memrchr`/`index` conditions come out right; noexcept of C++ overloads vs
   reference: probe `static_assert(noexcept(strchrnul((char*)0,0)))` both sides). Regenerate twice: byte-stable.
2. Rerun o2probe niji vs ref.txt and cxxprobe niji: equal. Rerun p3.sh after.
3. callsite_probe.py: in `--include` mode add libstdc++'s include dirs (from `g++ -xc++ -E -v /dev/null` search list,
   entries with `/c++/`) as `-isystem` BEFORE libniji's dir, for C++ compiles only (compile_unit, lang != c); leave
   plain_inc (cxx98 redeclaration check) alone. Removes the abs/div addr rows. Then P3 on all generated headers.
4. attr_probe.py: add an errno test (`errno = 0; r = F(opaque); if (errno) mk_e();`, at -O2) and veto pure/const when
   the reference keeps mk_e; regenerate diagattrs.tsv on keke (reference = Ubuntu 24.04 glibc 2.39 there), then
   headers-gen; errnoprobe after: 0 "libniji folds where the reference keeps". attr_probe `check` must stay 0 diffs
   (if -Wunused-value now differs for atof etc., it was the literal-argument artifact: explain, do not loosen).
5. Tests in a new module `xtask/src/headers/slices/o2inlines.rs` (SLICE with no markers; the s18.rs
   `an_open_without_its_mode_calls_the_checked_entry` pattern): per-name call shape at -O2 / -O0 / -Os, C++ overload
   shape, errno kept after atof and acos at -O2/-Os. Clause names in comments.
6. Plants in docs/plants.md form (file e.g. validation/headers_o2inlines.plants): getchar's inline reverted, `pure`
   back on atof (diagattrs.tsv row). Conformance s08_strto_fenv_flags with `--niji-headers` (61 atof rows); P7 zlib +
   xz (Devuan flags, see handoff/SL-HEADERS-S18.md "Next step"). Gate `cargo xtask affected --run`, plants, precheck.
7. Decision row (reserve an id: `ops reserve --story HEADERS-O2-INLINES D-X`) for the inline design + errno veto.
