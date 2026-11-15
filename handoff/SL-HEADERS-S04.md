# SL-HEADERS-S04: call-site and noexcept tables (slice 4 of plan/headers.md), CHECKPOINT handoff

Branch `task/sl-headers-s04` (from origin/next). Role keke:~/w/n2. Brief: ~/state/briefs/SL-HEADERS-S04.md.
Stopped at the context checkpoint (~145k of the ~150k checkpoint); state is WIP, pushed.

## State
* `spec/tools/callsite_probe.py`: writes `spec/abi/callsites.tsv` and `spec/abi/noexcept.tsv`. Reads back only
  error line numbers in its own units, `readelf -rW` relocations of its own sections, `nm` sizes/demangled names
  of its own symbols (docstring has the method). About 2 min with 20 threads on keke.
* Tables committed from run `sl-headers-s04-probe3`; `-probe4` is byte-identical (both on keke, ca0f1c8cc tree).
  3778 of 3780 functions probed (2 have no `function` row in visibility.tsv), 7889 rows, 78 `!error` rows (all
  `addr` in C++ configurations: `&f` on a C++ overload set such as strchr, abs).
* Coverage: all 44 `__isoc*` and 81 of 83 `*_chk` exports are reached. Exempted with reason in the checker:
  `__gets_chk` (no `gets` prototype row) and `__vprintf_chk` (fortified `vprintf` references `__vfprintf_chk` +
  `stdout`). FD_SET/FD_CLR/FD_ISSET are probed (mode `macro`) and reach `__fdelt_chk`.
* `xtask/src/callsites.rs`: static check `callsites` (also `cargo xtask callsites`), 6 unit tests. Helpers
  shared with visibility.rs made `pub(crate)`. Head has the fix for the static-checks array length (36 -> 37);
  the unit tests have NOT yet run green (`sl-headers-s04-xtest` failed to compile before that fix).

## Fixed (do not redo)
* Run 1/2 differed only in the FD_* rows: the three macro jobs shared unit file names and raced. Now one unit
  per (header, configuration) and a guard refuses two jobs with the same (mode, header, configuration).
* Pointer arguments are `(void *)` buffers (no type spelling; `vprintf`'s `__va_list_tag *` failed before).
* C++98 redeclaration: `__va_list_tag*` spelled `__builtin_va_list`; `-fno-builtin` in the plain-include
  units; both-compile is `unknown` (1 row), not `error`.

## Open
* noexcept.tsv cxx98 has 241 `error` rows (neither redeclaration compiled). Likely cause, NOT VERIFIED: a whole
  header failing in the `-nostdinc -I` (non-system) mode raises in compile_unit and is caught as `ok = []`
  (ctype functions, fstat*, obstack). Check by logging which headers raise; then decide fix or label the gap.
* 368 rows are cxx17 `-` but cxx98 set (e.g. `__cabs*` in complex.h): a visibility.tsv fact, not checked further.
* Plants file not written; gate not run; no spec C.13 text or PROVENANCE line yet.

## Next step
1. Sync, run `cargo xtask clippy` + `cargo xtask static` + `cargo test -p xtask callsites` on the role.
2. Look into the 241 cxx98 errors (above); rerun probe twice and `cmp` if the tool changes.
3. Plants on `xtask/src/callsites.rs` (pattern: handoff/SL-HEADERS-2/plants.txt), gate (direct tier), report.
4. Appendix C.13 text and a PROVENANCE line (as SL-HEADERS-2 did for C.12).

Plan rows: plan/headers.md §2.2, §3 (P3, P4), §4 slice 4.
