# PRODUCTS-ONE-BINARY: proposed by TEST-REDUNDANCY-AUDIT (M7)

Id: PRODUCTS-ONE-BINARY
Kind: code
Priority: 25
Size: M
Model: opus
Effort: high
Premise: The 71 products targets cost 274 of the 1,092 unit-s of cold test compile+link and 71 pass-2 copies; nextest isolates each test by process whatever the binary count.

## Ruling

Changes the affected tiers' selection unit: a maintainer's ruling on the design.

## The change

After PRODUCTS-HARNESS-SHARED: let the affected tiers select tests by name inside a target, then make the products tests modules of one target. Say what the `cargo test` fallback (threads in one process) does with them.

## Proof

Direct-tier selection unchanged for a sample of products changes (unit tests); cold and incremental build time before/after.

## Context

Read plan/reviews/TEST-REDUNDANCY-AUDIT.md (item M7) first: it has the evidence, the timing sources and the risk.
Measure before/after from the next CI gates' nextest lines (the report's T1 method), not from an estimate.
