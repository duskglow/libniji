# MP-ISQRT-FAST: the math oracle's square root and products (D-10.130)

Branch `task/mp-isqrt-fast` (stacked on `task/math-oracle-profile`, D-10.129). Host keke (Zen 1, 60 vCPU), role n2.
Plants: `handoff/MP-ISQRT-FAST-plants.txt`. Report: `handoff/reports/MP-ISQRT-FAST.json`.

## What changed (crates/mathtools only; test and tool code, no product)

1. `Nat::sqrtrem` (root and remainder) replaces Newton-on-division `Nat::isqrt`: Zimmermann's Karatsuba square
   root (INRIA RR-3805; Brent-Zimmermann MCA Alg. 1.12) on bit fields, one half-size division per level, correction
   loops that make it exact whatever the estimate's error, u128 base case from the double root plus integer
   correction. Callers use the remainder instead of squaring the root (`sqrt_parts`, exact `pow` roots in func.rs).
2. `Big::sqrt` (correctly rounded) takes the root from the operand's leading 2p+6/7 bits; dropped bits only set the
   sticky bit, so the rounded value is identical. Measured first (scratch instrumentation): 95 % (f128) / 90 % (x87)
   of the quadratic square-root work was operand bits beyond those (exact products and sums of the complex oracle).
   `sqrt_exact` keeps the whole operand (its returned value unchanged).
3. `Nat::mul`: Karatsuba (TAOCP 4.3.3) when both factors have >= 40 limbs, unbalanced products split first. After
   (1)+(2) the self profile was `Nat::mul` 60 % (f128) / 78 % (x87).

ln 10 (brief item 3): NOT done. The precision a process needs comes from Ziv's loop per input (not known up front),
and the cache already grows geometrically (recomputation <= 2x the largest). After this change `consts::compute_ln10`
(through `log_core`) is still 11.8 % (f128) / 8.7 % (x87) of shard 0, inclusive. A cheaper ln 10, 3 ln 2 + 2 atanh(1/9)
by series (ln 2 is already cached), is unmeasured. It runs on `divrem_small` (`__udivti3`) and can change the cached
constant's last bits, so `part(2p+40)` values could change; correctly rounded results would not (finding #6890).
RULING NEEDED to allow it as a follow-up story (recommended).

## Equivalence and counts

- Replay harness (scratch, `~/work/mp-isqrt-fast/files/isqbench.rs` on keke): 1/50 of the differing and random lines
  of the saved complex_f128 / complex_ld_cr program outputs through the sweeps' `cr` (Ziv to 2^17) and `part(2p+40)`;
  digest of every result. Same digest before and after every step (f128 `a5cd00a0399f56dd`, x87 `f57a8809f0162ed3`).

| harness, instructions:u | base | + sqrt (1)(2) | + Karatsuba (3) |
|---|---|---|---|
| complex_f128 1/50 | 73.16 G | 33.91 G | 26.22 G (-64 %) |
| complex_ld_cr 1/50 | 207.89 G | 113.75 G | 85.04 G (-59 %) |

Shard 0 of each, one thread, `NIJI_NO_VERDICT_CACHE=1`, `perf stat -e instructions:u` (whole test process,
programs included); `~/work/mp-isqrt-fast/measure.sh` on keke n2, jobs `mp-isqrt-fast-before` (base a33f92314) and
`mp-isqrt-fast-after` (head 57c4154bc):

| shard 0, instructions:u | before | after | change |
|---|---|---|---|
| complex_f128 | 641.11 G | 165.00 G | -74.3 % |
| complex_ld_cr | 765.83 G | 235.08 G | -69.3 % |

Pass-1 test-seconds of the five sweeps (same host, same role, `cargo nextest ... --test-threads 24`, under
`~/bin/gate 16G`; wall clock on a shared host, so noisy; `ts.sh` of MATH-ORACLE-PROFILE):

| binary | before | after |
|---|---|---|
| complex_ld_cr_products (21) | 12912.5 | 4388.7 |
| complex_f128_products (8) | 3936.8 | 1557.5 |
| special_ld_products (9) | 2319.2 | 1597.7 |
| special_f128_products (8) | 2319.0 | 1615.6 |
| complex_products (8) | 273.4 | 389.8 |
| total | 21760.8 | 9549.3 (-56 %) |

complex_products went up 273 -> 390 s, in wall time only. It was not counted deterministically: its oracle is the
double-precision `cplx::complex`, which the replay harness does not cover. Wall time on this slot is noisy: the two
runs of MATH-ORACLE-PROFILE moved the same row 479 -> 217 s with no oracle change. NOT VERIFIED either way.
Next step: an instruction count of one complex_products shard on the base and on this head (INFORMATIONAL).

Profile after (complex_f128 shard 0, perf 199 Hz, DWARF; `~/work/mp-isqrt-fast/after/f128.*.txt`): the oracle is still
~86 % of the shard; `Nat::mul` 34.6 % self (56 % inclusive), `__udivti3` 12.2 %, then `Nat::add`/`shr`/`sub` and malloc.
Inclusive: `cplx::tanh_parts` 58 % -> `func::mulw` 55 % -> `Big::mul` (exact) -> `Nat::mul`. `Big::sqrt` is no
longer visible above 4 %.

Tests and proof: lint `mp-isqrt-fast-lint-2` (clippy workspace all targets all features -D warnings, `cargo xtask
static`, `records check`: rc 0; the first `-lint` failed on the record's key order, fixed in 57c4154bc). Plants
`mp-isqrt-fast-plants`: 4 plants, 4 CAUGHT with their messages, 0 WARN. Gate
`mp-isqrt-fast-gate` (`cargo xtask affected --run`, direct tier, code head 57c4154bc): rc 0, 9 suites ok, 4357 passed,
0 failed, 359 ignored, 0 SKIPPED lines, peak 1.8G.

## Next

- Next lever (profile above): `func::mulw` under `cplx::tanh_parts` forms the exact product of long operands, then
  rounds to w bits. A truncated product (operands cut to w + guard bits, error bounded) would remove most of the
  56 %. Then `__udivti3` (Knuth D's u128 quotient estimate, `divrem_small`: a 128/64 division) and `Nat` allocation.
- ln 10 by series only if a call-graph profile shows `ln10_w` again.
- Mistakes of this run: the chain script on keke was edited once with `sed -i` (a scratch file, not refused, but
  against the rules). One ssh call chained commands before `setsid nohup` and was held open until its 10-minute
  limit (the remote chain was detached and unaffected).
- ORACLE-VALUE-CACHE re-measures on top of this (coordinator ruling #6889).
