# PRINTF-NO-HEAP-3 (handoff; run 1, end of leg 1)

State: branch `task/printf-no-heap-3` (continues task/printf-no-heap-2 at 629dd2c71; merge base with origin/next
28843dea1, not merged). Code head `d61ef591a`; plants `ea69122fb`; HEAD = the commit adding this note. Role
mei:~/w/s2 (Skylake-SP). Predecessor notes: handoff/PRINTF-NO-HEAP-2.md (its cbench table is the "before"),
handoff/PRINTF-NO-HEAP.md. Decisions in force: D-12.32, D-12.33; ruling #7808 (restructure, no third attribute pass).

## Done
- Step 0: clippy (`--workspace --all-targets --all-features -D warnings`) and `cargo xtask static` green on the
  inherited 629dd2c71 (foreground, s2).
- Step 1, cause of the red `printf-no-heap-2-hot-text` (`init_zero_fault_calls_hold_at_every_text_phase`, "pre
  0x2000: strings 1 faults"): reproduced with mei `~/work/printf-no-heap-3/diag.sh <tree> <out> <pad>` (links
  page_faults.c with the test's `pre` pad against a tree's products, prints the strings mode's minflt, records the
  faults with `perf record -e page-faults -d` and tools/hottext/faultmap.py). Products: branch = s3's release build
  (06:10), base = /mnt/zram/claude/printf-no-heap-2/base (06:15). pad 0x2000: branch minflt 1 (3 runs), base 0;
  pad 0x800: both 0. The fault list differs in exactly one fault each:
  - base: warm-up fault at `SimpleFormat::pair_s` 0x4207c0 (out of line, signature ...strings.pre);
  - branch: measured-phase fault at `dispatch::resolve_fold_slow` ip 0x41fffd (section 0x41ffc0..0x420369, crosses
    the window boundary 0x420000); `pair_s` is inlined in the branch (no symbol).
  Both plans put `resolve_fold_slow` (signature includes `strings` but not `strings.pre`) near the end of bin 0, the
  head bin, which starts wherever the program's text before the group ends and so straddles two 64 KiB windows
  at some pads. Its second window is faulted during the strings warm-up only if some warm-up section happens to
  lie there: in base that was the incidental out-of-line `pair_s` call. So base passes by that incidental call;
  the defect is the planner's (xtask/src/bins.rs `head_takes`/`head_may_take`: a measured-phase section may join
  the head bin's tail). Bin plans compared: `~/w/s3/target/product/release/layout-bins.tsv` vs the base copy's.
  Not fixed here (layout tool, not this story): RULING NEEDED below. Restoring `pair_s` out of line would only
  hide it.
- Restructure (#7808), `d61ef591a`: `format_routed` is an `#[inline(always)]` dispatcher holding only the bare `%s`
  path and a kept lone plan (`render_lone`); `format_planned` (`#[inline(never)]`: plan, lone, pair, render_plan
  with its four render_simple instances) and `render_kept` (`#[inline(never)]`: a stream's kept plan of several
  conversions) are the out-of-line calls, at the same call depth as the old call to `format_routed`. Why: the old
  `format_routed<ToBuffer>` prologue (6 pushes, `sub $0x138,%rsp`, 5 argument spills) ran for the bare `%s` and
  kept-lone paths too (disassembly mei ~/work/printf-no-heap-2/dis-after-_format_routed___n.s). clippy clean.

## Jobs started on d61ef591a + ea69122fb (sync tree c0e9dbac..., then plants file added)
`~/work/printf-no-heap-3/chain.sh` (detached, in sequence): `printf-no-heap-3-cbench-1` (`cargo xtask bench
--compare --counters --runs 3 cbench_opt cbench_stdio`, the predecessor's command), `printf-no-heap-3-hot-text`
(NIJI_FULL_PROFILE so + hot_text_products), `printf-no-heap-3-gate` (affected --run), `printf-no-heap-3-xtask-test`.
Separately `printf-no-heap-3-plants` (3 plants, handoff/PRINTF-NO-HEAP-3.plants, dir
/mnt/zram/claude/printf-no-heap-3/plants-1).

## Exact next step
1. `~/bin/jobwait printf-no-heap-3-cbench-1 printf-no-heap-3-plants` on mei; read `~/bin/bench-summary
   ~/logs/printf-no-heap-3-cbench-1.log`. Table: before (PRINTF-NO-HEAP-2 handoff "before" column) / prev
   (ef49de906 "after") / now. Land condition (#7808): every printf row at or below before, same host.
2. A row above before: kill the rest of the chain (`~/bin/kill-label <label> --kill`), profile that row (disassemble
   cbench_opt as ~/work/printf-no-heap-2/dis.sh does), fix, re-run. All at or below: let the chain finish.
3. hot-text: if still red, it is the planner defect above (re-run diag.sh on s2's products to confirm the
   faulting section is a measured-phase section in bin 0's second window) -> hand back with RULING NEEDED.
4. Plants: a SURVIVED plant = the new function is untested: add a test (name the clause) and re-run plants.
5. precheck on the final head, report json, handback-check.

## RULING NEEDED
- Layout planner: a section a measured phase runs (strings: `dispatch::resolve_fold_slow`) may sit in the head
  bin's tail, whose window is mapped during the warm-up only by chance (base: printf's out-of-line `pair_s`).
  Recommend a layout story: end the head bin with a section every mode's start-up runs, or keep measured-phase
  sections out of the head bin (bins.rs `head_takes`), then the strings mode holds at every pad by construction.

## Follow-ups still open (from PRINTF-NO-HEAP-2)
#7764 (a) grouped/`I` floats past 128 bytes allocate; (b) registered conversions allocate. Not started.

## Refused
None.
