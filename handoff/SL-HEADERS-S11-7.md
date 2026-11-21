# SL-HEADERS-S11-7: slice 11 as data only on the generator

**State.** Branch `task/sl-headers-s11-7` (stacks on `task/sl-headers-s11-6`), origin/next fe0899b79 merged. Earlier history of the slice: `handoff/SL-HEADERS-S11.md`. The generator, `spec/tools/header_check.py` and `abi_extract.py` are next's (`git diff origin/next -- xtask spec/tools/header_check.py` is empty). libc-include is exactly `cargo xtask headers-gen` output (`--check` ok, 170 headers).

**What the slice is.** Templates `headers/{math.h,fenv.h,fpu_control.h,ieee754.h,values.h}.in`; table rows: 147 macros, 40 `union ieee754_*`/`ieee854_*` layouts, 6 typedefs, 160 visibility rows (in the table's key order: header, name, kind rank); `headers/slices/s11.tsv`; `spec/tools/header_checks/s11.py` (P1 floating macros, unions, C-only names; P6 of ieee754.h runs from `problems(ctx)`); `spec/tools/ieee754_fields.c`; records D-10.125 and D-10.126 (rewritten for the data-only slice).
* math.h includes `stdlib.h` in C++: visibility.tsv shows the names the C++ wrapper pulls in (FD_*, MB_CUR_MAX, NULL, alloca, be16toh ...) in math.h; the include covers them, so headers-gen needs no macros.tsv value for them.

**Results (final head).** clippy and `cargo xtask static` clean; `headers-gen --check` ok; `headers-gen --rows`: 0 failed (macros 70 excepted = the `_FloatN` literal macros); header_check: 8 failed of 11659, all P8 `sys/mount.h`/`linux/mount.h` (`struct winsize` redefined; sys/mount.h is identical to next's: INFORMATIONAL, not slice 11); direct-tier gate (`affected --base origin/next --run`) green; plants: see the report.

**RULING NEEDED (HEADERS-TABLES-COMPLETE).** Left out because today's tables cannot express them: (1) `complex.h`, `tgmath.h` (their C++ rows are the wrappers' pull-ins incl. `pthread_attr_t`, a union typedef without a C++ spelling; `tgmath.h` type-generic macros); (2) the 70 `_Float32/64/128/32x/64x` literal macros of math.h (rows-exceptions); (3) hex floating constants of values.h are emitted as hex (C89 and C++98-14 cannot lex them; s11.py asserts them from C99 and C++17); (4) `math-vector-fortran.h`.

**Plants.** `handoff/SL-HEADERS-S11-7/plants.txt` (18 plants). The eight `header_check.py` plants drop the sys/mount.h P8 lines (they fail the baseline otherwise). Brief item 4: `s11-dontcare-index` was a plant on a generator change that is gone.

**Not to redo.** Do not put hand-written header text or generator edits back (contract). Do not prune visibility.tsv rows of complex.h/tgmath.h (typedefs.tsv lists them; abi-types fails).

**Next step.** Land; then HEADERS-TABLES-COMPLETE takes the four items above (complex.h/tgmath.h, `_FloatN` literals, decimal spelling of hex constants).
