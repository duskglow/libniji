# LAYOUT-WINDOW-BINS handoff (checkpoint, run 1)

**State:** branch `task/layout-window-bins` from origin/next f53c74611; this run adds only this note and
`handoff/reports/LAYOUT-WINDOW-BINS.json`. No product, tool, test or bound change yet. Role keke:~/w/r5.
Design: `plan/layout-budget-rethink.md` section 3 (landed; its pending record is now **D-X.103**). Owner rulings R1-R4
granted 2026-11-14 (#5743-#5746).

**Why a checkpoint:** the run's budget (50 calls / 90k context) was spent reading the grouping and trace code
(about 60k context before any change). A Size L story with `Effort: max` cannot finish inside it; the slices below
let a continuation start writing code without re-reading.

## What the code does today (read so the next run does not have to)

* `xtask/src/hottext.rs` `group()` (line ~819): renames each listed function section to
  `.text.sorted.h<pos 5 digits>.<f>` (`section_for`/`rename_for`); with `@place end` in
  `crates/libc-so/hot-text.clusters` each cluster's members become `.text.sorted.h9<cluster 2d><pos 5d>.<f>`
  (`cluster_section`; `position_of` decodes the 8-digit `9...` form). After the renames, lines ~899-934 give each
  cluster head `cluster_align(bytes, min)` (power of two >= the cluster's bytes, <= 64 KiB) through
  `objcopy --set-section-alignment` (`head_alignments`). GNU ld's `SORT(.text.sorted.*)` then orders everything by
  name. This is the code to replace (design items 2, 3, 5).
* Hand-kept inputs: `hot-text.clusters` (strings, time, rebind, rebind-tail, float, keyed by first/last function) and
  `hot-text.first-window` (`liella_sys_fstat` after the start-up tier). Both name internal functions; a bin plan
  derived from signatures makes both redundant (retire them in the slice that applies the plan, after checking that
  `cargo xtask static` and `so` no longer need them).
* The trace: `tools/hottext/regen.sh` traces the `p_*` probes (one names file per traced program,
  `$work/names.<n>`, via `trace_tagged`) and every `page_faults.c` mode through `probe_text.sh`
  (`$work/probe/names.<mode>`); `order.py` keeps only the FIRST tier per key (`<key>\t<tier>\t<profile>`).
  **The signature data exists but is discarded:** a key's signature = the set of `page_faults` modes whose names
  file contains it (the counted probes; the `p_*` probes only order tiers). The lists are in
  `xtask/src/hotlists.rs` `GENERATED` (written under `target/product/<profile>/`).

## Slices (each lands alone; recommend one story each)

1. **SIGNATURES (no layout change):** regen writes `hot-text.signatures` (`<key>\t<mode,mode,...>`) next to the
   list (add to `GENERATED`); `so` runs the planner and writes `target/product/<profile>/layout-bins.tsv`
   (bin, section, bytes, signature; generated) WITHOUT applying it, and prints the planned bins and fill. Unit tests
   for the planner (pure function over (key, size, align, signature)). Measures the two NOT MEASURED numbers first:
   residual fill per bin and the head fill (variant i) per profile.
2. **APPLY:** names `.text.sorted.h9<bin 2d><0|1><pos 5d>.<f>` (0 = member, 1 = filler, so fillers sort after their
   bin and before the next; extend `position_of`), 65536 alignment on each bin's first section (variant (i) also the
   head bin, one constant), fillers best-fit from cold members of the always-linked Rust object; delete the
   `cluster_align` pow2 path. Then hot_text_products both profiles, tighten the bounds that drop, plants of the brief.
   Open question: whether the probe link in `crates/core/tests/hot_text_products.rs` uses `--gc-sections` (fillers
   would then be dropped: fill grows, counts unchanged, the brief's "equivalent" plant).
3. **PROOF:** the pad sweep, reusing shiki `~/work/LAYOUT-BUDGET-RETHINK/sweep.{sh,py}` (nine points x 133 pads x
   2 profiles, ~25 min on Haswell-EP); file sizes before/after; head variant numbers for the owner.

**Planner note:** bins are contiguous runs in name order, so the problem is an ORDER of signature groups plus cut
points (each bin <= 64 KiB after alignment) minimising the sum over modes of bins touched. Order groups so nested
signatures sit adjacent (start-up, all modes, in the head); split a group only at 64 KiB. Deterministic tie-break:
first-touch position.

**Decisions in force:** D-X.63 (clusters, pow2 alignment), D-X.49, D-X.62, D-X.103 (proposal). None changed.
**Not verified:** everything above is read from the code, not measured. **Next step:** cut slice 1 as its own story
with a budget of at least 100 calls / 200k (rules: Effort max), then 2, then 3.
