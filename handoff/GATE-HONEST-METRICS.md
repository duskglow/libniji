# GATE-HONEST-METRICS: start-up gates on causes and ratios to the reference, kernel cost counted

State: branch `task/gate-honest-metrics` (from origin/next 0495c4684), role mei:~/w/s1 (Skylake-SP host). Decision
in force: D-X.165 (owner rulings #8133-#8136, #8144, #8158). Plants: `handoff/GATE-HONEST-METRICS-plants.txt`.

## What changed

1. #8133, `init_data_clusters_hold_at_every_text_phase`: the text pads (`pre`, `cold`) no longer compare fault
   counts. `startup_text_outside_the_hot_group` traces `page_faults` true / locale / tool / fork with the build's
   tracer (`trace_gdb.py`, new `HOT_TEXT_ADDRS` output, `HOT_TEXT_CLASSES=host`, host data files) and names every
   `libc.a` function that ran outside the hot group. Equal counts stay at the `lbss` / `data` pads (+ FORK_BOUND).
   On next's products (Skylake-SP host, no locale-archive) it names `locale::name::candidates<..Files..>` and
   `global_locale::note_failure`; with (2) it reads 0 outside (177 functions traced, 174 of libc.a).
2. #8134: `canon_host.sh` also writes `en_US.utf8` as a per-locale directory (`localedef --no-archive`) and
   `perdir.redirect`; `trace_gdb.py` redirects a path prefix ending in `/`; `regen.sh` traces locale and tool again
   with it and appends the names to those modes' traces (build log: "5 functions the archive trace did not run").
   Effect, Skylake-SP host, fast: static locale 56 -> 53, tool 58 -> 55; dynamic locale 107 -> 103, tool 109 -> 105.
3. #8135: every runtime fault bound of hot_text_products is a ratio to the reference (`reference_probe`: the same
   page_faults.c linked against the host's C library, static or PIE-dynamic; `check_ratio`), thousandths, highest
   ratio of three host classes, no margin. Still report-unless-`NIJI_FAULT_BOUNDS=enforce` (D-X.136). The
   "more than 4 under" reports are gone.
4. #8136: the dynamic probe counts true, locale and tool (`DYN_PROBE_MODES`, `DYN_RATIO`).
5. #8144: `oracle/exec_instr.c` prints `kinstr MIN MEDIAN MAX`; the instruction test reports kernel and
   user + kernel ratios, not gated (spread over 1%, below), and fails if the counter vanishes where
   perf_event_paranoid <= 1. `validation/wl_verdict.py` prints an advisory kernel line (instructions:k geomean,
   highest row, user + kernel geomean); never gates.
6. #8158, `init_dynamic_empty_main_stays_within_its_instruction_bound`: ratio to the reference, ten-thousandths,
   per CPU model plus a default row (no class skipped). Rows = old absolute bound / that host's reference, rounded
   down: as tight as before on each host that had a bound. Why there: the 11 reds were real growth past a 2%
   headroom (user counts identical run to run on every class), so the ratio changes no verdict.

## Measurements (next 0495c4684 fast products; warm-up + 5, every run identical)

| probe (libniji / reference) | Skylake-SP (per-locale dirs) | Haswell-EP, Zen 1 (archive) |
|---|---|---|
| static true / locale / tool | 10/29, 56/53, 58/53 | 10/29, 43/39, 45/39 |
| static fork, localtime(+mktime), float | 5/7, 36/32, 4/1 | same |
| dynamic true / locale / tool | 65/49, 107/81, 109/81 | 65/48, 93/66, 95/66 |
| dyn empty main user instr (median 20) | 170,965 / 101,873 | 170,769 / 98,341 (HSW); 170,771 / 96,012 (Zen 1) |

Kernel instructions, dynamic empty main, 20 runs x 3 series: (max - min) / median up to 10.6% (Skylake-SP),
10.2% (Haswell-EP), 15.4% (Zen 1); the reference up to 20%. Min of 20 steadier (libniji within 0.4% across
series, the reference 1.1%). Median ratio 1.30-1.32. Verdict: over 1%, reported, not gated.

## Open questions / next step

- RULING NEEDED: gate the kernel count on the min of 20 (ratio), which held within 0.4% / 1.1%? Recommend a
  follow-up story to measure min-of-N stability on more programs before gating.
- Release-profile ratios take the fast bounds (release absolute counts were at or under fast); measure release on
  the three classes in a follow-up and tighten.
- Not done: the cross-host run of the whole test on Haswell-EP / Zen 1 (no role there); the counts were measured
  there with the same probe and harness (scratch script), not with this test binary.
- Full report: handoff/reports/GATE-HONEST-METRICS.json.
