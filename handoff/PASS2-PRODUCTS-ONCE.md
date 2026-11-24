# PASS2-PRODUCTS-ONCE (TEST-REDUNDANCY-AUDIT H1)

State: branch `task/pass2-products-once`, cut from origin/next 0495c4684. Code commit 0fa78d265; handoff/report commit on top.

## What changed
- `xtask/src/nextest.rs`: `PRODUCTS_SUFFIX`, `PASS2_PRODUCTS` (niji `heap_meta_products`, `hot_text_products`,
  `timers_products`), `products_once`, `first_pass_only`. `pass1_filter` (the second pass of `cargo xtask test`)
  leaves out `binary_id(/_products$/) & not (<kept ids>)`; a run-once id from `unchanged_binaries` that a rule already
  covers is not repeated. `PASS1_SHARD0`, `pass1_shard_filter`, `later_shard_terms`, `LATER_SHARDS` removed: their five
  math products targets now run once (D-X.36 superseded).
- `xtask/src/affected.rs`: `pass_targets(pass, c, ts)` (was inline) uses `first_pass_only`; the shard-0 filter call removed.
- `xtask/src/main.rs`: comment only.
- `records/decisions/D-X/D-X.161.toml` (reserved id): supersedes D-X.36, amends D-X.135.
- Tests: `nextest::tests::products_targets_run_in_the_second_pass_only_when_listed`,
  `affected::tests::the_native_pass_runs_only_the_listed_products_targets`; the removed
  `pass2_runs_shard_0_of_the_heavy_products_targets` tested the removed rule.

## Proof (Haswell build host, role sh1)
- check job `pass2-products-once-check`: `cargo xtask clippy`, `cargo xtask static`, 15 xtask filter tests: rc 0.
- plants `pass2-products-once-plants`: 6 of 6 CAUGHT, no WARN (results: handoff/PASS2-PRODUCTS-ONCE-plants.txt):
  every products target leaking into pass 2, the filter term leaking, a kept target dropped, the run-once dedup,
  `PASS0_ONLY` lost from `first_pass_only`, affected's wiring reverted to `pass0_only`.
- gate `pass2-products-once-gate` (SUPERSEDED): red in `xtask --test transcripts` only (2 iconv tests). Cause: the role's
  `target/niji-test/iconv` dated 07:45 (before this story) had its interpreter `target/niji/ld-linux-x86-64.so.2`
  missing (no `target/niji` on the role), so exec gave ENOENT; this bounded gate does not build the products.
  `pass2-products-once-gate2` = `cargo xtask so` then the same gate: green, 6 suites, 627 passed, peak 2.0G.
- `pass2-products-once-list`: `cargo nextest list` with the filter on the real workspace (the products count and
  which products binaries stay in pass 2); `pass2-products-once-precheck`: `cargo xtask precheck` on the final head.
- Direct tier: "FULL: the train's" (xtask/src/main.rs, nextest.rs are gate commands); the bounded xtask set ran.

## Open
- Saving NOT VERIFIED here: measure on the next CI gates' second-pass nextest `Summary` and test-s (audit T1 method)
  against 268 s / 2,839 test-s; audit estimate -965 test-s, ~-91 s.
- INFORMATIONAL: `xtask/tests/transcripts.rs::iconv_test_link` treats a leftover `target/niji-test/iconv` as built
  even when the products it loads are gone; recommend also checking `target/niji/ld-linux-x86-64.so.2` (skip with a
  reason), or the gate building the products when it selects these tests.

Next step: land with the train; then read pass 2's numbers from the first CI gate after landing.
