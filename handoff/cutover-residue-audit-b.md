# CUTOVER-RESIDUE-AUDIT-B (audit only, 2026-11-21)

State: branch `task/cutover-residue-audit-b`, cut from origin/next 43058b7df. No code changed.
JOBS: keke:cutover-residue-audit-b-precheck2 (cargo xtask precheck on the final head). The brief says "JOBS: none",
but it has no `Kind: planning` header, so handback-check rejects none. SUPERSEDED: cutover-residue-audit-b-precheck
(GREEN, 34 checks, on 9a9e79174, before this note and the report record were last edited).
Report (the ranked table, method, not-verified items): `plan/reviews/CUTOVER-RESIDUE-AUDIT-B.md`.

## Result

All 28 stories: **close**. Nothing they did is missing from next.
- 13 branches are 0 commits ahead of `historic/next`. The public root `7374bf3f2` equals `historic/next`
  `76041c218` minus 784 files, all under `handoff/` (checked: `git diff --name-only historic/next 7374bf3f2 | grep -v
  ^handoff/` is empty). So these reached the public tree as they were.
  - Five of them (RPC-SERVER-5, RPC-SERVER-TCP, RPC-REPLY-SOURCE, RPC-XENCRYPT, the DES part of RPC-GETRPC-CORPUS)
    were later replaced by failing stubs on purpose (D-29.4/D-29.5, owner rulings R13 and 10-27,
    LEGACY-RPC-NIS-STUBS-7).
  - COMPAT-12 (RPC-REPLY-SOURCE's flag) is withdrawn.
- 15 branches have commits ahead. Each is an ancestor of its chain's next link, which HELD-REDS-TRIAGE (11-05) found
  CONTAINED or OBSOLETE. The exception is the complex-f128 pair, whose successor ZIV-C-4 exists only in the
  rewritten history; its content was checked directly instead. Added-line containment (liella->niji normalised,
  tree-wide fallback) is 63-100%. Every unmatched group was read and its later form found on next: renames, file
  splits, the hot.rs dispatch, the register split into docs/saturation/, test addresses moved to 192.0.2.x, D-10.101
  (sqrt without an MXCSR read), and STRPTIME-SIGNED-YEAR (#18).
- Compat symbols were checked by readelf on keke `~/wl/products/fdbb6011...-release-dyn/libc.so.6`. That revision
  is from task/heap-class-spacing (cut from next, 11-21), not next itself. Exported: condvar `@GLIBC_2.2.5` +
  `@@GLIBC_2.3.2`, `tr_break`/`__default_morecore` `@`, `__libc_freeres@@`, `__nss_hosts_lookup`/
  `__nss_database_lookup` `@`, and the RPC stubs `@`.
- The libresolv.so.2 compat names were checked in source only: no product on hand, and an audit runs no jobs.

## Do not redo

- The containment scripts (read-only) are in the run's scratch: `~/w/.scratch/CUTOVER-RESIDUE-AUDIT-B/contain2.py`,
  which normalises the rename and falls back to the whole tree, and `fam.py`, which lists family states from ops.db.
  CUTOVER-RESIDUE-AUDIT (A) can reuse the root-equals-historic-next fact: any branch 0 commits ahead of
  `historic/next` is in the public tree.

## Next step

Coordinator: close the 28 (`ops` close with "CUTOVER-RESIDUE-AUDIT-B: contained/obsolete, see
plan/reviews/CUTOVER-RESIDUE-AUDIT-B.md"). MATH-M5-CPLX-F128 and MATH-M5-CPLX-F128-ZIV-A-3 are already closed.
