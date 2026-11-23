# NSCACHE-MODE-SCOPE: proposed by TEST-REDUNDANCY-AUDIT (H2, H3)

Id: NSCACHE-MODE-SCOPE
Kind: code
Priority: 55
Size: S
Model: opus
Effort: high
Premise: D-16.10 runs every phase 10-11 differential test with the name-service cache on and off; seven programs (aio, catalogs, syslog, fstree, posixipc, mntent, mallocdbg) never reach the cache, which only stores a flag at start and attaches at the first lookup.

## Ruling

Amends D-16.10's test clause: needs the owner's ruling (recommend yes).

## The change

Amend D-16.10 (new record with `amends`): both modes only for a differential program that reaches the cache. Add a static test listing the cache's callers (NSCACHE users) and the lookup families, failing when one of the seven programs starts calling one (it then goes back to both modes). Then drop aio_products' `uring_on` and `eperm_on` (their `_off` twins stay) and run the six in-test doubles in one mode.

## Proof

The static test, red-before-green (add a getpwnam call to a copy of one program); plants on the static test; the next CI gate: aio_products pass 1 286 -> ~191 test-s, the six tests ~-45 test-s.

## Context

Read plan/reviews/TEST-REDUNDANCY-AUDIT.md (item H2, H3) first: it has the evidence, the timing sources and the risk.
Measure before/after from the next CI gates' nextest lines (the report's T1 method), not from an estimate.
