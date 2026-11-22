# ZIV-TEST-ISOLATION: the Ziv tests read this thread's counts

State: branch `task/ziv-test-isolation`, code commit 3ce35974f on origin/next 90390071c (the head with this note is
the branch tip). Test-only change in `crates/math/src/complex.rs` (`mod ziv_tests`); no library code changed.

## Mechanism

CI train 902 failed `complex::ziv_tests::cpow_integer_powers_beyond_the_exact_width` with
`F128 (1 + i·2^-k)^-2 Down: not decided at 384 bits, left: (0, 2), right: (0, 0)`. `far_pow` asserted on
`(n::misses(), residual())`: the first is this thread's count (a `cfg(test)` thread-local in `complex_body.rs`), the
second is `wy::UNDECIDED`, the process-wide count of parts on the 3072-bit residual path. In the same test binary,
`forced_undecided_climbs_each_step_of_the_ladder` (its k = 4 phase and its carg/cpow tail) forces parts onto that
path on another thread. `(0, 2)` = no 384-bit miss on this thread, global residual two parts higher: one forced
two-part call inside far_pow's window. Not rounding-mode state, a cache or dispatch: the result bits matched (the
first assert passed); only the counter pair differed.

The same global reads made three tests pass on another thread's retries (`forced_384_undecided_retries_at_768`,
the ladder test, `carg_and_cpow_retry`: `retries() > before` is satisfied by anyone's retry).

Evidence:
- Deterministic forced overlap: the new `counts_ignore_other_threads` spawns and joins a thread forced up the whole
  ladder inside a measured window. The four global counters grow; this thread's stay put. With the helper reading
  the globals (plants `mine-all-global`, `mine-residual-global`, `mine-384-global`) it fails every run.
- Free-running overlap is rare, as CI saw (first failure in the logs on hand): on the base code, the pair of tests
  passed 40/40 at the default thread count and 40/40 at `--test-threads=1`, and the whole ziv_tests module passed
  100/100 both ways (Skylake-SP host, 48 vCPU). Timing alone almost never puts a forced ladder inside a far_pow
  window, so loops cannot decide this. The deterministic window can.

## The fix

`fn mine() -> [u64; 4]`: this thread's undecided parts at 384/768/1536/3072 bits (`n/w/wx/wy::misses()`). Every
assertion in ziv_tests reads per-thread counts; no expected count changed. Production `step` still reads the global
counters (a no_std library has no thread-locals). Another thread's increment there causes at most one extra
evaluation at the next width, which is still correct (documented at `step`). The tests do not depend on that.

## Checked for the same pattern

- ziv_tests: all 12 non-ignored tests; `tiny_trig_*`, `proven_sites_*`, `inverse_and_sqrt_*`, `run_time_bound_*`,
  `cpow_huge_*`, `constructed_hard_cases_*` and `exact_quotients_*` already read thread-locals; the four above
  now do too. `random_sample_retry_count` (ignored count job) uses thread-locals.
- `crates/math/tests/complex_directed.rs`: global counters, printed only (diagnostics, no assertion). Under parallel
  tests a printed per-test count or "residual path at" line can include another test's parts. Left as is.
- `crates/math/tests/complex_f128_products.rs::accuracy_suite_retry_count` (ignored): asserts on global deltas.
  Isolated as invoked (longsweeps runs it alone, `--exact`); under a bare `--include-ignored` the shards could
  overlap it. Integration tests link the no_std library, so no thread-local exists there.
- `crates/math/src/stats.rs` (`fastpath-stats`): process-wide by design. Its module doc states the rule (a rate
  check uses a name no concurrent test calls, or runs alone). The fast32 rate checks overlap only the `#[ignore]`d
  exhaustive sweeps.
- Elsewhere: a name search found test reads of other process-wide counters: `niji_boundary::catalog::FS_CALLS`
  (core `s07_locale_clauses`) and the regex path `COUNTS` / dfa `FALLBACKS` (core). I found them but did NOT
  inspect them, since they are outside this crate's test binary.

## Report

- Commit 3ce35974f "Count this thread's undecided parts in the Ziv tests".
- Gate (mei, `ziv-test-isolation-gate`, direct tier): 4 suites ok, 751 passed, 0 failed, 450 ignored; peak 2.2G.
- Plants (`ziv-test-isolation-plants`): 3 CAUGHT, 0 SURVIVED, no WARN lines (handoff/ZIV-TEST-ISOLATION-plants.txt).
- 200-run demonstration (`ziv-test-isolation-repro-fix`, the whole ziv_tests module including the overlap pair):
  0 failures of 200 at the default thread count, 0 of 200 at `--test-threads=1`.
- Base runs (`ziv-test-isolation-repro-base`, `-repro-base-module`): 0 failures. That is evidence of how rare the
  timing is, not of the base being correct.
- clippy `--workspace --all-targets --all-features -D warnings` and `cargo xtask static` clean on the role.
- No decision row: this changes only test code (it fixes test isolation; no spec or interface changes).
- INFORMATIONAL: the integration-test notes above (complex_directed prints, the f128 count job under a bare
  `--include-ignored`); FS_CALLS / regex counters not inspected.

## Next step

None for the fix. Possible follow-up: a `std`-only per-thread counter for the integration tests' diagnostics.

Plans to read: D-10.17 (Ziv ladder).
