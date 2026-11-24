# PASS2-KERNEL-SHARD0 (TEST-REDUNDANCY-AUDIT M1)

State: branch `task/pass2-kernel-shard0`, cut from origin/next 80d1f809b. Code commit a0b33082c; handoff/report commit on top.

## Premise check
The brief said to extend `PASS1_SHARD0`; PASS2-PRODUCTS-ONCE (landed since the brief) removed it (D-X.161). The premise
still holds (the shards vary needle length or size range, not the thread block, heap or locale), so this adds a sibling
list next to `PASS2_PRODUCTS` instead.

## What changed
- `xtask/src/nextest.rs`: `PASS2_SHARD0` (`ShardFamily` = crate, target, family name, first shard, later shards):
  strstr_find's three families (`m_16_to_31`, `m_32_to_47`, `m_48_to_70` left out; `m_0_to_15` kept) and small_compare's
  `..._in_each_build_1..4` (`_0` kept). `later_shards_term` builds `binary_id(..) & (test(=<full name>) | ..)` per target;
  `pass1_filter` (the second pass of `cargo xtask test`, so CI) adds it; `later_shards_filter` gives affected's native pass
  `-E not (..)` for the selected targets only.
- `xtask/src/affected.rs`: `pass_filter(pass, nextest, c, ts)`: the native pass under nextest only; first pass, interposed
  pass and the `cargo test` fallback run every shard.
- `records/decisions/D-X/D-X.167.toml` (reserved id): amends D-X.161.
- Tests: `nextest::tests::the_second_pass_runs_the_first_shard_only_of_the_listed_kernel_sweeps` (every listed name exists
  in the test source, the first shard never left out, the exact term, pass1_filter carries it, per-target selection),
  `affected::tests::the_native_pass_runs_the_first_shard_only_of_the_listed_kernel_sweeps`; the existing
  `products_targets_run_in_the_second_pass_only_when_listed` expects the new term in the base filter.
- casecmp_avx2 is not included (as the brief says).

## Proof (Zen 1 build host mei, role s3)
- `pass2-kernel-shard0-check`: `cargo xtask clippy`, workspace clippy all-features, `cargo xtask static`, `records check`,
  `cargo test -p xtask --bin xtask`, then `cargo nextest list` of strstr_find + small_compare with the exact pass-2
  expression: the 13 later shards left out, shard 0 of each of the 4 families kept (the `test(=a::b)` syntax checked on
  the real workspace).
- `pass2-kernel-shard0-plants`: handoff/PASS2-KERNEL-SHARD0-plants.txt (9 plants); results under "Results" below.

## Results
- check: rc 0; `cargo test -p xtask --bin xtask` 623 passed; nextest list: 37 tests in the two binaries, 24 kept, the
  13 later shards left out, `m_0_to_15` of the three strstr families and `_in_each_build_0` kept.
- plants: 9 plants: 9 CAUGHT, 0 SURVIVED, 0 WARN (first shard leaking into the list, a misspelled shard, the term dropped
  from pass1_filter, crate or target ignored by the selection, family grouping, affected's native filter dropped or
  applied to the first pass, affected not wired).
- Gate tier: xtask/src is gate tooling (as for PASS2-PRODUCTS-ONCE): FULL: the train's; the changed crate's tests (above)
  and `cargo xtask precheck` ran here.

## Open
- Saving NOT VERIFIED here (ruling's condition): read the next CI gates' second-pass nextest lines (audit T1 method) for
  strstr_find and small_compare after landing; audit estimate -249 test-s of pass 2, ~-24 s.

Next step: land with the train; then read the pass-2 test-s of the two binaries from the first CI gate after landing.
Plan rows: D-X.161, D-X.167; plan/reviews/TEST-REDUNDANCY-AUDIT.md (M1).
