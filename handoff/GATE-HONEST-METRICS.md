# GATE-HONEST-METRICS: start-up gates on causes and ratios to the reference, kernel cost counted

State: third leg GATE-HONEST-METRICS-3, branch `task/gate-honest-metrics-3` (leg 2's
`task/gate-honest-metrics-2` + 3 commits, no merge; merge base with next 7a0ba4cb2), role mei:~/w/s3
(Skylake-SP host). Decision in force: D-X.165 (owner rulings #8133-#8136, #8144, #8158, answers #8197-#8199).
Reports: leg 3 `handoff/GATE-HONEST-METRICS-3-report.md` (train 970 cause), leg 2
`handoff/GATE-HONEST-METRICS-2-report.md`, leg 1 `handoff/reports/GATE-HONEST-METRICS.json`. Plants:
`handoff/GATE-HONEST-METRICS-3-plants.txt` (leg 3), `handoff/GATE-HONEST-METRICS-2-plants.txt`.

## What the branch does (all legs)

1. #8133: `init_data_clusters_hold_at_every_text_phase` gates on the cause. The text pads (`pre`, `cold`)
   no longer compare fault counts. `startup_text_outside_the_hot_group` traces `page_faults` true / locale /
   tool / fork, and locale and tool again in both locale load forms (`canon_host.sh`'s archive and
   per-locale-directory redirections, #8200). It names every `libc.a` function that ran outside the hot
   group. Equal counts stay at the `lbss` / `data` pads (+ FORK_BOUND).
2. #8134: the build traces locale and tool again from a per-locale directory (regen.sh).
3. #8135 / #8136: the runtime fault bounds of hot_text_products are ratios to the reference
   (`reference_probe`). They report unless `NIJI_FAULT_BOUNDS=enforce`.
4. #8144: kernel instructions are reported (not gated, #8198); `wl_verdict.py` prints an advisory kernel line.
5. #8158: the dynamic empty-main instruction bound is a ratio to the reference per CPU model.
6. Leg 3 (train 970 red): regen.sh lists libc.a's section names whole (`keys.py --sections <archive>`,
   `objdump -h`). Before, it used `readelf -SW`, which cuts names at 256 bytes, so a traced function with a
   long mangled name was never listed. In train 970 that was a 538-byte `try_fold` over a `find_named`
   closure, which the per-directory trace runs. regen.sh also fails now when a traced function of libc.a
   has a section the listing missed (`keys.py --unlisted`), so a drop like this one is an error, never silent.
   Details: leg 3 report.

## Jobs, leg 3 (mei, role s3; final code head b7b37e33f)

- gate-honest-metrics-3-lint0: clippy + static on the inherited head 48adf85a1 (step 0), green.
- gate-honest-metrics-3-gate4: direct tier, 7 suites, 669 passed / 0 failed.
- gate-honest-metrics-3-plants2, gate-honest-metrics-3-xtest2, gate-honest-metrics-3-htp-release2: results in
  the hand-back and `handoff/reports/GATE-HONEST-METRICS-3.json`.
- Foreground `cargo clippy --workspace --all-targets --all-features -- -D warnings` on s3: clean.
- Superseded: gate1 (0610a173c, stopped), gate2 (unsynced tree after a BUSY sync, stopped at once), gate3
  (9e34353e7, green, 668 passed), plants1 (9e34353e7: 2 CAUGHT, 1 SURVIVED, the end-to-end cut plant; the
  regen guard of b7b37e33f answers it), xtest1 (67b95b6b1, stopped for b7b37e33f).

## Open / next

- The train re-runs the merged head; that is the check on b3f3fd771's members together (no merge here).
- Follow-ups (#8198/#8199): min-of-N kernel-instruction stability on more programs and the workload kit;
  release-profile ratios on the three host classes, then tighten.
- INFORMATIONAL follow-ups (leg 3 report): the v0 walker's leading-zero decimal; `so.rs`
  `exclude_link_edges` still reads `readelf -SW`.
- Not done: the whole test on Haswell-EP / Zen 1 (no role there).
- Do not redo: the #8200 analysis, the train-970 cause analysis (leg 3 report), the leg-1 measurements.
