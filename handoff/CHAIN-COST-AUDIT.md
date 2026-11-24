# CHAIN-COST-AUDIT handoff

State: branch task/chain-cost-audit, audit only (investigate). Results in `results/chain-cost-audit/README.md`;
the re-runnable query is `tools/chain_cost_audit.py` (opens `~/state/ops.db` read-only on the coordinator host).

Done:
* 689 chains (2026-10-24..11-23, $4,665, 1,757 runs) ranked by cost per realized measured gain; themes table.
* Worst 10 explained with hand-back and red evidence (`evidence.md`): start-up layout bounds moving under the
  branch (LAYOUT-TEXT-PIN 31 stories, TB-FOREIGN 27), shared-file conflicts (LIBM tiers), INCOMPLETE legs, an
  unreachable design kept going (CLASSED-SLOT/NULLFOLD), scope growth (PRINTF-NO-HEAP).
* Tripwire proposed from the data: review when a chain reaches its 5th story, $20, or 8 session legs with no
  landed measured gain (27 fires in 10.7 days, median 16 h after chain start); second-order triggers for
  maintenance-only legs and a test failing in 3+ stories' reds in 7 days.

Decisions in force: none added (no D-row; the tripwire is the coordinator's to build in liella-ops).

Open questions (RULING NEEDED, coordinator): adopt the thresholds (5 stories / $20 / 8 legs); whether a review
holds the chain by default; review of `init_dynamic_empty_main_stays_within_its_instruction_bound` (11 reds in
many stories since 11-16).

Not to redo: the cost-before-any-recorded-gain trigger never fires above $10 (hand-backs record gains easily);
count only gains on landed stories.

Limits: cost before 11-10 estimated from tokens; impact before 11-12 mostly absent (`n/r`); no runs recorded
10-25..11-03; Stacks-on joins neighbouring work into one chain.

Next step: the coordinator builds the tripwire in liella-ops (query logic in the tool's `tripwire.md` section).
Plan rows to read: none.

Report: handoff/reports/CHAIN-COST-AUDIT.json.
