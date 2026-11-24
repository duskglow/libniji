# AFFECTED-TESTS-ONE-LOAD: proposed by TEST-REDUNDANCY-AUDIT (M4)

Id: AFFECTED-TESTS-ONE-LOAD
Kind: code
Priority: 60
Size: M
Model: opus
Effort: high
Premise: xtask's affected::tests share `repo()` through a OnceLock, but nextest runs each test as its own process: ~70 tests each load the whole tree's index (19.7 s alone), 1,403 test-s in the full gate's pass 1; the same 104 tests in one process take 60.5 s wall, ~243 CPU-s.

## Ruling

No coverage change. A cache on disk is a trust question (SLOW AND WORKING WINS): if you choose one, say how it can never serve a stale index.

## The change

Profile `Repo::load` first (where do 19.7 s go; every `cargo xtask affected` pays it too). Then choose: make the load cheap; or turn the selection cases into table-driven tests over one load; or run xtask's unit tests outside nextest in the full gate as the per-fix gate already does. Keep every case's assertion and its failure message naming the case.

## Proof

Before/after of `Repo::load` (instructions or a repeat-count, not wall alone); xtask unit tests green; the next CI gate's affected::tests test-s against 1,403.

## Context

Read plan/reviews/TEST-REDUNDANCY-AUDIT.md (item M4) first: it has the evidence, the timing sources and the risk.
Measure before/after from the next CI gates' nextest lines (the report's T1 method), not from an estimate.
