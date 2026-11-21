# MATH-POW-EDGES: pow's signalling-NaN cases and pow(-1, huge even)

State: branch `task/math-pow-edges`; code commit 6391de2bd (the report commit follows it). Ruling #6108 in force
(follow IEEE 754-2019 9.2.1 / 6.2 and C23 F.10.4.5). No new decision record: a local fix of the special-case
dispatch toward the specification, no difference from the reference left in the cases touched.

## Change

* `crates/math/src/pow.rs` (the accurate path of `pow`, `powf`, `powf32`, `powf32x`, `powf64` and, by its `jmp`,
  `pow@GLIBC_2.2.5`): a signalling-NaN operand no longer takes the `x^±0 = 1` / `1^y = 1` shortcut; it reaches
  `nan_operands` (quiet NaN of that operand, sign kept, FE_INVALID, errno unchanged). `x = -1` with an integer y
  returns ±1 by y's parity right after the domain check (`integer_parity` already reads every |y| >= 2^PREC as
  even). Cause of the old 0/ERANGE: the power-of-two shortcut formed `two_prod(0, y)`, whose Dekker split of y
  overflows near DBL_MAX, so it fell through to the core with |x| = 1, which the core is not built for.
* `crates/math/src/wide.rs` (`powl`, `powf64x`, `powf128`): the same sNaN rule; the old x87-only exception for
  `sNaN^±0` is subsumed. `(-1)^huge` was already right there (Mp arithmetic, no splitting).
* The fast tiers (`quick_pow::pow_tiered`, `fast64::pow_nearest`, `pow_onepass`) already refuse x = -1 and NaN
  operands; untouched, so the bench rows' paths do not move.

## Probe-settled behaviour (black box, hand-built probe: reference vs libniji's loader and libraries)

Reference, all four formats: `pow(±sNaN, ±0)` and `pow(1, ±sNaN)` = quiet NaN with the sNaN's sign, FE_INVALID,
errno 0. `pow(-1, ±1e308)`, `pow(-1, ±DBL_MAX)` = 1, no flags, errno 0. Base libniji: 1 with FE_INVALID for every
sNaN case except `powl(sNaN, ±0)`; double `pow(-1, huge)` = 0, FE_UNDERFLOW|FE_INEXACT, ERANGE; powf, powl and
powf128 at -1 already 1.

INFORMATIONAL (not changed): the reference's `powl(-1, y)` raises FE_INEXACT for |y| >= 2^64 (exact result 1);
libniji raises nothing there, which IEEE 754 7.6 supports (the result is exact). Not in this story's scope.

## Tests and proof

* `crates/math/tests/s10_math_clauses.rs` `s10_3_c_r3`: new `pow_edges::<F>` for f32, f64, F80, F128 (sNaN with
  both zero signs, 1^±sNaN, (-1)^{2^P, 2^(P+7), 3·2^P, ±max} = 1 and (-1)^(2^(P-1)+1) = -1, every direction).
* `crates/core/tests/crt_products.rs` `old_libm_versions_bind_and_match_the_reference`: the five DEFAULT_POW
  exemptions this story fixes are removed, so the reference comparison (bits, errno, flags) now covers them for the
  default `pow` and `pow@GLIBC_2.2.5`. The remaining three (FE_UNDERFLOW on an exact subnormal) are unchanged.
* Plants: `handoff/MATH-POW-EDGES-plants.txt` (each fix reverted on its own).
* Jobs (keke, role n3): see `handoff/reports/MATH-POW-EDGES.json` (`jobs`).

## Counters (cbench_libm, Zen 1 host, instructions per call, libniji / reference)

| row | before | after | reference |
|---|---|---|---|
| pow | 91.3 | 91.2 | 60.4 / 60.1 |
| pow_upward | 495.7 | 494.5 | 61.0 / 60.6 |
| powf | 58.4 | 58.4 | 26.7 / 26.4 |
| powl | 62837.7 | 62819.0 | 1280.5 / 1295.0 |
| powf128 | 62949.7 | 62872.8 | 9022.5 / 8988.0 |

Parity by instructions (the reference column moves by the same run-to-run spread): the fast tiers are untouched.
Before: job math-pow-edges-counts-before3 (base ebb670553); after: math-pow-edges-counts (final code).

Results: gate math-pow-edges-gate rc 0 (clippy, static, direct tier: 3 suites, 674 passed, 0 failed, peak 1.6G);
plants math-pow-edges-plants rc 0: minus-one-shortcut and snan-shortcut-wide CAUGHT, snan-shortcut-small
WARN-NO-MSG: caught (rc 101) by the earlier `pow(1, sNaN)` INVALID assertion of the same test (line 968), which
runs before `pow_edges`; a real catch. After the fix the hand probe matches the reference on every case of the
story (bits with sign, flags, errno) in all four formats; only the powl FE_INEXACT lines below still differ.

## Notes

* The probe service (`tools/probe/remote.sh --liella`) reported "products still missing" after its own
  `cargo xtask so` (it looks for the products where the build no longer writes them); a hand probe through
  libniji's `ld-linux-x86-64.so.2` with `LD_LIBRARY_PATH` replaced it. libniji's loader has no `--library-path`.
* Next step: none for this story.
