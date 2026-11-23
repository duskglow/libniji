# GATE-HONEST-METRICS: start-up gates on causes and ratios to the reference, kernel cost counted

State: second leg GATE-HONEST-METRICS-2, branch `task/gate-honest-metrics-2` (the first leg's
`task/gate-honest-metrics` plus a merge of origin/next 7a0ba4cb2, ruling #8197), role mei:~/w/s1 (Skylake-SP
host). Decision in force: D-X.165 (owner rulings #8133-#8136, #8144, #8158; #8198/#8199 answered: kernel count
reported, not gated; release ratios take the fast bounds; a follow-up story measures both). Plants:
`handoff/GATE-HONEST-METRICS-2-plants.txt` (3 CAUGHT). Full report: `handoff/GATE-HONEST-METRICS-2-report.md`.

## What the branch does (both legs)

1. #8133: `init_data_clusters_hold_at_every_text_phase` gates on the cause. The text pads (`pre`, `cold`) no longer
   compare fault counts. `startup_text_outside_the_hot_group` traces `page_faults` true / locale / tool / fork
   with the build's tracer (`HOT_TEXT_ADDRS`) and names every `libc.a` function that ran outside the hot group.
   Equal counts stay at the `lbss` / `data` pads (+ FORK_BOUND).
   Leg 2 (#8200): it also traces locale and tool with `canon_host.sh`'s two redirections (archive and per-locale
   directory). The host's own files run only one load form.
2. #8134: the build traces locale and tool again from a per-locale directory (`canon_host.sh` perdir.redirect,
   `trace_gdb.py` prefix redirection, `regen.sh` appends the names).
3. #8135 / #8136: runtime fault bounds of hot_text_products are ratios to the reference (`reference_probe`, the
   same `page_faults.c` against the host's C library, through the same `faults()` harness), thousandths, highest
   of three host classes. Dynamic probe counts true, locale, tool. Report unless `NIJI_FAULT_BOUNDS=enforce`.
4. #8144: `exec_instr.c` prints kernel instructions; the instruction test reports kernel and user+kernel ratios
   (not gated, #8198); `wl_verdict.py` prints an advisory kernel line.
5. #8158: dynamic empty-main instruction bound is a ratio to the reference per CPU model.

## #8200: why the first plant survived (settled)

The perdir-trace-dropped mutant really lacked the four per-directory keys in `hot-text.list` (diff against the
branch build: `candidates<..Files..>`, `note_failure`, `equal_rest`, `niji_hot_equal_big`). Nothing else grouped
them. But mei gained `/usr/lib/locale/locale-archive` at 18:10:18 on 11-23, just before that plant run. The check traced
with the host's own files, so it took the archive path and never ran those functions. Fix: commit 57aac469c (item 1,
leg 2). Proof: the same plant is now CAUGHT on that host (archive present). It names `locale (per-locale
directory)`: `locale::in_dir<Files>`, `name::candidates<..>`, `global_locale::note_failure` (names as of
next's LOCALE-PER-DIR-LOAD). Cost: the test takes 76 s instead of 48 s (4 more traces + canon data).

## Merge notes

Next's LOCALE-PER-DIR-LOAD gives the fault probes their own en_US archive, bind-mounted in a user and mount
namespace (`probe_locale_archive`). The reference probe runs through the same `faults()`, so both sides of each
ratio count the same locale layout. On the Skylake-SP host the per-locale-directory fault numbers of leg 1
(e.g. static tool 58/53) no longer occur in the fault probes. The bounds (highest ratio of three classes) are
unchanged and hold.

## Jobs, leg 2 (mei, role s1; final code head 57aac469c)

- gate-honest-metrics-2-lint0: clippy + static on the inherited head ee996a26c, green (step 0).
- gate-honest-metrics-2-lint1: clippy + static + affected list on 57aac469c, green.
- gate-honest-metrics-2-gate1: direct tier, 7 suites, 660 passed / 0 failed.
- gate-honest-metrics-2-plants1: 3 plants, 3 CAUGHT, no WARN.
- gate-honest-metrics-2-xtest1: `cargo xtask so && cargo xtask test`: see the report.
- gate-honest-metrics-2-htp-release1: release products + hot_text_products: see the report.
- gate-honest-metrics-2-lint2: clippy + static + `cargo test -p xtask` on the final head (record edit).

## Open / next

- Follow-up story (#8198/#8199): min-of-N kernel-instruction stability on more programs and the workload kit;
  release-profile ratios on the three host classes, then tighten.
- Not done: the whole test on Haswell-EP / Zen 1 (no role there).
- Do not redo: the #8200 analysis above; the leg-1 measurements (report of leg 1:
  `handoff/reports/GATE-HONEST-METRICS.json`).
