# LAYOUT-CANONICAL-SET: the start-up set by rule, the same on every build host, checked against traces

Id: LAYOUT-CANONICAL-SET
Kind: code
Size: M
Model: opus
Effort: high
Waits for: the owner's ruling R3 of `plan/layout-budget-rethink.md`. Independent of the bins; it can land first.

## The change

Design: `plan/layout-budget-rethink.md` section 5. Today the start-up set is what the probes executed on the build
host (trace-at-build), so it follows that host's CPU (IFUNC choice, CPU-class pages), its locale data and its
environment. On 11-14 a 1,640-byte table joined the start-up data only on the CI host, so the CI build needed one
written page more than the developer's build ever showed.

1. Fixed trace environment: `env -i` with fixed variables, locale data from the tree, fixed argv0, cwd and path
   length, `ADDR_NO_RANDOMIZE` (check what LT-4 LAYOUT-TRACE-HOST-CANONICAL already does; keep, do not redo).
2. Declared set = traced set, plus, by rule: every candidate of a traced IFUNC resolver, both pages of each
   CPU-classed pair, and every object a start-up function references RIP-relatively (one level, data only; the
   call closure was measured at 2-4x the traced bytes and rejected, `plan/layout-rethink.md` 3.3; the data
   closure is NOT MEASURED: measure it first).
3. Check on every build: traced set is a subset of the declared set; an extra member is red, naming the function or
   object and the host class (the rule missed a host dependence).
4. Prove host independence: one tree built on the Haswell-EP, Skylake-SP and Zen 1 build host classes gives
   byte-identical `layout-bins.tsv` (or, before the bins, the four generated lists) and `libc.a`.

## Proof

Byte-identical lists across the three host classes (jobs on each); the 11-14 table appears in the declared set on
every host; plants: one rule dropped (the subset check names the missed member on the host that needs it).
