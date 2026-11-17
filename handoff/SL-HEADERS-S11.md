# SL-HEADERS-S11: slice 11, math (handoff after run 1)

**State.** Branch `task/sl-headers-s11` (cut from origin/next 20e47e117; slice 6 is already on next). INCOMPLETE: stopped at the
context budget. Say "slice 11" in commits and handoffs. No gate, no plants, no clippy/static and no P2/P3/P6 ran on this head.

**Done.**
* Tables (header_probe.py `math_names()`, rerun on `math.h complex.h tgmath.h fenv.h fpu_control.h ieee754.h values.h` only,
  job `sl-headers-s11-probe3`): `visibility.tsv` +424 rows, every earlier row unchanged (checked: old rows are a subset of the
  new run). `macros.tsv` +147 rows (M_* in 8 types, HUGE_VALF/L, MAXFLOAT, `_FPU_*`, values.h constants) and `typedefs.tsv` +6
  (`float_t`, `double_t`, `fpu_control_t`, the three ieee754 unions; type_probe, new names only, so the header column of a name
  another header also shows is the first of the restricted run). The other headers were not re-probed: a new name that
  another header (not in this list) also shows has no row there.
* Generator (`xtask/src/headers.rs`): `@@emit-c` (names a C configuration shows; C++ configurations do not decide a
  condition: libstdc++'s math.h/complex.h/tgmath.h wrappers supply the C++ names, e.g. complex.h has 2712 C++-only rows),
  `@@generated-fortran`, floating literals in `literal()` (exact hex constants with the C or `fN` suffix, `inf`/`nan` via
  `__builtin_inf*`/`__builtin_nan*`), `complex` spelled `_Complex` in prototypes.
* Templates: `math.h` (classification and comparison macros on GCC builtins; `isinf` is `__builtin_isinf_sign`: the reference
  returns -1 for -inf, probed), `complex.h` (`complex`, `_Complex_I`, `I`, `CMPLX*` on `__builtin_complex`), `fenv.h`
  (`FE_DFL_ENV` -1, `FE_NOMASK_ENV` -2, `FE_DFL_MODE` -1, probed as bytes of the probe's own object), `fpu_control.h`
  (`_FPU_GETCW/SETCW` from the x87 FNSTCW/FLDCW), `values.h`, the Fortran header (empty, R-H5).
  `headers-gen` writes 33 headers; `headers-gen --check` ok (job `sl-headers-s11-check2`).
* `header_check.py`: slice 11 headers in HEADERS and the P1 list; floating macro P1 (`_Static_assert`); most-negative literal.

**Open (found by `sl-headers-s11-check2`, rc 1).**
1. Hex floating literals do not tokenize in C89 and C++98 (`exponent has no digits`): `MAXFLOAT`, `MAXDOUBLE`, `M_PI` ... are used
   in those modes (g++ predefines `_GNU_SOURCE`). The generator must write decimal literals with enough digits to round-trip
   (9 float, 17 double, 21 x87 long double, 36 binary128; exact conversion needs a small bignum in `literal()`, or a decimal
   column from type_probe via Python's `decimal`). This is a real user-visible defect, not a test artefact.
2. `fenv.h` in C++11+: `int fegetexceptflag(short unsigned int*, int)` "conflicts with built-in declaration": the prototype is
   emitted with `fexcept_t` resolved to `unsigned short`; check the spelling the compiler's built-in uses (keep the typedef name).
3. `header_check.py` P1 for `@@emit-c` headers must skip names that have no C row (complex.h cxx11: `FILE` row comes from the
   wrapper). Same filter as the generator.
4. `tgmath.h` not written (about 110 type-generic macros on `__builtin_tgmath`, list = the tgmath.h C macro rows of
   visibility.tsv; narrow arithmetic `fadd`... generic over double/long double and the `f32add`... family; the template must
   `@@owns-macro` them all). `ieee754.h` not written: `layouts.tsv` has no member rows for `ieee754_float/double`,
   `ieee854_long_double` (bit-fields); extend abi_extract.py first.
5. Not run: P2/P3 (`header_probe.py --include`, `callsite_probe.py --include`, `probe_compare.py`), P5 extras, P6 (libm
   conformance programs against the staged headers; `cargo xtask so`), `<cmath>`/`<complex>` in every `-std`, type-generic macros on
   float/double/long double/_Float128, clippy, static, plants (`handoff/SL-HEADERS-S11/plants.txt` not written), gate.
6. Pending record not written (`records/pending/SL-HEADERS-S11-1.toml`, D-NEW, series D-X): `@@emit-c`, floating literals, the
   `math_names()` candidates (interface identifiers; values and visibility come from probes), `iscanonical` is `1` (the x87
   non-canonical encodings are not rejected: needs a decision), `iseqsig` on relational operators.

**Next step.** Fix 1 (decimal literals) first; regenerate (`cargo xtask headers-gen`, copy `libc-include/` back), rerun
`header_check.py`; then 2-4, then P2/P3/P6, plants, clippy, static, gate.
