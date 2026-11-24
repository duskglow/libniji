# Handoff: COORD-PROCESS-REVIEW (independent review of the coordinator's process)

State: branch task/coord-process-review, cut from origin/next. Read-only review; changes only
results/coord-process-review/README.md (the full report), this note and handoff/reports/COORD-PROCESS-REVIEW.json.
No code, records or decisions changed.

Decisions in force: none made here. The review reads the owner rulings #8009 (decision principles), #7769, #7365 and
GOALS.md as of 2026-11-23.

Findings (details and citations in the README):
- P1: infrastructure changes have no change control (locales-all incident; gate sudo env; in-place watch edit).
- P2: next stays red with no automatic stop; 59 manual red->finished overrides 11-20..23 (34 on 11-23); trains
  946-955 red, 93 verdicts carried on 11-23.
- P3: dependencies and holds are kept by hand; 14 CI-conflict continuations; continuations are 29-44% of runs.
- P4: 11 coordinator premises were found false mid-run in 4 days; the owner caught both unevidenced defaults.
- P5: structural gains came from owner questions (STARTUP-STRUCTURE-REVIEW, -56% empty main); layout chain was patches.
- P6: overload: ~60 submissions/day, 566-1,169 manual ops events/day, 100 owed open, 78 overdue.

RULING NEEDED (owner): adopt the ranked recommendations 1-3 (automatic stop-the-line canary, infra change control,
machine-checked explained-red overrides). Recommendations 4-8 are the coordinator's to build or decline.

Not done / not verified: release-step failure cause (RELEASE-STEP-EXEC-FAIL owns it); escalation and alert tables
not read; patterns before 11-20 not checked.

Next step: the coordinator answers the recommendations (accept / decline with reason) and turns accepted ones into
ops owe items or stories. The owner is considering a review by another model class after the 11-23 audits.

Scratch: the coordinator container's ~/w/.scratch/COORD-PROCESS-REVIEW/work (read-only query scripts).
