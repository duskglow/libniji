# MP-LN10-SERIES: the math oracle's ln 10 by series, and its error bound checked (D-10.133)

Branch `task/mp-ln10-series` (from origin/next 8e34f9c72). Host: Skylake-SP build host, role s4. Plants:
`handoff/MP-LN10-SERIES-plants.txt`. Report: `handoff/reports/MP-LN10-SERIES.json`. Scratch scripts and logs:
`~/work/mp-ln10-series/` on the build host (count.sh, eq.sh, eq_patch.py, prof.sh, out/, eqlog/).

## What changed (crates/mathtools only; test tooling, no product)

`consts::compute_ln10` was `log_core(10, bits+8)` (cached ln 2 plus the log1p kernel: square roots, divisions and
full-width products). Now `ln 10 = (10 ln 2 - ln(128/125)) / 3` (2^10 = 1000 · 128/125), `ln(128/125) = 2 atanh(3/253)`
in fixed point (single-limb multiply by 9, divide by 253^2 and 2k+1; 12.8 bits per term, against 6.3 for the brief's
`3 ln 2 + 2 atanh(1/9)` split), ln 2 from the existing cache. `atan_inv` became `atan_frac(a, b, ..)` (a = 1 keeps pi and
ln 2 bit for bit). Proved bound in the comment at `compute_ln10`: relative error < 2^-(bits+6) (ln 2 at bits+8: 0.028 ·
2^-bits after the factor 10; series < 2^-(bits+8); quotient by 3 truncated at bits+8 bits: < 2^-(bits+7); total 0.013 ·
2^-bits). The module comment's series bound was restated (the running power carries 1/(1 - a^2/b^2) units: 1.125 for
1/3, not the 1.1 it said; the 2.1-unit term bound and the result still hold).

## Brief steps

0. complex_products shard 0, instructions:u (whole test process, one test thread, verdict cache off), MP-ISQRT-FAST's
   base a33f92314 (build-only tree, coordinator note on #7336, removed afterwards) vs landed head 8e34f9c72:
   base 44.76 G / 43.42 G (two runs), head 41.65 G / 40.83 G. No regression (about -6 %); the 273 -> 390 s was wall
   noise. Run-to-run spread of this row is about 3 % (the C programs are rebuilt in each run).
1. Both constants against an independent value (scratch: exact Python integer sums of two different splits,
   `3 ln2 + 2 atanh(1/9)` and `(10 ln2 - 2 atanh(3/253))/3`, agreeing, plus Python `decimal` `ln(10)` up to 4400 bits):
   | bits | old log_core, log2 rel err (claim) | series (claim) | old vs new |
   |---|---|---|---|
   | 256 | -267.3 (< -264) | -263.0 (< -262) | -263.0 |
   | 1024 | -1036.3 (< -1032) | -1033.0 (< -1030) | -1033.2 |
   | 4096 | -4107.3 (< -4104) | -4104.6 (< -4102) | -4104.9 |
   | 16384 | -16397.0 (< -16392) | -16392.2 (< -16390) | -16392.2 |
   | 65536 | -65547.8 (< -65544) | -65543.3 (< -65542) | -65543.4 |
   | 131072 | -131084.0 (< -131080) | -131084.0 (< -131078) | identical |
   Both within their stated bounds everywhere: #6922's "one bound is wrong" is NOT reproduced (I could not read
   #6922 itself; if it compared `ln10_w` values cached at different sizes, two values each within 2^-w may differ by
   up to 2^-(w-1), which is not a violation). Time at 65536 bits: 1.94 s old, 0.19 s series.
2. Unit tests `mp::consts::tests` (embedded 4400-bit reference; series and log_core within their bounds at 53..4300
   bits; series vs logarithm at 20000 bits).
3. Equivalence (job `mp-ln10-series-eq-2`): an instrumented copy logs every `correctly_rounded` result; previous
   ln 10 (env switch to the old route) vs series, each binary's full run, one test thread. 9 binaries, 844,812
   results, ordered digests identical, 0 differences, 0 undetermined: complex_products 230906, complex_f128 93836,
   complex_ld_cr 186122, complex_ld 138, complex_directed 147456, elem 3865, elem_ld 5686, elem_f128 7043,
   special_cr 31760.
4. Shard 0, instructions:u, same method as step 0 (jobs `-count-head`, `-count-after`):
   | shard 0 | before (8e34f9c72) | after | change |
   |---|---|---|---|
   | complex_f128 | 165.36 G | 155.04 G | -6.2 % |
   | complex_ld_cr (x87) | 235.52 G | 225.03 G | -4.5 % |
   | complex_products | 41.65 / 40.83 G | 40.81 / 40.81 G | within noise |
5. ORACLE-VALUE-CACHE, re-measured (job `mp-ln10-series-prof`, perf 199 Hz DWARF, ~4300 samples, sampled not
   counted): `correctly_rounded` (the oracle) is 86.3 % (f128) / 87.7 % (x87) of shard 0 inclusive, so a cache
   that served every value would save at most about that (about 134 G of 155 G, 197 G of 225 G).

## Why the gain is 6.2 / 4.5 % and not 11.8 / 8.7 %

After the change `ln10_w` is still 12.0 % (f128) / 9.5 % (x87) inclusive, reached through `round_const`
(`mp::ln10(w)` from `cplx`, Ziv-escalated), and it is now almost all `compute_ln2` (atanh(1/3), 3.2 bits per term;
14.7 % / 11.5 % inclusive). The old route paid that same ln 2 cost (log_core asked for ln 2 at bits+14); only the
kernel was removed.

## Next step

ln 2 by a Machin-like atanh formula with large arguments (published identities such as
`ln 2 = 18 atanh(1/26) - 2 atanh(1/4801) + 8 atanh(1/8749)`, verified numerically before use): about 0.19 terms per bit
against 0.32, so ln 2 (and ln 10 on top of it) about 1.7x cheaper. Measure first with prof.sh; same equivalence run.

Gate `mp-ln10-series-gate` (direct tier): rc 0, 7 suites ok, 1072 passed, 0 failed, 89 ignored, 0 SKIPPED, peak 2.8G.
Lint `mp-ln10-series-lint-4` (clippy workspace all targets all features -D warnings, `cargo xtask static`): rc 0.
Plants `mp-ln10-series-plants`: 3 CAUGHT, 0 WARN. Superseded: `-lint` (my fmt step: the tree is not rustfmt-clean,
not a project check), `-lint-2`/`-lint-3` (clippy expect_used in the test module, fixed; -3 ran before the sync),
`-eq` (killed: its command line named the role and held syncs BUSY), `-diag` (scratch diagnostic).
