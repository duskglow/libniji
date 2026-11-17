# SL-HEADERS-S06: slice 6, minimal C program (handoff after run 1, a checkpoint hand-back)

**State.** Branch `task/sl-headers-s06`, cut from origin/next 06c0b561e. INCOMPLETE: slice 6 is started, not finished. The
branch must not land as it stands: `headers-gen` still reports 20 problems, so no `libc-include/` file exists for the
eleven new templates and `cargo xtask static` (the `headers-gen --check` step) is red for them. Head: see
`handoff/reports/SL-HEADERS-S06.json`. Say "slice 6" in commits and handoffs.

**Done (run 1).**
* Templates `headers/{assert,ctype,stdio,stdlib,string,strings,memory,unistd,getopt,time}.h.in` and
  `headers/sys/unistd.h.in` (`memory.h` is the R-H7 "ship" row of plan/headers.md section 5.1; `memory.h` and
  `sys/unistd.h` include `string.h` and `unistd.h`). They carry the Appendix C.9 protocols: `assert` with
  `__assert_fail` (C23 variadic form, `NDEBUG`, no include guard), the ctype masks (`__isctype`, `(*__ctype_b_loc ())[c]`,
  `tolower`/`toupper` through `__ctype_tolower_loc`/`__ctype_toupper_loc`, only in the configurations whose rows say
  they are macros), `MB_CUR_MAX` as `__ctype_get_mb_cur_max ()`, `stdin`/`stdout`/`stderr` as `FILE *` objects with
  `#define stdin stdin`, `NULL`, the wait-status macros, `FD_*` (same text as `sys/types.h`), `TEMP_FAILURE_RETRY`,
  `strdupa`/`strndupa`, and the [cstring.syn] overloads of `strchr strrchr strpbrk strstr memchr` in C++.
* Generator (`xtask/src/headers.rs`):
  * `@@owns-macro A B`: the template defines these macros; their function rows are still emitted (ctype's `isalpha`
    is both a function and a macro).
  * the `enumerator` kind: `enum { NAME = value };` once per translation unit (guard `__NIJI_D_enum_NAME`), the value from
    `macros.tsv` (the macro row of the same name gives the `#define`). Needed by `_SC_*`, `_CS_*`, `_PC_*` and `_IS*`.
  * structure members that are complete structures (their definition is emitted first) and pointer members with a
    typedef-named target (`int32_t *`).
  * a call-site row whose `&f` probe did not compile (`!error`) is ignored; `LINKED_ELSEWHERE` lists the four declared
    names no default-version export provides (`atexit`, `at_quick_exit`, `pthread_atfork` from the static archive
    linked into every program; `crypt` from the crypt library).
  * `_FORTIFY_SOURCE` configurations stop deciding a name's condition when no condition over the levels separates it
    there (they are slice 18's; the `addr` probes also run in none), and they are outside the care set of the `addr`
    symbol conditions.
* `headers/reserved.tsv`: `__assert_fail`, `__assert_perror_fail`, `__assert`, `__ctype_b_loc`, `__ctype_tolower_loc`,
  `__ctype_toupper_loc`, `__ctype_get_mb_cur_max` are emitted (C.9 names).

**Results (keke, role n2).**
* `sl-headers-s06-gate` (final head): `cargo clippy -p xtask --all-targets -D warnings` and the 5 xtask `headers` unit tests:
  rc 0.
* `headers-gen` runs (jobs `sl-headers-s06-gen1` 49 problems, `-gen2` 27, `-gen3` 20): evidence of progress, not on the
  final head's proof list, all rc 1.
* NOT RUN: plants (`cargo xtask plants`), P1 to P3, P5, P6, P10, `cargo xtask static`, the conformance programs. No plants
  file exists yet: the generator changes and the templates are unproved.

**The 20 remaining `headers-gen` problems and the fix for each.**
1. `scanf` family (14): `__isoc99_*` in C++98 and `__isoc23_*` from C++11 on, but the levels have one `cxx` flag. Add a level
   `cxx11` (`headers/levels.tsv` column, `LEVEL_COLS`, `features.h.in`, the `lv()` unit test array, `spec/tools/header_check.py`
   reads the columns) or let the symbol condition use `__cplusplus`.
2. `gets` (3 lines): visible in C89/C99 and C++98/11, absent from `prototypes.tsv` (removed in C11). The template must declare
   it (name and type from the standard, C99 7.19.7.7) or `visibility.tsv` row gets a differences row.
3. `struct _IO_FILE` has no usable layout rows: members whose type is a pointer to a structure (`struct _IO_marker *`) or an
   array of `char`. Extend `members_nested` to pointers to a structure named in `layouts.tsv` or opaque (an incomplete tag).
   Appendix C.9's inline `getc_unlocked` protocol needs the member names (R-H4: complete).
4. `struct timex`, `struct itimerspec` (time.h): check why the nested-structure rule does not take them (`struct timeval`,
   `struct timespec` members: the tag may be keyed `struct timespec` in `layouts.tsv` with a header column filter).

**Not designed or started.** P1 `_Static_assert` TUs (C and C++), P2 and P3 on the staged tree (`header_probe.py` and
`callsite_probe.py` with an extra-flags option), P5 hygiene run, P6 hello and the covered conformance programs
(`conformance/run.sh --liella-headers`), P10 ctype/`assert`/`errno` differentials, the C++ `<cstdio>`... compile in every
`-std`, plants file `handoff/SL-HEADERS-S06/plants.txt`, the pending record for the generator additions
(`records/pending/SL-HEADERS-S06-1.toml`, `id = "D-NEW"`, series D-X: owns-macro, the enumerator kind, the fortify
fallback, `LINKED_ELSEWHERE`).

**Open design points.**
* The brief says `__LIELLA__` is always defined; the tree has `__NIJI__` (D-X.107, the libniji rename). The templates use the tree's.
* The `*_l` ctype macros (`isalnum_l` ...: inline through the locale's `__ctype_b` member) and the `-O2` inline forms of
  `getc_unlocked`, `putc_unlocked`, `getchar` and `putchar` in `callsites.tsv` (`call` rows: `__uflow`, `__overflow`,
  `getc`+`stdin`) are not implemented; they need a differences row or an implementation. The `call` rows are measured only in
  the fortify configurations, so slice 18 may own them.
* The templates emit every name `visibility.tsv` lists for the header (the reference's pollution too: `stdlib.h` shows
  `pthread_*`, `unistd.h` shows `__environ`-style types). P2 will confirm or ask for differences rows.
* `size_t` and `va_list` are typedefs of libniji's own under a guard; whether the compiler's `<stddef.h>`/`<stdarg.h>`
  included afterwards under `-std=c99 -pedantic-errors` redefine them is not tested.

**Next steps.** Fix the four problems above (`TAILN=30 bash` a run helper that syncs to the role, runs `cargo xtask headers-gen`
through `jobrun` and waits is the quickest loop: about 2.5 minutes the first time, less cached); run `cargo xtask headers-gen`
to write `libc-include/`; `cargo xtask static`; then P5 with `spec/tools/header_check.py --include libc-include`; the plants;
the gate. Plan rows: plan/headers.md section 2.4, 3, 4 (slice 6).
