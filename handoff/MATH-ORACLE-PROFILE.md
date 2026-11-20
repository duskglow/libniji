# MATH-ORACLE-PROFILE: profile of the C-oracle sweeps; their programs run once per test run

Branch `task/math-oracle-profile` (from origin/next 90f71362a). Decision in force: D-10.129 (amends the sweeps of
D-10.57). Plants: `handoff/MATH-ORACLE-PROFILE-plants.txt`. Host: keke (Zen 1, 60 vCPU), role n2.

## 1. Profile (perf, 199 Hz, DWARF call graphs; the dev test profile the gate builds: test crate opt 2, niji-mathtools opt 3)

One shard each, `NIJI_NO_VERDICT_CACHE=1`, `--test-threads 1`, alone on the role. Shares of all samples
(inclusive, `perf report --children`):

| complex_ld_cr shard 0 (7347 of 146932 lines, 3879 differ; 119.8 s) | share |
|---|---|
| the oracle: `niji_mathtools::cplx::complex_wide` (through the test's `part`/`cr`) | 90.4 % |
| of it: `Big::sqrt` -> `Nat::isqrt` (Newton steps on `Nat::divrem`) | 54.9 % |
| of it: `log_core` / `log1p_kernel` | 33 % / 31 % (overlap the sqrt share) |
| of it: `sin`/`cos` reduced | 18 % |
| building and running the two programs (test's own timer: 4.6 s of 119.8 s) | ~4 % |
| parsing, comparison, flags, errno, tally, I/O | < 2 % (no test-crate symbol >= 0.3 % self) |
| self time: `Nat::divrem` 56.1 %, `Nat::mul` 31.3 %, `__udivti3` 3.7 % | |

| complex_f128 shard 0 (10539 of 84308 lines, 4920 differ; 99.5 s) | share |
|---|---|
| the oracle: `cplx::complex_wide` | 96.8 % |
| of it: `Big::sqrt` -> `Nat::isqrt` | 60.7 % |
| of it: `consts::ln10_w` (recomputing ln 10 as the precision grows, per process) | 10.8 % |
| building and running the two programs (3.0 s of 99.5 s) | ~3 % |
| parsing, comparison, I/O | < 2 % |
| self time: `Nat::divrem` 60.9 %, `Nat::mul` 24.9 %, `__udivti3` 4.8 % | |

So the oracle dominates (premise confirmed), and inside it one routine does: the integer square root by division.

## 2. Programs once per test run (D-10.129)

`verdict_programs::shared_outputs` (crates/core/tests/support): the first shard of a nextest run runs both programs
and stores their outputs under `target/test-oracles/outputs/<sweep>-<key>-<run>.<j>`; the key is the verdict key
without the shard, `<run>` is `NEXTEST_RUN_ID` (or the pid under `cargo test`), flock + temp-and-rename; a failed
program, an incomplete key or a failed write stores nothing. Tests: `verdict_cache.rs`
(`the_shards_of_one_run_share_the_programs_outputs`, `an_output_of_a_failed_program_or_an_incomplete_key_is_not_shared`).
Plant `sweep-every-shard` breaks libniji's `csqrtl` and must fail all 20 complex_ld_cr shards.

Pass-1 test-seconds of the five sweeps on keke (n2, `cargo nextest run ... --test-threads 24 --no-fail-fast`,
`NIJI_NO_VERDICT_CACHE=1`, under `~/bin/gate 16G`; jobs `math-oracle-profile-before` / `-after`):

| binary | before | after |
|---|---|---|
| complex_ld_cr_products (21) | 12965.7 | AFTER_LD |
| complex_f128_products (8) | 4598.7 | AFTER_F128 |
| special_ld_products (9) | 2459.9 | AFTER_SLD |
| special_f128_products (8) | 2327.6 | AFTER_SF128 |
| complex_products (8) | 479.3 | AFTER_C |
| total | 22831.2 | AFTER_TOTAL |

(keke's numbers are ~3.3x CI's 6930 test-s: 24 threads on a smaller slot; compare before with after only.)

## 3. For ORACLE-VALUE-CACHE (owner ruling 11-20: approved, its own story stacked on this one)

- Saving: at most the oracle share, 90-97 % of a shard (about 20k of the 22.8k keke test-s, ~6.2k of CI's 6930), on
  runs where the lines that reach the oracle were seen before. The grid and the random sample are fixed (the C
  programs seed their xorshift with a constant), so a product change sends mostly the same lines.
- Key of one value: the bytes of `crates/mathtools/` sources (it has no dependencies; Cargo.lock lists none for it), the
  toolchain (`rustc -vV`: release and commit hash; `rust-toolchain.toml` pins 1.99.1), the build flags of the test
  profile (`[profile.dev.package.niji-mathtools] opt-level = 3`, any `RUSTFLAGS`/`.cargo/config` flags, the target
  triple), and the CALLER's wrapper, which is not in mathtools: each sweep's `part`/`cr` (function -> oracle routine,
  `carg`'s operand swap, start precision 64, the Ziv cap 2^17 bits, `exact_zero`), so the sweep source bytes or a
  versioned wrapper id; then function, format, rounding mode, operand bits and part index. Cache the decided result
  only (rounded bits, inexact/underflow/overflow, exact zero), never `None` (undecided).
- Outside-the-key dependence: `mp/consts.rs` caches pi, ln 2, ln 10 and 2/pi at the largest precision a process has
  asked for and returns that value truncated, so an approximation at a fixed precision depends on which lines the
  process evaluated before (the shard split, the order). The error bounds hold either way, so a correctly rounded
  value does not depend on it unless a bound is wrong; but whether Ziv resolves before the cap, and the random lines'
  ulp-error tally (`part(.., 2*prec+40)`, docs/libm-accuracy.tsv), can differ in the last bits. `euler_w` is fixed
  (2048 bits). A stale value arises from any change to mathtools, the toolchain or flags, or a sweep's wrapper,
  that the key misses.
- Cheaper first (no staleness at all): `Nat::isqrt` by division is 55-61 % of a shard; a division-free square root
  (Newton on the reciprocal square root, multiplications only; or Zimmermann, "Karatsuba Square Root", INRIA RR-3805,
  1999) and a fixed-precision ln 10 would cut the oracle itself. Recommended before or beside the value cache.

## Open / next

- RULING NEEDED: order of ORACLE-VALUE-CACHE vs a faster `Nat::isqrt` (recommend the isqrt rewrite first: no stale risk,
  ~half the shard time if the square root gets ~5x cheaper).
- Next step: the coordinator lands this; ORACLE-VALUE-CACHE stacks on it.
