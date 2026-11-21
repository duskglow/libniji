# SL-HEADERS-S11-7: slice 11 as data only on the generator (leg 1 of the run; refresh pending)

**State.** Branch `task/sl-headers-s11-7` (stacks on `task/sl-headers-s11-6`). origin/next (fe0899b79) merged. Earlier history of the slice: `handoff/SL-HEADERS-S11.md`.

**Done.**
* Merged origin/next and took next's `xtask/src/headers.rs`, `spec/tools/header_check.py`, `abi_extract.py` (bit-field form `bits W@P`) and layouts. `git diff origin/next -- xtask spec/tools/header_check.py` is empty (slice contract).
* `spec/abi/layouts.tsv`: only the 40 `union ieee754_*`/`ieee854_*` rows added (re-extracted with `abi_extract.py --layouts-only` on the build host; next's form).
* Templates kept: math.h, fenv.h (pointer-valued macros through `@@value`), fpu_control.h, ieee754.h, values.h. `@@emit-c`, `@@if-macro`, `@@generated-fortran` no longer exist: frames use `@@emit`, `@@if`.
* `spec/abi/visibility.tsv`: next's table plus 160 slice-11 rows (math.h, fenv.h, fpu_control.h, ieee754.h, values.h), restored by scratch script (pre-existing rows are untouched).
* **Left out (RULING NEEDED, HEADERS-TABLES-COMPLETE):**
  1. complex.h and tgmath.h are not in the slice: their C++ rows are the libstdc++ wrappers' pull-ins (pthread_*, FD_*, MB_CUR_MAX, ...); the generator emits them without `pthread_attr_t` (a union typedef without a C++ spelling), so complex.h fails to compile in C++ (`pthread_attr_destroy initialized and declared extern`). Pruning those rows breaks `abi-types` (typedefs.tsv lists them), so the headers are out. tgmath.h's type-generic macros are the other reason. Templates deleted, tgmath_types.c deleted.
  2. The 70 `_Float32/64/128/32x/64x` literal macros of math.h (M_Ef32 ...): the generator has no literal form for those types; math.h `@@owns-macro` them and `headers/rows-exceptions.tsv` has 70 `macros` rows.
  3. Hex floating constants (values.h MAXDOUBLE/MAXFLOAT/MINDOUBLE/MINFLOAT) are emitted as hex, which C89 and C++98-14 cannot lex: s11.py P1 asserts them only in C99+ and C++17+ (guard `H:`). Fix belongs in the generator (decimal spelling).
  4. math-vector-fortran.h (a Fortran include) is out: no template, row removed from s11.tsv.
* `spec/tools/header_checks/s11.py`: P1 skips excepted names; hex guard; **P6 of ieee754.h now runs from `problems(ctx)`** (gcc -O1 -S on `spec/tools/ieee754_fields.c`): header_check's units are parser-only, so as a unit the proof was vacuous once header_check.py is next's.
* Last green (before complex/tgmath removal, chk5 on the build host): `headers-gen --check`, `headers-gen --rows` (0 failed, 70 excepted for math.h), header_check 8 failed of 11777, all P8 `sys/mount.h`/`linux/mount.h` (`struct winsize` redefinition; sys/mount.h is identical to next's: INFORMATIONAL, not slice 11).
* Brief item 4: `s11-dontcare-index` was a plant on my generator change; the generator is now next's, the plant is gone. Plants file `handoff/SL-HEADERS-S11-7/plants.txt` (18 plants; every `msg:` names the check or the stale-header message).

**Not done / exact next step.**
1. `cargo xtask headers-gen` on the build host, copy back libc-include (rsync), commit; then `cargo xtask clippy && cargo xtask static`. The last static run (before the removal) failed on: (a) records D-10.125 (`tests` names `xtask/src/headers.rs::floating_literals_are_decimal_and_exact`, which no longer exists; its body also describes decimal literals, `@@emit-c`; rewrite both records D-10.125 and D-10.126 to the data-only slice, drop `spec/tools/tgmath_types.c` from `tests`); (b) abi-types (1991 problems, all from the pruned complex.h rows: should vanish with the visibility.tsv restore; check also the math.h pull-in names that had no `macros.tsv` value: if headers-gen errors on them, they are next's rows for math.h: report, leave out via `@@owns-macro` + rows-exceptions only if the rows test allows).
2. Re-run header_check and the rows test, then plants (`cargo xtask plants handoff/SL-HEADERS-S11-7/plants.txt --dir /dev/shm/claude/sl-headers-s11-7/plants-1 --timeout 600` under jobrun, label `sl-headers-s11-7-plants`), gate (`cargo xtask affected --run` direct tier, `--base origin/next`), `cargo xtask precheck --jobs ... --head <sha>`.
3. Write `handoff/reports/SL-HEADERS-S11-7.json`, run `ops handback-check`.

**Not to redo.** Do not put hand-written header text or generator edits back (contract). Do not prune visibility.tsv rows of complex.h/tgmath.h again.
