# LAYOUT-WINDOW-BINS handoff (after run 3, LAYOUT-WINDOW-BINS-3: slice 2 applied, NOT landable yet)

**State:** branch `task/layout-window-bins-3` (from origin/next 7288d847a), head: see
`handoff/reports/LAYOUT-WINDOW-BINS-3.json`. Role keke:~/w/r5 (Zen 1). Design: `plan/layout-budget-rethink.md` section 3,
**D-X.103**; slice 1 record **D-X.108**. Ruling #5784: three slices, one per run. Slice 2's decision record is NOT
written yet (write it with the numbers once the 0-fault fix below is in: `records new decision D-NEW --series D-X`).

## Slice 2 (APPLY): what is done on this branch

* `xtask/src/bins.rs`: `section()` names `.text.sorted.h9<bin 2d><0 member|1 filler><place 5d>.<f>`;
  `Plan::placements()` gives every member/filler its bin, place and alignment (64 KiB on the first member with
  bytes of every bin after a gap; none on uncounted bins after the last counted one). `HEAD_ANCHOR = false`
  (variant ii) and `HEAD_ROOM = 0x400`: the head bin holds at most 64 KiB - 1 KiB, fillers stop there too.
* `xtask/src/hottext.rs` `group()`: two passes (collect sections, plan, then rename + `--set-section-alignment`);
  `plan_bins()` returns the placements. Removed: `hot-text.clusters`, `hot-text.first-window`, `cluster_align`,
  `head_alignments`, `parse_clusters`, `link_positions` etc. (and the two files, `git rm`). `group()` lost its
  `inputs` argument (so.rs updated). `position_of` reads the 9-digit member form (fillers: `None`).
* `crates/core/tests/hot_text_products.rs`: CLUSTER-WINDOW -> `init_window_bins_start_on_window_boundaries`
  (BIN-WINDOW: counted bins after the head start on 64 KiB and stay in one window; every counted bin of
  `layout-bins.tsv` must be in the link); the text-phase test checks BIN-WINDOW at every pad; FIRST-WINDOW checks
  the head bin (bin 0) ends inside the first window (fstat check gone: it is wherever its signature puts it).
* Plants written, NOT run: `handoff/LAYOUT-WINDOW-BINS-3-plants.txt` (5 unit, 2 product plants).

## Measured (keke, both profiles, `~/work/LAYOUT-WINDOW-BINS-3/counts.sh`: so + hot_text_products --nocapture)

| mode (whole process unless noted) | fast base -> bins | release base -> bins | bound f/r |
|---|---|---|---|
| true | 11 -> 11 | 10 -> 10 | 11 |
| locale (setlocale+printf) | 45 -> 43 | 45 -> 43 | 47 |
| tool | 49 -> 45 | 48 -> 45 | 50 |
| fork child | 5 -> 5 | 5 -> 5 | 5 |
| localtime, localtime+mktime | 40 -> 37 | 41 -> 37 | 40 / 42 |
| float (phase) | 3 -> **4** | 3 -> 3 | 3 |
| strings (phase, must be 0) | 0 -> **1** | 0 -> 0 | 0 |
| mktime, rebind (phase, 0) | 0 -> 0 | 0 -> 0 | 0 |

Windows per mode (planned = applied), fast: float 3 fork 2 locale 2 localtime 3 ltm 3 mktime 3 rebind 4 strings 3
tool 3 true 1 (sum 27, bound 19); release: the same except rebind 4 strings 2 (sum 26, bound 20). Fillers 4,458 B
(fast) / 13,992 B (release), residual 22 / 27 B. `.text` now starts at 0x410000 (the 64 KiB input alignment aligns
the output section; head bin 0x410040..0x41fc40, margin 960 B in the first window). Probe file: fast 5,852,976 B
(= base), release 4,516,528 B (base 4,582,064: -65,536). The whole-count tests for localtime fail only because
the level dropped more than the slack under the bound: tighten LOCALTIME bounds (ratchet) once the rest is green.

## Open (next run, still slice 2): the phase modes

**Likely cause (read from the plan, not traced yet):** signatures are whole-process traces, so the planner cannot
see that `strings` and `float` count only the phase after a warm-up (printf). Fast bin 2's modes lack `locale` and
`true`, so the string functions the measured phase calls probably sit there, a bin the printf warm-up never touches:
+1 fault; float likely the same. Confirm with the fast `layout-bins.tsv` (keys of bin 2 with `strings`). **Fix (recommended):**
regen.sh traces each phase mode's warm-up separately (names before the `f0` marker of page_faults.c, e.g. mode
`strings.pre`), keys.py writes it as its own signature mode, and the planner gets one rule: a section of a phase
mode's measured phase (mode in its signature, warm-up mode not) goes only into a bin its warm-up touches (fresh bin
only when none fits, reported). Then rerun counts.sh, tighten the bounds that dropped (locale 47 -> 43, tool 50 -> 45,
localtime 40/42 -> 37, if all host classes agree), run the plants file, the direct gate, `cargo xtask test` and the
release hot_text_products (brief: start-up checks), write the D-NEW record.

**Not to redo:** the base counts above (origin/next 7288d847a); variant (i) analysis: a static probe's first fault
maps the 64 KiB from its exec segment start, so an anchored head bin would straddle two windows unless .text itself
is 64 KiB aligned, which the bins' alignment already makes it.
**Slice 3 (PROOF, LAYOUT-WINDOW-BINS-4):** pad sweep (shiki `~/work/LAYOUT-BUDGET-RETHINK/sweep.{sh,py}`), file
sizes (libc.so.6 too), head variant numbers.
