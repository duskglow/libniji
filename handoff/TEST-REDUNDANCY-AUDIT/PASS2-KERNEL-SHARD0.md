# PASS2-KERNEL-SHARD0: proposed by TEST-REDUNDANCY-AUDIT (M1)

Id: PASS2-KERNEL-SHARD0
Kind: code
Priority: 45
Size: S
Model: opus
Effort: high
Premise: strstr_find's needle-length shards and small_compare's build shards run the same niji-boundary code in both passes; pass 2 varies the thread block, heap and locale, which shard length does not; 249 test-s of pass 2, no pass-2-only failure in 698 gate runs.

## Ruling

Reduces gate coverage: needs the owner's ruling (recommend yes, as D-X.36).

## The change

Extend the `PASS1_SHARD0` mechanism (or a sibling list) to run only shard 0 of each strstr_find family (strstr, strcasestr C locale, strcasestr UTF-8 locale) and small_compare's `in_each_build_0` in pass 2; every shard still runs in pass 1. casecmp_avx2 is not included (one unsharded, locale-dependent test).

## Proof

xtask unit tests for the filter terms; plants on them; the next CI gate's strstr_find and small_compare pass-2 test-s.

## Context

Read plan/reviews/TEST-REDUNDANCY-AUDIT.md (item M1) first: it has the evidence, the timing sources and the risk.
Measure before/after from the next CI gates' nextest lines (the report's T1 method), not from an estimate.
