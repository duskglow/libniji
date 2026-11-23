# LAYOUT-HEAD-BIN-TAIL: the head window bin no longer depends on which function sorts last

State: branch `task/layout-head-bin-tail`, code commit `78527098f` (on origin/next `0495c4684`); this note and
`handoff/reports/LAYOUT-HEAD-BIN-TAIL.json` in the commit after it. Decision in force: **D-X.162** (record
`records/decisions/D-X/D-X.162.toml`; the train regenerates the view), on top of D-X.103 / D-X.144 (window bins, phase tie-break).

## Cause (reduced, no printf needed)

The head bin is the one window bin not on a 64 KiB boundary (`HEAD_ANCHOR = false`); with more than `HEAD_ROOM`
(0x400) ahead of the group its suffix lies in a second window. Members are linked in list (first-touch) order, so
the measured phases' own sections sort last, and a phase stayed at 0 faults only while some warm-up function
happened to sort after them. next's release head bin: `resolve_fold_slow` (strings phase counts it; the strings
warm-up does not run it) was covered only by printf's `pair_s` (strings warm-up). Planner-level reduction
(scratch evaluator: re-plans the build's `layout-bins.tsv` with a given `bins.rs`, lays the sections out in GNU
ld's name order, counts each page_faults mode's windows at every text pad 0..0xfff0 in 16-byte steps; alignment
taken as 1): removing `pair_s` from the items (what inlining does) gives strings 1 fault at 60 of 4096 pads
(0x2710..0x2ac0). The fast plan of next already gives `float` 2 faults against 1 planned at 97 pads (0x1e30..0x2430,
pad 0x2000 among them; float is not in the text-phase test's modes). The other branch was not checked out (rules:
no checkouts of others' work); the reduction above shows the same mechanism on next's own plan.

## Fix (D-X.162, `xtask/src/bins.rs`)

`exposed()` finds phase modes with a counted head-bin section linked after the last section their warm-up runs.
Only then `head_order()` reorders the head bin: the **cover** (last member every counted phase's warm-up runs;
`getrusage` on both profiles: every warm-up ends with it) stays in place and every later member a phase counts is
linked just ahead of it. Placements keep list positions except where the order differs (strictly increasing).
Membership, modes, planned windows unchanged. `so` prints `so: head bin order: ...; phase modes exposed: ...`.

Options (evaluator, fast / release; "true above plan" = pads where the probe's start-up mode faults more than
planned, base 3517 / 3411; phase modes = float, mktime, rebind, strings):

| option | phase modes above plan (any pad, all 4 plans) | planned windows | `true` above plan |
|---|---|---|---|
| base (next) | float 97 pads (fast); strings 60 pads once `pair_s` is gone | as next | 3517 / 3411 |
| A: cover linked last | none | unchanged | 3799 / 3977 |
| B: groups a phase counts kept out of the head bin | float still exposed | locale 2->3, tool 2->3, fast float 1->2, release strings 0->1 | 3517 / 3411 |
| **A2 (taken)**: phase sections ahead of the cover | none | unchanged | 3619 / 3495 |

Start-up counts at `pre` pads are not a contract (`HEAD_ROOM` doc, init_data_clusters_* skips `pre` for
true/tool/fork); real programs do not run the cover `getrusage`, so their start-up does not move.

## Tests and plants

`bins::tests::a_head_bin_split_costs_no_phase_a_fault_whatever_is_inlined` (reduced case: `pair` removed must
not leave `fold` alone after the split; tries every 16-byte split of the head bin independently of `exposed`;
checks names sort into link order, planned windows unchanged, summary line, the no-cover case). Two existing
tests updated (their head-bin order is the defect pattern). Plants: `handoff/LAYOUT-HEAD-BIN-TAIL-plants.txt`
(old list-order tail put back; cover moved to the end (option A); place = list position).

## Results

All on shiki, role sh2, code head `78527098f`:

- `layout-head-bin-tail-gate` (direct tier; xtask bounded set, FULL is the train's): 6 suites, 627 passed, 0 failed.
- `layout-head-bin-tail-plants`: 3 plants, 3 CAUGHT, no WARN.
- `layout-head-bin-tail-full` (`NIJI_FULL_PROFILE=1 cargo xtask so` + `hot_text_products`): 33 passed; release
  plan `so: head bin order: 4 measured-phase sections linked ahead of ...getrusage; phase modes exposed: none`.
- `layout-head-bin-tail-xtest` (`cargo xtask test`): 7188 + 4878 + 129 + 46 passed; 13 layout contracts hold;
  fast plan: 6 sections ahead of `getrusage`, none exposed.
- Before: `layout-head-bin-tail-ht0` (hot_text_products on next's products, same role): 33 passed.
- Start-up rows: windows/bound per mode identical to next on fast (`float 1/0 fork 2/1 locale 2/2 localtime 2/2
  localtime_mktime 2/2 mktime 0/0 rebind 0/0 strings 0/0 tool 3/2 true 1/1`, residual 111 bytes both). Fast
  `layout-bins.tsv` against next: same 549 rows (same multiset), same bin-0 bytes; only the order of
  `getrusage`, `niji_sys_fstat`, `niji_hot_equal_rest` inside bin 0 differs. Release: next's release plan was not
  built on the role (only fast), so the release before/after is the evaluator's (identical planned windows).
- The scratch evaluator (`~/work/layout-head-bin-tail/` on shiki: `main.rs`, `run.sh`, `bins-{base,A,B,A2}.rs`,
  `eval-*.txt`) produced the option table above.

## LAYOUT-HEAD-BIN-TAIL-2 (continuation, 2026-11-23)

The first run's handback-check failed only on the train-owned view `plan/07-decisions/x-implementation-decisions.md`.
Commit `3263d7cbe` restores that view to the merge base and keeps D-X.162 in its record
(`records/decisions/D-X/D-X.162.toml`). No code changed after `78527098f`. Branch `task/layout-head-bin-tail-2`. On
shiki sh2, at `3263d7cbe`: clippy (`--workspace --all-targets --all-features -D warnings`) and `cargo xtask static`
are clean. `cargo xtask precheck` is GREEN with 34 checks. The four jobs above count, because the tested content
differs from the head only in prose.

## Next step

None for this story if the jobs are green. Open: the evaluator could become an xtask check over the real plan
(every phase mode at its planned count at every pad); today the build prints the exposed modes only.
