# PRODUCTS-HARNESS-SHARED: proposed by TEST-REDUNDANCY-AUDIT (M5)

Id: PRODUCTS-HARNESS-SHARED
Kind: code
Priority: 35
Size: M
Model: opus
Effort: high
Premise: 71 products tests carry 1,531 lines of harness functions: `build` in 58 files (18 variants), `root` in 63, the KNOWN/ADDED/DROPPED filter in 11; a link-flag change touches 58 files.

## Ruling

None (test-support refactor, no test removed).

## The change

One `build_pair` (host vs libniji static link, with per-test libraries and flags) and one known-differences filter in `crates/core/tests/support/oracle.rs`; move the tests to them in batches by crate; no assertion changes.

## Proof

Every moved test green in both passes; plants on the shared helpers (a dropped library, a filter that lets one unrecorded difference through); lines before/after.

## Context

Read plan/reviews/TEST-REDUNDANCY-AUDIT.md (item M5) first: it has the evidence, the timing sources and the risk.
Measure before/after from the next CI gates' nextest lines (the report's T1 method), not from an estimate.
