# HEADERS-TABLES-COMPLETE-2: handoff and report (2026-11-23)

**State.** Branch `task/headers-tables-complete-2` from origin/next e3b0caf60; role shiki:~/w/sh1. Commits df5d19211
(decimal spelling, `_FloatN` literal form, signaling NaNs, record D-10.134, plants file) and 81cd6e9c1 (abi-types
static check accepts `snan`). Brief items 1 and 2 are done; items 3 and 4 are not started (see Next step).

## What changed

* `xtask/src/headers/values.rs`: `FLOATS` (type, builtin suffix, literal suffix, `*_DECIMAL_DIG` digits) and
  `decimal()`: an exact hexadecimal value as a decimal rounded half to even to 9/17/21/36 digits (C11 5.2.4.2.2).
  Every finite floating macro is now decimal + suffix; `_Float32/64/128/32x/64x` take `f32/f64/f128/f32x/f64x`;
  `snan` -> `__builtin_nans<f> ("")`.
* `spec/tools/type_probe.py`: a NaN with the quiet bit (the fraction's top bit) clear is `snan`. `spec/abi/macros.tsv`:
  the eight `SNAN*` rows `nan` -> `snan` (hand edit matching the probe change; the table has no generator marker).
* `headers/math.h.in`: the 70 names leave `@@owns-macro`; `headers/rows-exceptions.tsv`: the 70 rows are gone (only
  va_list PERMANENT remains). Regenerated: `libc-include/math.h`, `libc-include/values.h`.
* `xtask/src/headers/rows.rs`: FMAX and value checks for the `_FloatN` types; `nan` must be quiet and `snan`
  signaling (`__builtin_issignaling`). `spec/tools/header_checks/s11.py`: no C99/C++17 restriction; every floating
  value, NaN kind and infinity is checked in every configuration that shows it (C `_Static_assert`, C++ array bound).
* Unit test `headers::tests::floating_literals_are_decimal_and_exact` (expected strings from Python's exact
  `decimal`, not from the code, including half-even ties and a carry into a new digit).

## Probe-settled (black-box, gcc/g++ 13.3 on the build host)

* gcc and g++ accept `1.5f32`, `f64`, `f128`, `f32x`, `f64x` in every mode c89..gnu17 and c++98..c++23, with the
  `_FloatN` type; hexadecimal floating constants fail in c89 and c++98/11/14 ("exponent has no digits").
* The reference's `M_Ef32`..`M_Ef64x` have the `_FloatN` types in C++98..C++20 (typeid `DF32_`, `DF64_`, `DF128_`,
  `DF32x`, `DF64x`); its eight `SNAN*` macros are signaling NaNs in C and C++.
* `typedef char t[(float cmp) ? 1 : -1]` and `_Static_assert` fold floating comparisons in every C mode (the array form
  draws a "variably modified" diagnostic under `-Werror`, so C uses `_Static_assert`); in every C++ mode the array form
  works and `static_assert` only from C++11. A false comparison is rejected in each form.

## Results (head 81cd6e9c1 for the jobs; generation checked on df5d19211's code)

* headers-tables-complete-2-gen2 (code of df5d19211): headers-gen ok; `cargo test -p xtask -- headers::` 24 passed;
  `headers-gen --check` ok; `headers-gen --rows` ok (0 failed, the `_FloatN` and SNAN rows included);
  header_check 0 failed of 16193 units.
* Gate: `cargo xtask affected` escalates to FULL (generated headers), so: `headers-tables-complete-2-test1`
  (`cargo xtask static`, `headers-gen --check`, `cargo test -p xtask`) and precheck. Plants:
  `headers-tables-complete-2-plants1` (6 plants, `handoff/HEADERS-TABLES-COMPLETE-2-plants.txt`): 6 CAUGHT, 0 survived,
  no WARN lines. `headers-tables-complete-2-test2` (`cargo xtask so`, then static, `headers-gen --check`,
  `cargo test -p xtask`): all rc 0. SUPERSEDED: gen1 (s11.py C units red: a folded floating array bound is a
  "variably modified" error under -Werror; fixed with `_Static_assert`), test1 (abi-types rejected `snan`, fixed in
  81cd6e9c1; its two iconv transcript failures were the role's missing product, green in test2 after `cargo xtask so`).
* clippy `--workspace --all-targets --all-features -D warnings`: clean (df5d19211).

## RULING NEEDED / INFORMATIONAL

* INFORMATIONAL: before this branch libniji's `SNAN`, `SNANF` and `SNANL` were quiet NaNs (`__builtin_nan`); the
  rows test checked only `x != x`. Fixed here (signaling), with the check strengthened for both NaN kinds.
* INFORMATIONAL: the type_probe change has no test of its own (the probe runs against the reference only when the
  tables are re-probed); its result is pinned by the eight macros.tsv rows and the rows test.

## Next step

Land through the train. Then a follow-up story for brief items 3 and 4, which need more than a form each:
complex.h and tgmath.h do not exist in libniji yet (visibility.tsv: about 3,100 rows each, mostly the C++ wrappers'
pull-ins), so item 3 is a new slice: a C++ spelling for union typedefs (`pthread_attr_t`), type-generic macro rows for
tgmath.h, templates `complex.h.in` / `tgmath.h.in`, their slice table and header_check module. Item 4,
`finclude/x86_64-linux-gnu/math-vector-fortran.h`, is a Fortran `!GCC$ builtin ... attributes simd` file, not C: it
needs a non-C output form of the generator fed by the libmvec tables. Rows to read: plan/headers.md slice 11 (lines
~94, ~310), D-10.125, D-10.126, D-10.134.
