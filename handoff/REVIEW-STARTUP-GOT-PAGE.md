# REVIEW-STARTUP-GOT-PAGE: hand-off

State: branch `task/review-startup-got-page`, cut from origin/next edec37111. Planning story: no code, no jobs
(`JOBS: none`). Full report: `plan/reviews/REVIEW-STARTUP-GOT-PAGE.md`.

## Result
The chain STARTUP-GOT-PAGE is not a design rabbit hole: the design settled in run 4 (option C, ruling #6606).
What is left is landing friction under the zero-margin start-up fault gate. The owner ruled on 11-14 to replace that
gate (D-X.103 / `plan/layout-budget-rethink.md` R2). Its story, LAYOUT-BYTE-BUDGETS-2, is on a manual hold that looks
4 days overdue, with priority cut from 96 to 40. User-visible fault gain: none. Cost: up to 4 KiB of RELRO per program.
The dynamic effect was never measured as a before/after pair.

## Premise corrections
Reds are overcounted: run 3 was an incomplete hand-back by design, and the listed reasons add up to 7. Gains are
overcounted: runs 5 and 6 carried run 4's impact entries forward, the options A and B gains were never delivered,
and a regression (first %.2f 3 -> 4) and a size cost were listed as gains. Agent-hours (15.4) are correct.
STARTUP-GOT-PAGE-7's brief is stale: the bound is now [73, 67], not 86.

## Rulings needed
* Unhold LAYOUT-BYTE-BUDGETS-2 and raise it to at least 93 (recommend yes).
* STARTUP-GOT-PAGE: close after R2 lands, or land only after one paired static + dynamic count on the Skylake-SP
  class (recommend: decide after R2).

## Next step
The coordinator and the owner act on the rulings. There is no developer step.
Plan rows: D-X.101, D-X.103, D-X.122, D-X.133.
