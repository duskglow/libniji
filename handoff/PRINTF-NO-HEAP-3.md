# PRINTF-NO-HEAP-3 (handoff; run 1, end of leg 1)

State: branch `task/printf-no-heap-3` (continues task/printf-no-heap-2 at 629dd2c71; merge base with origin/next
28843dea1, not merged). Commits: `d61ef591a` dispatcher split (MEASURED WORSE, see below: rework or revert before
hand-back), `ea69122fb` plants file. Role mei:~/w/s2 (Skylake-SP). Predecessor notes: handoff/PRINTF-NO-HEAP-2.md
(its cbench "before" column is the land bar), handoff/PRINTF-NO-HEAP.md. Decisions in force: D-12.32, D-12.33;
ruling #7808 (restructure, no third attribute pass). Nothing of this story runs on mei (chain killed, PID-checked).

## Done
- Step 0: clippy (`--workspace --all-targets --all-features -D warnings`) and `cargo xtask static` green on 629dd2c71.
- Step 1, cause of the red `printf-no-heap-2-hot-text` (`init_zero_fault_calls_hold_at_every_text_phase`, "pre
  0x2000: strings 1 faults"), probe-settled with mei `~/work/printf-no-heap-3/diag.sh <tree> <out> <pad>` (links
  page_faults.c with the test's `pre` pad against a tree's products, prints the strings minflt, records faults with
  `perf record -e page-faults -d` + tools/hottext/faultmap.py). pad 0x2000: branch 629dd2c71 minflt 1 (3 of 3), base
  0; pad 0x800 both 0. One fault differs: base faults at `SimpleFormat::pair_s` 0x4207c0 in the WARM-UP (out of
  line); the branch (pair_s inlined) faults at `dispatch::resolve_fold_slow` ip 0x41fffd in the MEASURED phase
  (section 0x41ffc0..0x420369 crosses window 0x420000). `resolve_fold_slow` (signature has `strings`, not
  `strings.pre`) sits near the end of bin 0, the head bin, which straddles two windows at some pads; its second
  window is mapped in the warm-up only if some warm-up section happens to lie there (base: the incidental `pair_s`).
  The defect is the planner's (xtask/src/bins.rs `head_takes`/`head_may_take`): RULING NEEDED below.
- Dispatcher split `d61ef591a`: `format_routed` `#[inline(always)]` (bare `%s` + kept lone plan), out of line
  `format_planned` (plan, lone, pair, render_plan) and `render_kept` (kept multi-conversion plan).

## Measured on d61ef591a (`printf-no-heap-3-cbench-1`, same command as PRINTF-NO-HEAP-2: `cargo xtask bench
--compare --counters --runs 3 cbench_opt cbench_stdio`), instructions per call, before / prev ef49de906 / now:
snprintf_d 489.5/487.5/**515.5**; ld_large 549.4/547.5/**575.5**; x08 530.5/527.6/**555.6**; s_path
229.2/231.2/**236.2**; s_d_colon 961.0/941.2/**969.2**; ls_line 2420.9/2379.4/2407; chk_ls_nlink 760.6/699.6/727.6;
mixed 1997.6/1924.7/1952.6; f2 1110.3/1060.3/1088.3; g 1286.3/1237.3/1265.2; e 1582.3/1532.3/1560.2;
fprintf_ls_line 1877.2/1877.2/**1889.1**; du_line 842.4/842.3/**854.2**; stdio fprintf_d 633.7/639.5/632.8 (now
below base); fputs_32 125.0, putc 23.2, fwrite_4k 1087.8 (unchanged). So every ToBuffer row +25..28 vs prev, kept
multi-conversion stream rows +12: WORSE. Symbols in s2's cbench_opt: `format::<ToBuffer>` inlined (no symbol);
`format_planned::<ToBuffer>` 0x9579 B (render_simple still inlined); `format_planned::<ToStream>` 0x183d B and
`render_kept::<ToStream>` 0x20e B (render_simple NOT inlined there: separate calls). Cause of the +28 not yet seen:
disassemble `__niji_libc_vsnprintf` and `format_planned::<ToBuffer>` (prologue, frame) against the predecessor's
`~/work/printf-no-heap-2/dis-after-*.s` (script `~/work/printf-no-heap-2/dis.sh`, adapt paths to s2).
- Hot text on d61ef591a (`printf-no-heap-3-hot-text`): 32 passed, 1 failed, now
  `init_data_clusters_hold_at_every_text_phase` (hot_text_products.rs:1720); the strings test passed. Layout moves
  with every printf restructure; re-check on the final head.
- Plants on d61ef591a (`printf-no-heap-3-plants`): 2 CAUGHT (nh3-kept-count by s12_stdio_symbols
  sym__IO_fprintf, nh3-bare-s-count), 1 SURVIVED: `nh3-planned-skips-hooks` (no s12_stdio_symbols test registers a
  conversion for a letter the simple path takes, e.g. `d` or `s`, then prints through snprintf). Add that test
  (handler registered for `d`: snprintf("%d") must call it; spec: register_printf_specifier clause) or point the
  plant at the test that has it (conformance/c/s12_registry.c may: check), then re-run plants on the final head.
- SUPERSEDED: printf-no-heap-3-gate (killed: head to change), the three jobs above (d61ef591a, to be reworked).

## Exact next step
1. Profile the +28 (disassembly above) before changing code. Likely shapes to test: keep ToBuffer exactly as prev
   (format_routed<ToBuffer> one function, kept=None) and split only the stream side (kept lone inline in to_stream,
   render_kept out of line), since fprintf_d gained there; or move only the bare `%s` check ahead of the call.
   Keep only what measures at or below "before" on every row; else revert d61ef591a (`git revert`, a new commit).
2. Re-run: cbench (same command), hot_text (full profile), plants, gate, `cargo xtask test`, precheck, report json,
   handback-check. A row that cannot reach before: hand back with the per-row table (owner decision, #7808).

## RULING NEEDED
- Layout planner: a section a measured phase runs (strings: `dispatch::resolve_fold_slow`) may sit in the head
  bin's tail, whose window is mapped during the warm-up only by chance (base: printf's out-of-line `pair_s`).
  Recommend a layout story: end the head bin with a section every mode's start-up runs, or keep measured-phase
  sections out of the head bin (bins.rs `head_takes`); then phase modes hold at every pad by construction.

## Follow-ups still open (from PRINTF-NO-HEAP-2)
#7764 (a) grouped/`I` floats past 128 bytes allocate; (b) registered conversions allocate. Not started.

## Refused
None.
