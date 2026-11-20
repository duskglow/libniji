# Hand-off: REVIEW-SL-HEADERS-S07 (independent review, planning story)

**State.** Branch `task/review-sl-headers-s07`. HEAD is the commit that adds this note. It is a planning story: no
code changes and no jobs (`JOBS: none`).

**Scope.** The brief asked for a review of chain SL-HEADERS-S07. A coordinator note at 02:17Z widened the scope to
the whole SL-HEADERS series (the tripwire fired on S07, S08, S10, S11, S12, S16 and S19).

**Report.** The full report is `plan/reviews/REVIEW-SL-HEADERS-S07.md`. It has the per-chain table, the red
categories, the false premises, the processes due for a rethink and the recommendation.

## Findings in short
- **False premise.** The brief says "no measured gain". That text is hard-coded in `ops/reviewer.py`. The database
  holds 5 measured gains for S07 and 35 across the 7 chains. None is delivered, because nothing in these chains has
  landed.
- **Reds are mostly process outcomes.** The 7 tripwired chains have 32 reds:
  - 9 are work failures;
  - 7 are INCOMPLETE hand-backs, which go red by design;
  - 8 are CI conflicts in shared generator files;
  - 5 are wall-time kills;
  - 2 are stale-head verifies;
  - 1 is an id placeholder.
- **S07's hours.** S07's content was done at S07-3 (about 3 h). 69% of its 9.9 h came after that, on integration.
- **Wall-time kills.** Five runs on 11-19 hit the 4 h limit while blocked in `jobwait`: 20 agent-h with no hand-back
  and NULL cost in `runs`. These five are most of the trend alert "wall-time kills 6/day".
- **Deadlock.** The fix in ruling #6748 (data-only slices, HOOKS stories first) is schedule-locked. The HOOKS stories
  have priority 86 and the remaining slices 85-87, while `admit_min_priority` is 93.

## Recommendation
Continue, with named changes:
1. Admit the HOOKS stories one at a time (or park the series explicitly).
2. Start no new slices until the pending slices land; then run at most two slices in parallel.
3. Tooling fixes: the reviewer prints the real gains; the tripwire stops counting INCOMPLETE-by-design reds; a
   deadline-aware jobwait; killed runs' cost is recorded.

## Not done or not verified
- Whether S07's content still applies cleanly to next.
- The red categorisation is my own reading of the `events` reasons.

## Next step
The coordinator or owner decides the RULING NEEDED items in the hand-back. Nothing is left on this branch.
