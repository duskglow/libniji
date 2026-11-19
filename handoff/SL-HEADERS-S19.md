# SL-HEADERS-S19: slice 19, diagnostic attributes (handoff)

**State.** Branch `task/sl-headers-s19`. Head: see `handoff/reports/SL-HEADERS-S19.json`. Say "slice 19" in commits and handoffs.
Clippy and `cargo xtask static` are clean on the code head. The plants file is `handoff/SL-HEADERS-S19-plants.txt` (8 plants,
one per attribute kind).

**What was done.**
* `spec/tools/attr_probe.py` (Appendix C.14). `table` writes `spec/abi/diagattrs.tsv` (name, nonnull, wur, noreturn, format,
  malloc, alloc_size, deprecated, sentinel, pure) from a corpus compiled against the system's headers; `check` compiles the same
  corpus against the system's headers and against `libc-include/` and prints each test line on which the compared options
  differ. Clean room (plan/headers.md section 2.2): the reader keeps only the line number and option name of each diagnostic
  in the probe's own file (`-fdiagnostics-format=json`, text dropped) and the undefined `mk_*` marker symbols of the probe's own
  object (noreturn, malloc and alloc_size are decided by whether the optimizer removes a call to a marker). No `-E`, `-dM`,
  disassembly or header text is read.
* `headers-gen` reads the table (`attrs`, `attr_text`), puts `__attribute__ ((...))` after the assembler label of each
  generated declaration, and substitutes `@@attrs(NAME)` in templates (`headers/string.h.in`: the strchr family, C and C++).
  `libc-include/` regenerated (7 headers changed).
* Result: 628 functions, 2688 tests (after merging next; was 519 and 2232), 0 differences in the compared options (-Wnonnull, -Wunused-result,
  -Wdeprecated-declarations, -Wformat*, -Wunused-value, and the noreturn/malloc/alloc_size markers).
* Pending decision record `records/pending/SL-HEADERS-S19-1.toml` (D-NEW, series D-X).

**Standards basis of each kind** (the probe says which function carries it; where a standard gives the rule, it is this):
format: C 7.21.6 (printf family), 7.21.7 (scanf family), 7.27.3 (strftime); nonnull: C 7.24.1p2 (string functions take valid
pointers, even for length 0) and the stream argument of 7.21; noreturn: C 7.22.4 (`abort`, `exit`, `_Exit`, `quick_exit`),
POSIX `_exit`; malloc and alloc_size: C 7.22.3 (`malloc`, `calloc`, `aligned_alloc`; `realloc`/`reallocarray` carry alloc_size only);
warn_unused_result and deprecated: man pages (`getwd` is LEGACY in POSIX); sentinel: `execl*` (POSIX: the list ends with a null pointer).

**Not matched, by design (open, belongs to slice 18).** `-Wformat-overflow=`, `-Wformat-truncation=`, `-Wstringop-overflow=`,
`-Wstringop-truncation` also read `access` and `restrict` information; the check counts them (`open`) and does not compare
(8 + 4 + 1 + 2 test lines today: sprintf/snprintf/strncat/strncpy/stpncpy with a null destination).

**Known differences (printed by `check`, not counted).**
* `is*` ctype macros of slice 6: `isalpha(1)` does not compile against libniji's `ctype.h` (the macro names `_ISalpha`
  enumerators that the header does not declare). A real defect of the landed header, not of slice 19: needs a slice 6 fix.
* `fwrite_unlocked`: libniji declares a plain function, so GCC's own builtin knowledge adds `-Wnonnull`.

**Things for a continuation.** `gets` (template-written; deprecated or removed by standard version) has no row; probe it in
a `-std=gnu99` configuration. C++ is not part of the corpus (the C++ overloads carry the same attributes). A new slice that
adds headers reruns `attr_probe.py table` (it probes every function the headers declare) and `check`.

**Source compatibility.** No package corpus was built here (P7 is slice 20): no Aqours source patch is known to be needed.

**Continuation SL-HEADERS-S19-2.** The CI red (corpus "does not build", lines []) had two causes, both in the probe: (1) slices that
landed on next include kernel UAPI headers (pty.h: asm/ioctls.h) and the probe's `-nostdinc` hid them; the probe now adds a
directory of links to only asm/, asm-generic/ and linux/ of the system; (2) errors inside an included header were dropped, so the
list was empty; the probe now quotes up to five compiler errors from libniji's own headers (the reference side is never quoted).
The table was rerun for the functions of the newly landed headers (pwd, grp, shadow, syslog, sys/time, utime...) and headers-gen
regenerated `libc-include/`: 628 functions, 0 differences.

## Continuation S19-3 (move onto the per-slice layout)

* Merged origin/next (SL-HEADERS-REGISTRY landed). Took next's `xtask/src/headers.rs`; the slice's own edits there were
  re-applied as the one shared-generator change that cannot be a slice file: the `diagattrs.tsv` table in `Tables`, the
  attribute text appended to each `@@emit` declaration, and the inline `@@attrs(NAME)` of hand-written lines. The slice
  adds no header (no `headers/slices/s19.tsv` row: every header it touches belongs to an earlier slice's row, and a
  header may be in one row only) and no check module (`spec/tools/header_check.py` was not edited by the slice).
  The differential test moved to `xtask/src/headers/slices/s19.rs` (`slices::s19::tests`).
* Two defects found by `header_check.py` after the merge (187 failed of 4563), both fixed: (1) the reference declares abs,
  labs, llabs, imaxabs, ffs*, div, difftime, getpagesize and the ctype/errno location functions `const`, which the compiler's
  warnings cannot tell from `pure`; the compiler rejects `pure` on the builtins abs/ffs. `attr_probe.py table` now reads the
  reference's preprocessed headers for `const` (cell value `const` in the `pure` column), the generator emits `__const__`.
  (2) the C++ inline overloads in `string.h` take their attributes before the declarator. Result: 0 failed of 4563.
* Plants: 10 (8 old + `s19-const-table`, `s19-const-emit`). The two const plants are caught by the products build's
  staleness check (`headers-gen: stale under libc-include/`), the same limit as the registry's plant: the plants tool builds
  products first, which refuses a table or generator that no longer matches `libc-include/`. header_check's own catch of
  `pure` on `abs` was shown by the 187 failures before the fix.
* Not to redo: the table rerun (628 functions) and the merge.
* Next step: land.

