# SL-HEADERS-S20 report (runs 1 to 4)

**Run 4 (SL-HEADERS-S20-4).** Commits: f05fd0ae1 (merge origin/next bf40b1b1c; one conflict, docs/install-packages.md:
next's libc-bin row kept, this branch's libc6-dev row kept), f811cf2d1 (`__NIJI_SIMD` defined in features.h beside
`__NIJI_NOTHROW`; libc-include regenerated: features.h, math.h), 6e5d31009 (plant file). Step 0 on the inherited
head 85021878b: clippy and static green. On the merged and fixed head: clippy and static green; rows test by name
green (job sl-headers-s20-4-rows: 1 passed in 9.6 s; the same test on the plant's failing units took 1563 s on
keke, so the CI's 1150 s was the failure itself, not the test's normal cost); sl-headers-s20-4-t1 green
(`cargo xtask so`; niji_cc_headers 5 passed incl. the simd comparison; deb_manifest
`coexist_lm_brings_in_libmvec_only_for_a_vector_call` passed). Plants: sl-headers-s20-4-plants1 (1 plant,
s20-4-simd-everywhere: the definition reachable only when math.h includes features.h): 1 CAUGHT by its message
(rows test FAILED, "storage class specified for parameter"), 0 warnings; results in handoff/SL-HEADERS-S20-4-plants.txt.
IMPROVEMENT: the rows test is ~10 s green but ~25 min red, and the run-3 test job never ran it although the change
reached its emitter; a header-generator change should run it by name (it is cheap when green).

**Commits (run 2).** 6228e4df5 (`__locale_t`, C++ overload macros), 287819e37 (unshipped-header list, P4 and
callsite_probe), b969dc875 (-nostdinc driver test, P4 plants), d22bb006b (cxx98 probe flags), 4b30fac10 (__locale_t test); run 1's commits before them.

**P4 C++ failures, by cause (header x -std pairs where the reference compiles and libniji's headers did not).**

| cause | before (run 1) | after |
|---|---|---|
| `__locale_t` undeclared (the C++ library's c++locale.h: every `<iostream>`, `<locale>`, `<sstream>` user) and `<cwchar>`/`<cstring>` const-overload collisions (wcschr, wcspbrk, wcsrchr, wcsstr, wmemchr; strchr ...) | 214 (leg 1 fixed both causes in one commit and did not split the count) | 0 |
| C++98/C++03 `<complex.h>`, `<tgmath.h>`: the C++ library's `#include_next` of a C header libniji does not ship | 4 | 0 FAIL, 4 "known" (validation/headers_unshipped.tsv) |

**P4 part 2.** callsite_probe `--include` failed on aio.h c89 (the host's aio.h through the /usr/include fallback);
fixed as in the handoff. Its cxx98 column read "unknown" everywhere (the -isystem flag bug, fixed in d22bb006b); result below.


**P4 part 2 on the final head (job sl-headers-s20-2-t5; ruling on #8420: a tracked follow-up, not this story).** C++ part:
c++98/03 104 compiled, 39 n/a, 2 known, 0 FAIL; c++11 134/11/0/0; c++14 and c++17 133/12/0/0; c++20 and c++23 134/11/0/0.
noexcept.tsv and callsites.tsv still differ, by header (`dunder` = a `__`-prefixed name libniji's header does not
declare; stdbit.h: its functions lack the exception specification the reference gives them):

| header | noexcept_rows | dunder/other | callsite_rows(spec-only/mine-only) | first noexcept rows (spec -> mine) |
|---|---|---|---|---|
| math.h | 750 | 716/34 | 720/718 | __acos noexcept/no/throw() -> error/-/error; __acosf noexcept/no/throw() -> error/-/error; __acosf128 noexcept/no/throw() -> error/-/error |
| stdbit.h | 70 | 0/70 | 0/0 | stdc_bit_ceil_uc noexcept/no/throw() -> none/no/none; stdc_bit_ceil_ui noexcept/no/throw() -> none/no/none; stdc_bit_ceil_ul noexcept/no/throw() -> none/no/none |
| resolv.h | 33 | 33/0 | 33/33 | __b64_ntop noexcept/no/throw() -> error/-/error; __b64_pton noexcept/no/throw() -> error/-/error; __dn_count_labels noexcept/no/throw() -> error/-/error |
| argp.h | 11 | 9/2 | 14/14 | __argp_error none/yes/none -> error/-/error; __argp_failure none/yes/none -> error/-/error; __argp_help none/no/none -> error/-/error |
| stdio_ext.h | 9 | 9/0 | 9/9 | __fbufsize noexcept/no/throw() -> error/-/error; __flbf noexcept/no/throw() -> error/-/error; __fpending noexcept/no/throw() -> error/-/error |
| ctype.h | 2 | 2/0 | 14/14 | __tolower_l noexcept/no/throw() -> error/-/error; __toupper_l noexcept/no/throw() -> error/-/error |
| argz.h | 5 | 5/0 | 7/7 | __argz_count noexcept/no/throw() -> error/-/error; __argz_create noexcept/no/throw() -> error/-/error; __argz_extract noexcept/no/throw() -> error/-/error |
| error.h | 4 | 4/0 | 8/8 | __error_alias none/yes/none -> error/-/error; __error_at_line_alias none/yes/none -> error/-/error; __error_at_line_noreturn none/yes/none -> error/-/error |
| string.h | 5 | 5/0 | 5/5 | __memcmpeq noexcept/no/throw() -> error/-/error; __mempcpy noexcept/no/throw() -> error/-/error; __stpcpy noexcept/no/throw() -> error/-/error |
| libintl.h | 2 | 2/0 | 6/6 | __dcgettext noexcept/no/throw() -> error/-/error; __dgettext noexcept/no/throw() -> error/-/error |
| sys/pidfd.h | 0 | 0/0 | 8/4 |  |
| stdio.h | 1 | 1/0 | 3/2 | __asprintf noexcept/yes/throw() -> error/-/error |
| signal.h | 1 | 1/0 | 2/2 | __sysv_signal noexcept/no/throw() -> error/-/error |
| sys/shm.h | 1 | 1/0 | 1/1 | __getpagesize noexcept/no/throw() -> error/-/error |
| unistd.h | 1 | 1/0 | 1/1 | __getpgid noexcept/no/throw() -> error/-/error |
| sys/gmon.h | 1 | 1/0 | 1/1 | __monstartup noexcept/no/throw() -> error/-/error |
| nss.h | 1 | 1/0 | 1/1 | __nss_configure_lookup noexcept/no/throw() -> error/-/error |
| libgen.h | 1 | 1/0 | 1/1 | __xpg_basename noexcept/no/throw() -> error/-/error |
| alloca.h | 1 | 0/1 | 1/1 | alloca noexcept/no/throw() -> error/-/error |
| netinet/in.h | 1 | 0/1 | 1/1 | bindresvport6 noexcept/no/throw() -> error/-/error |
| sys/stat.h | 1 | 0/1 | 1/1 | getumask noexcept/no/throw() -> error/-/error |
| stdlib.h | 1 | 0/1 | 0/0 | div error/-/error -> noexcept/no/error |
| sys/wait.h | 0 | 0/0 | 1/1 |  |
| netrose/rose.h | 0 | 0/0 | 1/1 |  |

**Unshipped headers (known limitation, ruling on #8419).** aio.h, complex.h, mqueue.h, proc_service.h, tgmath.h,
thread_db.h, Fortran math-vector-fortran.h. In the coexist flavour `NIJI_SYSTEM_HEADERS=1` builds such a package; the
system flavour (libniji's headers are `/usr/include`) has none. Before slice 20 it shipped no C headers at all.

**Plants.** Leg 1 run (head 6228e4df5, killed after its result was known): s20-cc-nostdinc SURVIVED (test fixed in
b969dc875); s20-cc-cxxdirs, -optout, -uapi CAUGHT before the kill. Run 2 (plants2, head b969dc875): 10 CAUGHT, s20-cxx-locale_t SURVIVED (ten other headers declare __locale_t; test fixed in 4b30fac10). Final run: sl-headers-s20-2-plants3 on 4b30fac10.

**INFORMATIONAL.** docs/RELEASE-NOTES.md's released sections say "The system flavour ships no C headers": true for
those snapshots; the new note (SL-HEADERS-S20.md) says what changes.

**Leftovers.** P6, P7 and a .deb with the new rows (brief: next stories); the 7 unshipped headers.
