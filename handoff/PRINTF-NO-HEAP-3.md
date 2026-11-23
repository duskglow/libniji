# PRINTF-NO-HEAP-3 (handoff; run 1, leg 2)

State: branch `task/printf-no-heap-3` (continues task/printf-no-heap-2 at 629dd2c71; merge base with origin/next
28843dea1, not merged). Code head `32b6f8490`. Role mei:~/w/s2 (Skylake-SP). Decisions in force: D-12.32, D-12.33;
ruling #7808 (restructure, no third attribute pass). Predecessor notes: handoff/PRINTF-NO-HEAP-2.md (its cbench
"before" column is the land bar), handoff/PRINTF-NO-HEAP.md. Plants file: handoff/PRINTF-NO-HEAP-3.plants.

## Commits (this story)
- `d61ef591a` dispatcher split: `format_routed` `#[inline(always)]` (bare `%s` + kept plans), out-of-line
  `format_planned` (plan, lone, pair, render_simple). Measured +25..28 on snprintf rows: fixed by the two below.
- `918114f6c` `vsnprintf` `#[inline(always)]` (it had gone out of line from its C entries: call, six saved
  registers, argument moves = 42 instructions per call); `render_kept` wrapper removed (it cost 37 to save 25 on
  kept multi-conversion stream plans: fprintf du/ls rows +12).
- `98bd3673b` conformance/c/s12_registry.c: a handler registered for a built-in letter (`x`) writes every later
  `%x`, through snprintf and through a memstream that kept the format's plan before the registration (text
  only). Probe-settled: the reference and libniji print the handler's text in both (mei ~/work/printf-no-heap-3/
  redef.c; reg-glibc passes the extended program). Plant nh3-planned-skips-hooks now runs that program.
- `32b6f8490` `vsnprintf` calls `format_routed` directly: `format::<ToBuffer>` had stayed a 15-instruction
  function between the entry and `format_planned`.

## How the cost was found (method worth reusing)
gdb instruction trace of ONE call on libniji's own bench binary (mei ~/work/printf-no-heap-3/trace.py:
`OUT=f FUNC=snprintf FMT=%d HIT=3 gdb -nx -batch -x trace.py --args <cbench_opt>`; FMTREG=rsi for fprintf):
exact per-function instruction counts, base vs branch. snprintf("%d"): base 437; d61ef591a 463; 918114f6c 446.
fprintf du_line (HIT=5): base 756, d61ef591a 768. Base binaries: /mnt/zram/claude/printf-no-heap-2/base.

## Measured (cbench, `cargo xtask bench --compare --counters --runs 3 cbench_opt cbench_stdio`, instr/call)
918114f6c (`printf-no-heap-3-cbench-2`) vs base: s_path 229.2 -> 221.2, s_d_colon 961.5 -> 952.1, ls_line
2422.9 -> 2390.2, chk_ls_nlink 761.5 -> 710.5, mixed 1998.7 -> 1935.5, f2 1111.8 -> 1071.3, g 1287.7 -> 1248.2,
e 1583.9 -> 1543.2, f_large 18478.1 -> 17821.3, fprintf_ls_line 1878.1 -> 1874.0, du_line 842.7 -> 839.2,
fprintf_d 633.7 -> 630.5; ABOVE: snprintf_d 489.6 -> 498.4, ld_large 549.7 -> 558.4, x08 530.8 -> 538.6
(the format::<ToBuffer> layer, fixed in 32b6f8490). Final head: see "Final head" below.

## Final head 32b6f8490
- clippy (`--workspace --all-targets --all-features -D warnings`) and `cargo xtask static` green on the role.
- cbench `printf-no-heap-3-cbench-3` (rc 0), base / now: snprintf_d 489.6/487.4, ld_large 549.7/546.5, x08
  530.8/526.6, s_path 229.2/201.2, s_d_colon 961.5/940.1, ls_line 2422.9/2378.4, chk_ls_nlink 761.5/698.6,
  mixed 1998.7/1923.7, f2 1111.8/1059.4, g 1287.7/1237.3, e 1583.9/1531.4, f_large 18478.1/17813.1,
  fprintf_ls_line 1878.1/1874.2, du_line 842.7/839.3, fprintf_d 633.7/630.5 (base from PRINTF-NO-HEAP-2's
  handoff), fputs_32 125.0, putc 23.2, fwrite_4k 1088.2 (not printf code; 1087.8 on d61ef591a). EVERY printf
  row at or below base. Base counters: /mnt/zram/claude/printf-no-heap-2/base/target/niji/bench/.
- hot_text `printf-no-heap-3-hot-text-3` (full profile): RED, 32 passed, 1 failed:
  `init_data_clusters_hold_at_every_text_phase` (hot_text_products.rs:1720): "tool: fault count moves with the
  text phase": `cold 0x2000` 60, the nine other pads 59. Mode `tool` = setlocale + malloc + printf
  (crates/core/tests/oracle/page_faults.c:2), so printf's text IS on that measured path: not exempt as
  LAYOUT-JITTER without evidence. Same test was red on d61ef591a (leg 1); base 28843dea1 green.
- STILL RUNNING at the end of leg 2 (chain ~/work/printf-no-heap-3/chain3.sh): `printf-no-heap-3-gate-3`
  (direct tier), then `printf-no-heap-3-xtask-test-3`; and `printf-no-heap-3-plants-3` (3 plants, dir
  /mnt/zram/claude/printf-no-heap-3/plants-3). Collect: `~/bin/jobwait printf-no-heap-3-gate-3
  printf-no-heap-3-xtask-test-3 printf-no-heap-3-plants-3`.
- SUPERSEDED: cbench-1, hot-text, plants, gate (leg 1, d61ef591a); cbench-2 (918114f6c: %d/%ld/%x +8..9);
  hot-text-2, plants-2 (98bd3673b, killed: head changed). A leg-1 gate orphan (`cargo xtask affected --run`,
  parent 1) was found still running in s2 and killed with kill-mine.

## Exact next step (leg 3)
1. Collect the three running jobs (command above); fix what they find.
2. hot_text: find the faulting section of mode `tool` at pad `cold 0x2000`, base vs branch, the way leg 1 did for
   strings (adapt ~/work/printf-no-heap-3/diag.sh: link page_faults.c with the test's `cold` pad (see
   hot_text_products.rs near :1690 for how the cold pad is applied), run `tool`, perf record page-faults +
   tools/hottext/faultmap.py). If the extra fault is a printf section (format_planned, render_simple, the C
   entries) crossing a window only at that pad, fix the placement or the code shape (new rare code cold) and
   re-measure cbench rows (all must stay at or below base); if it is the planner's head-bin defect (leg 1),
   hand back INCOMPLETE with the evidence under the open RULING NEEDED, never loosen the test.
3. Then: precheck (`cargo xtask precheck --jobs <labels> --head <sha>`), report json, handback-check.

## RULING NEEDED (from leg 1, still open)
- Layout planner: a section a measured phase runs (strings: `dispatch::resolve_fold_slow`) may sit in the head
  bin's tail, whose window is mapped during the warm-up only by chance (base: printf's out-of-line `pair_s`).
  Recommend a layout story: end the head bin with a section every mode's start-up runs, or keep measured-phase
  sections out of the head bin (xtask/src/bins.rs `head_takes`); then phase modes hold at every pad by construction.

## Follow-ups still open (from PRINTF-NO-HEAP-2)
#7764 (a) grouped/`I` floats past 128 bytes allocate; (b) registered conversions allocate. Not started.

## Not to redo
Cause of the leg-1 hot-text red (pre 0x2000 strings fault; planner, above). The +28 profile (done, fixed).

## Refused
None.
