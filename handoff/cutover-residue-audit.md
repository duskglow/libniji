# CUTOVER-RESIDUE-AUDIT handoff

State: branch `task/cutover-residue-audit`, cut from origin/next `43058b7df`. Audit only. The full report is
`plan/reviews/CUTOVER-RESIDUE-AUDIT.md` (verdict, method, per-story table, ranked table).

Result: 6 of the 7 stories landed by content. LEGACY-PROBE-COMPAT-2, LEGACY-OBJECTS-2, COMPAT-BEHAVIOUR-8 and
COMPAT-LIBM-3 entered the old history's `next` at `68cd7bafa`. MUTATION-CAMPAIGN-2 and 2B came in with
MUTATION-CAMPAIGN-2C (CI GREEN 10-24). The cutover root `7374bf3f2` equals that `next` minus `handoff/` only.
`readelf` on origin/next `43058b7df`'s products finds every version those branches added: 31 objects with manifest
sizes, 5 LG-2 behaviour entries and 110 libm aliases. COMPAT-BEHAVIOUR-G2 is the only story whose work is not on
next (regexec/nftw/nftw64@GLIBC_2.2.5, quick_exit@GLIBC_2.10), and it is corpus-gated by R14.

Jobs: `cutover-residue-audit-so` on the role's host (`cargo xtask so` on 43058b7df, rc 0), used for the product
check. No gate: no code changed.

Decisions in force: R14 (LG-2 and LG-3 corpus-gated, 11-04). Plan rows LG-0b, LG-2, LG-4, LG-6 and LG-19 are in
`plan/06-phases/18-phase-15-legacy-tier-and.md`.

Not to redo: the ancestry search and the content and product checks. The scripts are in the run's scratch (not in
the tree): `entered.sh` (binary search on the first-parent chain), `lines.py` (share of added lines found) and
`elfcheck.py` (readelf: version, non-default, object size).

Next step (coordinator): close the six as landed, and close COMPAT-BEHAVIOUR-G2, keeping its branch for a
corpus-gated story once LG-19 exists. Optional new stories: the 7 remaining LG-6 objects (`__ctype*`, `_obstack`;
size S, not corpus-gated), and a heap-module cargo-mutants rerun if mutation coverage of the heap is wanted
(size M).
