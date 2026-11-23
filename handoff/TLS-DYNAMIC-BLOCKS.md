# TLS-DYNAMIC-BLOCKS: on-demand TLS blocks for dlopened objects without DF_STATIC_TLS (DL-19)

State: branch `task/tls-dynamic-blocks`, code head 57f6c425d, records/handoff head after it (all
pushed). Brief: ~/state/briefs/TLS-DYNAMIC-BLOCKS.md. Role: mei:~/w/s1. Legs: 1 (89 calls), 2
(~115), 3 (~160, budget extended once). Full report: handoff/TLS-DYNAMIC-BLOCKS-report.md.

## Done (do not redo)

* DL-19: objects without `DF_STATIC_TLS` get per-thread blocks on first access, freed at thread
  exit and dlclose (legs 1-2). Leg 3: surplus = `glibc.rtld.optional_static_tls` + 1200 (1712 by
  default), as the reference (owner 2026-11-23 on #8009; differences row removed); TLS descriptors
  into on-demand blocks get an asm fast path (`niji_machine_ld_tlsdesc_ondemand`, 84 -> 38
  instr/access); niji-ld 1100-module test sets the tunable; D-22.6 corrected (coordinator 16:11).
* Records: D-27.94 (amends D-27.39), D-27.95 (amends D-27.49), D-22.6 corrected; docs/tunables.md.
* Jobs on the final code (57f6c425d): gate `tls-dynamic-blocks-gate4` (conformance 148/148; only the
  three mei-environment failures, also in other stories' mei gates), plants `tls-dynamic-blocks-plants4`
  (8 plants; plants file mei:~/work/tls-dynamic-blocks/plants4.txt), counts
  `tls-dynamic-blocks-final-counts`, `tls-dynamic-blocks-base-so` (like-for-like next build),
  start-up checks `tls-dynamic-blocks-startup2`.

## Decisions in force

D-27.94, D-27.95, D-22.6, D-27.39 (amended), D-27.49 (amended).

## Open questions / rulings

* COST: TLS descriptor into a dlopened GD object 16 -> 38 instr/access (no longer static). Fix-forward:
  a shorter descriptor path.
* `__tls_get_addr` fast path 55 vs the reference's 27 (next's, unchanged): INFORMATIONAL.
* IE (`TPOFF`) from one dlopened object into another's on-demand block fails to open: INFORMATIONAL.
* libniji's room is the reference's + 16 bytes (1728 vs 1712): layout slack, INFORMATIONAL.

## Exact next step

If the hand-back is PENDING JOBS: jobwait the named jobs on mei; then precheck on the final head and
`ops handback-check`. Otherwise: landing (coordinator).
