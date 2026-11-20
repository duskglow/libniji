# Hand-off: REVIEW-LOCALE-SLOTS-GROWTH (independent review, planning story)

**State.** Branch `task/review-locale-slots-growth`. HEAD is the commit that adds this note. It is a planning story:
no code changes and no jobs (`JOBS: none`).

**Report.** The full report is `plan/reviews/REVIEW-LOCALE-SLOTS-GROWTH.md`. It has the per-run table, the false
premises, the processes due for a rethink, cost vs gain and the recommendation.

## Findings in short
- **Not a content rabbit hole.** The product work (HL-10 caps removed, `ENOMEM`, hash index) was done and measured in
  runs 1-2 (about 2.1 agent-h). Runs 3-5 (about 2.9 agent-h) and a 50 h hold went on zero-margin start-up layout
  counts. R2 (landed today) and LAYOUT-GOT-ORDER (running) address that.
- **The tripwire's premise is partly false.**
  - One of the 4 reds was a login expiry, which the coordinator requeued as "not on the story".
  - The window had 0 reds and 0 continuations. The tripwire fired one minute after run 6 was admitted, after a 50 h hold.
  - 2 of the 10 "gains" are regressions. The "largest gain" printed in the brief is one of them.
  - The real correctness gain (1100 locales where next stops at 1024; `ENOMEM`) is not counted, because its before is null.
- **Inconsistency.** THREAD-FORCED-UNWIND-3 is held on LAYOUT-GOT-ORDER for the `.got` straddle that ended run 5.
  LOCALE-SLOTS-GROWTH-6 was unheld before that diagnosis and has not been re-held. Its brief also carries an 11-18
  note ("STARTUP-DATA-WINDOWS must pass") that disagrees with the 11-20 note ("R2 is the gate").

## Open questions (coordinator or owner)
- One rule for a residual `.got` straddle in both stories. The recommendation is to hold the landing, not a new run,
  behind LAYOUT-GOT-ORDER, and to fall back to ruling #4347 jitter if it closes.

## Not to redo
- The chain's history and the tripwire arithmetic are reconciled in the report (10 gains = 1 + 6 + 2 + 1 by run;
  5.05 agent-h).

## Next step
- The coordinator answers run 6 (inbox) before it hands back, and acts on the tripwire items (owed #32 plus three new ones).
