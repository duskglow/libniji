# REVIEW-GATE-HONEST-METRICS: hand-off

State: branch task/review-gate-honest-metrics, planning story (no code, JOBS: none). Deliverable:
`plan/reviews/REVIEW-GATE-HONEST-METRICS.md` (the full report: chain numbers, false premises, processes, cost vs gain,
recommendation, rulings).

Summary: the chain GATE-HONEST-METRICS (4 members, 6.5 agent-h, $17.35 before -4) is not a rabbit hole of repeated
failures. Each red had a different, real cause, and -3 found a genuine tooling defect: section names were cut at
256 bytes. But:
- its premise ("nothing landed since 06:44Z; this test is a blocker") has been false since 17:43Z on 11-23, and the
  premise was copied unchanged into -2, -3 and -4;
- the train-975 red (tool 3 windows > 2) is not shown to be this branch's. -3's own release build was WINDOW-BIN-BYTES
  ok, tool 2/2 (CI host log `gate-honest-metrics-3-htp-release2.log` lines 225-231). In the train the third window is
  bin 1, a packer reshuffle (train log `ci-gate-c03533f5a-1795488744.log` lines 794-813). Flagged as #8321 while -4 is
  live;
- the tripwire counted an INCOMPLETE-by-design red, and #8157's tripwire is unbuilt (owed #204).

Decisions in force: owner rulings #8133-#8136, #8143, #8144, #8153, #8157, #8158 (liella-ops state/PH14-OWNER-RULINGS.md).
Open questions: the two RULING NEEDED items at the end of the review (WINDOW-BIN-BYTES as a hard gate; #8134 after
the archive restore).
Not to redo: the chain's logs, run costs and both window-gate logs were already read; the numbers are in the review.
Next step: the coordinator acts on #8321 for -4, and takes the two rulings to the owner.
