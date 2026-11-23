# DECISION-EVIDENCE-AUDIT (investigate; no product change)

**State.** Branch `task/decision-evidence-audit` from `next` 0495c4684. Finished. Deliverable:
`results/decision-audit/README.md` (counts, ranked table, performance links); filter
`tools/decision_evidence_audit.py` (re-runnable, writes `results/decision-audit/items.tsv`); probes in
`results/decision-audit/probes/`. Report json: `handoff/reports/DECISION-EVIDENCE-AUDIT.json`.

**Counts.** 1,474 items (781 records, 384 differences rows, 309 rulings entries); 1,008 flagged (set a value or
depart); 454 cite a measurement, 367 a spec clause only, 187 nothing. Class a (everyone pays) 381, of which 114
unmeasured.

**Findings (probes on the shiki role sh1, Haswell-EP, release products of next 0495c4684, dynamic kit):**
1. THREAD-CREATE-COST cause: `KeyTable::span()` (`crates/rt/src/thread.rs`) scans all 4096 key sequence words
   (`KEYS = 4096`, D-22.5, no recorded reason) on every cached-stack reuse and every thread exit. perf annotate:
   about 34,700 of 36,026 instructions per create+join (reference 2,274). Fix proposal: high-water mark of created
   keys (O(1)); then a 1,024-entry key area with growth. Internal, no ruling.
2. Static TLS surplus 64 KiB (R14-4, #8009): D-22.6 says "sized by tunable" but `ld::tls::SURPLUS` is a const and
   `glibc.rtld.optional_static_tls` is parsed but unused; a reused thread stack zeroes the surplus each time.
   RULING NEEDED (owner ruling R14-4).
3. D-X.138's start-up data budget charges the `RESERVE_MEM` `.lbss` page: 0 faults measured in a static release
   process (probe 3). Proposal: charge a page only if the trace faults it.
4. Thread mapping 8,396 KiB vs 8,192; TP 139,200 B below the mapping end vs 2,368 (about 73 KiB unattributed).
5. Rulings file housekeeping: RPC 4 MiB cap (#1466) is moot (RPC stubs, D-29.5); #1317 already reversed.

**Not done.** Class b/c items and most class-a items are listed in `items.tsv` but not judged one by one. The
dynamic start-up gap (one-fopen program: 201k vs 139k instructions, 76 vs 57 faults) is not linked to any
assumption.

**Next step.** Brief a story for finding 1 (watermark; measure on the pthread create+join percall row); feed
finding 2 into #8009's story; brief the budget-model check for finding 3 (both profiles, every `.lbss`/`.bss` member).

**Do not redo.** The thread-create profile (probes/run.sh, run2.sh) and the RESERVE_MEM fault check (run3.sh).
