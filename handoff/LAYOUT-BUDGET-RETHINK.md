# LAYOUT-BUDGET-RETHINK handoff

**State:** design story, done. Branch `task/layout-budget-rethink` from origin/next e47a71997; HEAD in
`handoff/reports/LAYOUT-BUDGET-RETHINK.json`. Design: `plan/layout-budget-rethink.md`. Pending record:
`records/pending/LAYOUT-BUDGET-RETHINK-1.toml` (D-NEW, series D-X). Proposed stories, one file each:
`handoff/LAYOUT-BUDGET-RETHINK/{LAYOUT-WINDOW-BINS,LAYOUT-BYTE-BUDGETS,LAYOUT-CANONICAL-SET}.md`.

**Measured (shiki, Haswell-EP, role sh2, job `layout-budget-rethink-sweep`, rc 0, 1,462 s):** both profiles of
next built in the role, products copied to RAM, then 2,394 probe links (nine pad points x 133 pads x 2 profiles),
every mode run as hot_text_products runs it. Scripts: shiki `~/work/LAYOUT-BUDGET-RETHINK/sweep.{sh,py}`; full
per-pad counts and geometry: `~/work/LAYOUT-BUDGET-RETHINK/results.json`; summary lines `SUM`/`GEO`/`BASE` in
`~/logs/layout-budget-rethink-sweep.log`. Pad 0 reproduced the test's counts in both profiles.

**Ops history:** read-only queries of the ops store (scripts in the story scratch, not in the tree): CI verdicts,
trains, events, escalations, runs. Numbers in section 2.1 of the design.

**Decisions in force:** none changed. RULING NEEDED R1-R4 (design section 7).

**Not to redo:** the margins of section 2.2 (deterministic; reuse `results.json`); the history counts.

**Not verified / informational:** (1) after the release build, `cargo xtask layout-check` printed the same two
verdict lines as after the fast build (`budget-fast`, hot group 0x408040-0x49b84a), although the release link
geometry differs. Possibly a cached verdict or a profile read from somewhere else; not investigated. (2) The head bin's
fill and the cold-filler residual are estimates; LAYOUT-WINDOW-BINS measures them first. (3) The margins differ
between build hosts (470 B here, 1,494 B on mei for an older next): not separated into host and revision effects.

**Next step:** the owner rules R1-R4; then LAYOUT-CANONICAL-SET (independent) and LAYOUT-WINDOW-BINS, then
LAYOUT-BYTE-BUDGETS.
