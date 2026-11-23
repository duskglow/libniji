# STATS-TEST-ISOLATION: the fast-path rate checks read this thread's counts

State: branch `task/stats-test-isolation` on origin/next 5f07429d3; code commits 546df5cc9, 44faca04b (lints),
b54703ae6 (lints); this note and the report are the branch tip. Test-support change only: `crates/math/src/stats.rs`
under `cfg(test)`, plus tests. Products never enable `fastpath-stats`, so no library behaviour changed.

## Mechanism (trains 911 and 912)

The failure came from the test set `niji-math+fastpath-stats`, a plain `cargo test -p niji-math --features
fastpath-stats --lib` (all unit tests in one process, 24 threads), not from the nextest pass (one process per test).
`fast32::tests::fast_path_agrees_with_the_accurate_path` read `stats::counts("erff")`, the process-wide table, before
and after its 131072-operand grid. `fast32::erf::tests::erf_fast_path_agrees_with_the_accurate_path` calls
`eval(Fast::Erf, ..)` in all four modes on another thread, and `eval` records every call (`finish`). The failing counts
`fast 131302 + fallback 19 = 131321` are 249 calls more than the window's own grid: the erf test's calls, some of them
falling back.

- Forced on purpose: `rate_counts_ignore_other_threads` joins a thread making 524288 erff calls (fallbacks included)
  inside a window. With the window reading the process-wide table (plant `counts-global`, the base code's `counts`),
  it fails deterministically (results below).
- Free-running, the base pair alone (`--exact` both tests, one cached build, Zen 1 host, 60 vCPU): 0 failures of 10 at
  the default thread count and 0 of 10 at `--test-threads=1`. Timing alone rarely puts the erf test's calls inside
  the erff window (CI: 2 trains of many); a loop cannot decide it, the forced window can. At `--test-threads=1` no
  two tests run at once, so no other test's call can enter a window.

## The fix

`stats::record` also counts into a thread-local table under `cfg(test)` (the lib's unit tests have `std`), and
`stats::counts` reads it there: a unit test's window sees only its own thread's calls. Outside `cfg(test)` (the
integration tests link the no_std library) `counts` is unchanged, process-wide. No rate, ceiling or operand changed.
The module doc states both cases; the f32 Bessel band check's comment and the rate test's comment say what they rely on.

## Counter-reading tests checked

| test | counter | verdict |
|---|---|---|
| `fast32::tests::fast_path_agrees_with_the_accurate_path` (23 names) | stats | was disturbable (train 911/912); fixed |
| `fast32::tests::atan2_fast_path_*`, `powf_fast_path_*` rate windows | stats | single-thread loops; fixed (per-thread) |
| `fast32::bessel::tests::bessel_fast_path_agrees_*`, `jnf_ynf_low_orders_take_the_large_band` | stats | fixed (the band check could pass on another thread's call) |
| `stats::tests::counts_per_function` | stats | probe names; now per-thread |
| `tests/fast64.rs` exp/log and pow rates | stats (global) | no other test of the binary calls those entries (pow uses `exp_dd`/`log_dd`, unrecorded): OK |
| `tests/fast_special.rs` erf/erfc rate | stats (global) | other tests call the fast tier and `*_accurate` directly, which record nothing: OK |
| `tests/fast_gamma.rs` rate (lgamma, tgamma) | stats (global) | `double_gamma_fast_tier_rounds_correctly{,_at_the_negative_zeros}` call the counted entries: can overlap under plain `cargo test`; NOT fixed (follow-up) |
| `tests/fast_bessel.rs` rate (j0, j1, y0, y1) | stats (global) | `double_bessel_selfcheck_operands_*` calls the counted entries: same; NOT fixed (follow-up) |
| core `s07_locale_clauses` rebind (`FS_CALLS`) | catalog global | read inside `in_child`: a re-executed child process running only that test: OK |
| core regex dfa `BUILT`/`FLUSHES`/`FALLBACKS` | thread-local | OK |
| boundary `tests/regex_paths.rs` (regex `paths::COUNTS`) | global | the binary's only test: OK |

The two integration overlaps cannot fail the gate or CI today (nextest runs each integration test in its own
process; the only plain `cargo test` set over niji-math is `--lib`), but `cargo test -p niji-math` can. Follow-up:
make those two rate windows exclusive within their binaries (an `RwLock` held for writing by the window and for
reading by the entry-calling tests), with a forced-overlap test and plants.

## Proof

All on keke role n1 (Zen 1 host, 60 vCPU), final code head b54703ae6:
- Gate `stats-test-isolation-gate` (direct tier, `--base origin/next`): 7 suites ok, 1 skipped, 2075 passed, 0
  failed, 520 ignored; the `niji-math+fastpath-stats` test set (the one trains 911/912 failed) ok; peak 2.1G.
- Plants `stats-test-isolation-plants`: 2 CAUGHT, 0 SURVIVED, no WARN lines (handoff/STATS-TEST-ISOLATION-plants.txt):
  `counts-global` (the base code's process-wide `counts`) caught by `rate_counts_ignore_other_threads` with "another
  thread's erff calls counted in this thread's rate window"; `record-not-mine` caught by `stats::tests::counts_per_function`.
- Demonstration `stats-test-isolation-demo` (one cached build): forced overlap 0 failures of 200 in a row; the free
  train-911 pair 0 of 10 at the default thread count, 0 of 10 at `--test-threads=1`.
- Base runs `stats-test-isolation-base-overlap-2`: 0 of 10 and 0 of 10 (evidence of how rare the timing is, not of
  the base being right). `stats-test-isolation-base-overlap` was stopped by me after 2 runs (its cwd held the role).
- `stats-test-isolation-lint-6`: clippy `--workspace --all-targets --all-features -D warnings` and `cargo xtask static`
  clean. Earlier lint runs -2..-5 failed (a `cargo fmt --check` I wrongly added: the tree is not rustfmt-formatted; then
  clippy's arithmetic/indexing/unwrap/panic lints, fixed in 44faca04b and b54703ae6).

## Next step

The follow-up above (fast_gamma, fast_bessel integration binaries). Plans to read: none (no decision row: test
isolation only, no spec or interface change).
