# PASS2-PRODUCTS-ONCE: proposed by TEST-REDUNDANCY-AUDIT (H1)

Id: PASS2-PRODUCTS-ONCE
Kind: code
Priority: 60
Size: S
Model: opus
Effort: high
Premise: Pass 2 re-runs 58 `*_products` binaries whose subject is a child program pass 2 does not change; 965 test-s of pass 2's 2,839 (median of five CI gates), no pass-2-only failure in 698 gate runs.

## Ruling

Reduces gate coverage: needs the owner's ruling before it lands (recommend yes).

## The change

Run the `*_products` integration targets in pass 1 only, as `PASS0_ONLY` does for the pure-Rust sweeps (`xtask/src/nextest.rs`), keeping hot_text_products, heap_meta_products and timers_products in pass 2 (each has failed in pass 2 only at least once). A unit test pins the rule (a new products target runs once unless listed). Record the decision (pending D-X record, amends the PASS2-NATIVE-RUNNER rationale; cite D-X.36).

## Proof

xtask unit tests for the filter; plants on the filter (a products target leaking into pass 2, a kept one dropped); the next CI gate's pass-2 Summary and test-s against the report's 268 s / 2,839 test-s.

## Context

Read plan/reviews/TEST-REDUNDANCY-AUDIT.md (item H1) first: it has the evidence, the timing sources and the risk.
Measure before/after from the next CI gates' nextest lines (the report's T1 method), not from an estimate.
