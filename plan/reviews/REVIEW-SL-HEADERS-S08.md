# REVIEW-SL-HEADERS-S08: chain SL-HEADERS-S08 (slice 8): see the series review

This chain is already reviewed in [REVIEW-SL-HEADERS-S07.md](REVIEW-SL-HEADERS-S07.md), which covers the whole
SL-HEADERS series, S08 included. Its verdict stands for S08: the content is not a rabbit hole, the cost is
integration, and the recommendation is to continue with named changes. The series has been parked by ruling #6838
(HEADERS-GEN-S08-HOOKS first, then SL-HEADERS-S08-6 as a data-only rebase, one at a time once the feature pause
lifts; owed #30). On the coordinator's instruction (note of 2026-11-20 02:29Z) this review does not repeat it.

S08-specific points that the series review does not state:

1. **This brief's premise is stale.** The brief says "5 reds" and "measured gain recorded (runs.impact): none".
   `tripwire.chains()` on the live database (read-only, 02:3xZ) gives 5 runs, 5 continuations, **4 reds** (S08's
   INCOMPLETE-by-design red no longer counts) and 9.63 agent-h. It also finds **9 measured gains**, for example pthread.h
   C++ header_check failures 49 -> 0, P2 visibility differences 627 -> 21 -> 5, conformance programs built against
   libniji's headers (P6) 42 -> 45, and plants 0 -> 9. So the brief was generated from the pre-fix figures (#6841/#6842).
2. **The content was finished and verified on 11-18.** SL-HEADERS-S08-4 went `finished -> queued_ci` with all 4 of its
   jobs at rc 0: headers-gen 0 problems, header_check 0 failed of 5382, plants 9 of 9 caught. Its only red was a CI
   merge conflict in `xtask/src/headers.rs` and `docs/differences-from-glibc.md`. Everything after that was
   integration: S08-5 spent 4.0 of the chain's 9.6 agent-h (42%) on a merge-only run and was killed at the wall-time
   limit. It was waiting on a plants job that the S08-6 brief puts at about 100 minutes. After the merge, that run re-ran
   all 9 of the slice's plants, although only three of them had moved (to next's `cond.rs` and `values.rs`).
3. **A killed run's pushed report is lost as well as its cost.** S08-5 pushed `handoff/reports/SL-HEADERS-S08-5.json`
   before it was killed: header_check 0 failed of 6435, M up from 5382, and plants 9 of 9 kept. Its `runs` row still has
   `impact`, `calls` and `tokens` NULL. Owed #29 (recover the cost of killed runs from their transcripts) should also
   ingest the report JSON that the branch already holds, so the run's results are not dropped.

Recommendation: no change to the series review's recommendation. Follow #6838 and owed #30. When it resumes, the
merge-only step for S08-6 should re-run only the plants whose paths changed (series review, process item 6).
