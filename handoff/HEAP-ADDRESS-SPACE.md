# HEAP-ADDRESS-SPACE (GitHub issue #21): handoff

State: branch `task/heap-address-space-10` (continues -9 .. -1); origin/next `d4cbe1667` merged (`3750e5800`, the
brief's "rebase on LAYOUT-RO-STABLE" done as a merge: a rebase would need a force-push). origin/next has moved since
(`305065b07`); not merged in run -10. Build role keke `~/w/r1` (Zen 1, 60 vCPU); scripts in keke
`~/work/heap-address-space-10/` (`verify.sh`, `chain.sh`, `benchcopy.sh`, `soexport.sh`, `kcmp.sh`,
`kcmp-setloc.sh`); trees there: `next` (= `d4cbe1667`), `merge` (= `e76808c9a`), `head` (bench copy of r1).
Spec: `~/state/briefs/HEAP-ADDRESS-SPACE-9.md` "## The change" + `-10.md` (ruling #7028, #7061).
Decisions in force: D-3.19, D-3.14, D-X.103, ruling #6325; pending records `records/pending/HEAP-ADDRESS-SPACE-3-1.toml`
(D-3: growth with use; run -10 added the exact-placement rule), `-6-1.toml` (D-X, head-bin rule). Full run -10
report: handoff/HEAP-ADDRESS-SPACE-10-report.md (earlier: -9-report.md, -8-report.md).

## Done in run -10 (do not redo)

- Fuzz-check red fixed: `ForeignPages` delegates `map_room`/`grow` (`e76808c9a`).
- xtest red of run -9 (RLIMIT_AS test) was a library defect: `map_aligned_exactly` failed with room left when the
  kernel's choice was a gap holding the bytes but no aligned range. Fixed `39acf3bd3` (cold, blocks such a gap with
  one-page PROT_NONE NOREPLACE mappings, all removed before return); deterministic test `88506b099`. verify (lint,
  both tests 20/20 native and interposed) and plants (6/6 CAUGHT) green.
- WINDOW-BIN-BYTES red (fast profile, `tool 3/2`) SETTLED, not this run's fix and not the planner: see the report.
  The merge alone (`e76808c9a`) gives the identical plan (bins 0/1/2 = 63980/58090/60451 bytes) and `tool 3/2`;
  the merge with next's `xtask/src/bins.rs` (run -6's `head_may_take` removed) gives the same again. Cause:
  fast-profile (dev, many codegen units) re-partitioning of niji_boundary when heap.rs changes: the
  `locale,rebind,rebind.pre,tool` group in bin 2 grew +1033 B (`each_block<publish_global>` 696 -> 2083,
  `setlocale_env_kept` outlined 1208); the heap's own hot text net -23 B; no traced mode changed. Release product:
  head = next on every mode (`tool 3/3` both); full-profile hot_text_products 31/31.
- Counts: cbench_startup head vs next: same LOAD count (5 / 5, two read-only each: no pad pair needed). kinstr
  +18.6k in the bench job is noise (direct 300-run perf stat, alternating: next 200699 / 183955, head 183942 /
  181839; syscalls, faults and user instructions equal).

## Open

RULING NEEDED (in the hand-back): accept the fast-profile-only WINDOW-BIN-BYTES red as layout jitter (as ruling
#7028 for the static row), and cut a tooling story to make the fast profile's window gate insensitive to
codegen-unit drift. Until ruled, the story stays red on `cargo xtask test`.

## Exact next step

1. After the ruling: merge current origin/next, sync r1, run `cargo xtask test` (fast products), the full-profile
   pair (`chain.sh` does both), cbench_startup head vs next (`benchcopy.sh`), precheck, hand back.
2. Follow-ups (own stories): HEAP-SPILL-COLD; relro by-section comparison (#6325); returning empty segments'
   address space; fast-profile window gate stability (above).

## Do not redo

The +1 dynamic fault hunt (-8); the fixed-layout #21 check (#6566, -7); the `.got` alignment (dropped -8); the -9
cbench_startup cause (an extra read-only PT_LOAD at a static link's boundary); the -10 RLIMIT_AS xtest cause; the
-10 WINDOW-BIN-BYTES split (head / merge / merge with next's planner / next, above).
