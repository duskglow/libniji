# HEADERS-NULL-TYPE: handoff and report

State: branch `task/headers-null-type` from origin/next 56dd6bde9, role mei:~/w/s2. Ruling #6743 (match the reference):
a macros.tsv type `null` (value 0), emitted per language: `((void *) 0)` in C, `__null` in C++.
Only va_list's C++ member offsets stay in `headers/rows-exceptions.tsv` (PERMANENT, ruling #6721).

## What changed

* `spec/abi/macros.tsv`: rows `NULL stdlib.h null 0` and `TD_SIGNO_MASK thread_db.h null 0` (the visibility rows show
  NULL in stdlib.h; thread_db.h has no template yet, so TD_SIGNO_MASK is a table row only).
* `xtask/src/headers/values.rs`: `null_lines` (the one definition, both languages); `xtask/src/headers.rs`: the
  `@@null NAME` marker. The 13 templates that spelled NULL by hand now use `@@null NULL`.
* `xtask/src/headers/rows.rs`: type `null` check: C `void *` (`__builtin_types_compatible_p`), C++ `long`
  (`niji_same<decltype>`), value 0 at run time from both the macros and the constants row; it uses the generator's own
  lines. `xtask/src/abi_types.rs`: the `null` type, value must be 0 (unit test `null_constants`).
* `headers/rows-exceptions.tsv`: the NULL and TD_SIGNO_MASK rows removed.

## Changed header lines

None. `cargo xtask headers-gen` on the role gave `libc-include/` and `crates/abi` byte-identical to the tree: the
templates already spelled NULL as `__null` in C++, so the move to the table changes the source, not the output.
(TD_SIGNO_MASK is not emitted today.)

## Plants (handoff/HEADERS-NULL-TYPE-plants.txt)

null-cxx-spelling (C++ spelled `((void *) 0)`), null-c-type (C spelled `((char *) 0)`): the rows test; null-value-check:
`null_constants`. Results in the report json / hand-back.

## Notes

* The "sample names must have a table row" improvement: the rows test iterates the tables, and the new unit test uses
  the existing MAC fixture row (`FP_NAN`) it replaces, so the name is a row by construction.
* Process slip: one heredoc attempt (refused by the hook, no effect).
* Premise: the brief's (#6743 and the probe in HEADERS-TABLES-COMPLETE); nothing that landed since moves it.
