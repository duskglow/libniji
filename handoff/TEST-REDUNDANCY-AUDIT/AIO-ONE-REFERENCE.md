# AIO-ONE-REFERENCE: proposed by TEST-REDUNDANCY-AUDIT (M2)

Id: AIO-ONE-REFERENCE
Kind: code
Priority: 40
Size: S
Model: opus
Effort: high
Premise: Each aio_products variant rebuilds aio_diff.c twice and re-runs the identical reference; six variants, 286 test-s per pass.

## Ruling

None if every comparison is kept; say so in the hand-back.

## The change

First measure one variant's split (builds, reference run, libniji run). Then one test that builds once, runs the reference once and compares each libniji configuration in turn (each failure still names its engine and mode), unless the split shows the reference is a small share (then report that and stop).

## Proof

The split measured; plants: a planted difference in one configuration's run is still reported with that configuration's name; before/after test-s.

## Context

Read plan/reviews/TEST-REDUNDANCY-AUDIT.md (item M2) first: it has the evidence, the timing sources and the risk.
Measure before/after from the next CI gates' nextest lines (the report's T1 method), not from an estimate.
