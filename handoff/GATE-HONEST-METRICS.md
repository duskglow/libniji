# GATE-HONEST-METRICS: start-up gates on causes and ratios to the reference, kernel cost counted

State: fourth leg GATE-HONEST-METRICS-4, branch `task/gate-honest-metrics-4` (leg 3's head e00b91c27, then one
code commit d20f3ccc8, a merge of origin/next 9139a21d9 (0bf8eb08f), the plants file and this note), role
mei:~/w/s1 (Skylake-SP host). Decision in force: D-X.165 (owner rulings #8133-#8136, #8144, #8158, answers
#8197-#8199; #8323 and #8322 of 11-24). Reports: leg 4 `handoff/GATE-HONEST-METRICS-4-report.md`, leg 3
`handoff/GATE-HONEST-METRICS-3-report.md`, leg 2 `handoff/GATE-HONEST-METRICS-2-report.md`, leg 1
`handoff/reports/GATE-HONEST-METRICS.json`. Plants: `handoff/GATE-HONEST-METRICS-4-plants.txt` (leg 4), `-3-`, `-2-`.

## What the branch does (all legs)

1. #8133: `init_data_clusters_hold_at_every_text_phase` gates on the cause. The text pads (`pre`, `cold`) no
   longer compare fault counts; `startup_text_outside_the_hot_group` traces `page_faults` true / locale / tool /
   fork and names every `libc.a` function that ran outside the hot group. Since leg 4 (#8323) it checks ONE locale
   load form, the archive: true and fork (no locale) with the host's own files, locale and tool with the build's
   data files in their archive form (`canon_host.sh`), whatever form the host has. Equal counts stay at the
   `lbss` / `data` pads (+ FORK_BOUND).
2. #8134 (trace the per-locale-directory load path too) is HELD by owner ruling #8323 until the target
   distributions' default form and the archive host's cost are measured. Leg 4 removed it: no per-directory trace
   in regen.sh / runner.sh / canon_host.sh, no prefix redirection in trace_gdb.py.
3. #8135 / #8136: the runtime fault bounds of hot_text_products are ratios to the reference (`reference_probe`).
   They report unless `NIJI_FAULT_BOUNDS=enforce`.
4. #8144: kernel instructions are reported (not gated, #8198); `wl_verdict.py` prints an advisory kernel line.
5. #8158: the dynamic empty-main instruction bound is a ratio to the reference per CPU model.
6. Leg 3: regen.sh lists libc.a's section names whole (`keys.py --sections`, `objdump -h`) and fails on a traced
   function whose section the listing missed (`keys.py --unlisted`). Kept in leg 4: it holds for any long name.
7. #8322: WINDOW-BIN-BYTES stays gated. The ruling makes it report-only in the same change that lands a blocking
   fault-ratio gate; this branch's ratios report unless enforced, so it lands none and the gate is unchanged.

## Leg 4 (train 975 red: release `tool` 3 windows of 2)

Cause: the per-directory trace put its 5 functions (`find_named<Files>` 6,189 B and others, about 8 KB) into
the `locale` and `tool` signatures. Release bin 2 (locale, tool, strings) then had 895 B of headroom on the
branch alone; with the train's other members it overflowed, and tool's own 4.5 KB group went to bin 1 (3
windows). With the archive form only, on the merged head (next 9139a21d9 + branch): release tool 2/2, bin 2
65,281 of 65,536 B (headroom 255), bin 0 63,560 of 64,512. No member of the plan has a section name past the
old 256-byte cut, so the bytes are next's own, not this branch's (see the report's INFORMATIONAL).

## Jobs, leg 4 (mei, role s1; code head d8f77f1a3 = merged head + plants file)

See `handoff/GATE-HONEST-METRICS-4-report.md` and `handoff/reports/GATE-HONEST-METRICS-4.json`.

## Open / next

- The train re-runs the merged head with the other members; release bin 2 headroom is 255 B (next's state).
- Follow-ups (#8198/#8199): min-of-N kernel-instruction stability on more programs and the workload kit;
  release-profile ratios on the three host classes, then tighten.
- #8323's measurement (target distributions' default locale form; the archive host's cost of grouping the
  per-directory path) is owed before #8134 comes back; leg 2/3's per-directory trace is in this branch's history
  (commits before d20f3ccc8) if it does.
- INFORMATIONAL follow-ups (leg 3 report): the v0 walker's leading-zero decimal; `so.rs` `exclude_link_edges`
  still reads `readelf -SW`.
- Not done: the whole test on Haswell-EP / Zen 1 (no role there).
- Do not redo: the #8200 analysis, the train-970 and train-975 cause analyses, the leg-1 measurements.
