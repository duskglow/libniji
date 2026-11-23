# TEST-BOUNDS-AUDIT (handoff)

State: branch `task/test-bounds-audit`, base next 0495c4684; the commit adding `results/test-bounds-audit/` and this note
is the head (see `git log -1`). Investigation only: no bound, test, contract or product changed.

Full report: `results/test-bounds-audit/README.md` (inventory, sensitivity measurements, classification,
replacements, answer to the owner's question). Raw counts: `counts-haswell-ep.tsv`, `counts-skylake-sp.tsv`; method:
`measure.py`, `analyse.py` there.

## Findings in force

* `init_data_clusters_hold_at_every_text_phase` is red on **unchanged next** on the Skylake-SP host class (CI's):
  `locale` 56 -> 57, `tool` 58 -> 59 at `cold 0x2000`. That is exactly the 11-23 reds of three unrelated stories. On
  Haswell-EP the same products move at no pad. Cause: cold `.text` (near 0x72d000 / 0x756000) that the locale path
  runs when the host has per-locale directories and no locale-archive; that path is outside the hot group. An
  unrelated glob change makes Skylake-SP pass again (58 at every pad). LAYOUT-JITTER holds on this test are next's
  state, not the held stories'.
* The file system under /tmp changes no count on this head (0 differences, static and dynamic, both host classes).
* Docs, test-only and libm changes leave libc products byte-identical (libm: only libm.a), so they cannot move the counts.
* Runtime fault bounds (non-blocking reports) are stale on both measured classes: tool 45 / 58 vs 44, float 4 vs 3,
  dynamic 65 vs 73.
* Reference comparison: static start-up only 10 vs 29 (libniji better); static tool 45 vs 39 (H) / 58 vs 53 (S), excess
  = anonymous memory (heap size-class first touches, locale data), not layout; dynamic 65/95 vs 48/66 (H).

## RULING NEEDED (recommendations in README section 5-6)

1. Replace the data-phase test's count equality with "every executed start-up text address lies in the hot group"
   (names the function; red at every head, not by chance). Recommend yes.
2. Trace the per-locale-directory load path too, so it is grouped. Recommend yes.
3. Runtime fault metrics as a per-host ratio to the reference (same probe, same run), ratchet the ratio. Recommend yes.
4. Report the dynamic probe's locale/tool modes (largest user-paid gap). Recommend yes.

## Not to redo

The pad sweeps and FS A/B above; plan/layout-budget-rethink.md (11-14) already covers why absolute counts flap.

## Next step

Owner rulings on 1-4; then a story for 1+2 (test change plus trace input), whose proof is: Skylake-SP data-phase
counts equal at every pad without the unrelated glob change, and the new address check red on today's next.
Not measured: Zen 1, release products, the 11-23 `DYN_START_INSTR` red (173,794 > 173,600).
