# LAYOUT-WINDOW-BINS handoff (after run 2, LAYOUT-WINDOW-BINS-2: slice 1 done)

**State:** branch `task/layout-window-bins-2` (from `task/layout-window-bins`, which is next f53c74611 plus run 1's
notes). Head: see `handoff/reports/LAYOUT-WINDOW-BINS-2.json`. Role keke:~/w/r5 (Zen 1, 60 vCPU). Design:
`plan/layout-budget-rethink.md` section 3, **D-X.103**; this slice's record: `records/pending/LAYOUT-WINDOW-BINS-2-1.toml`
(D-NEW, series D-X; CI assigns the id). Coordinator ruling #5784: three slices, one per run.

## Slice 1 (SIGNATURES, done; no layout change)

* `tools/hottext/regen.sh` records each counted `page_faults` mode's names file (`$work/modes`: the `./page_faults <m>`
  traces, `true` = the start-up trace, probe_text.sh's `localtime`/`localtime_mktime`) and writes
  `$L/hot-text.signatures` (`<key>\t<modes,sorted|->`, list order) via `keys.py --signatures <list> --alias <file>
  <mode>=<names>...`. Added to `hotlists::GENERATED` (so `so` starts it empty and the products' marker hashes it).
* `xtask/src/bins.rs`: the planner (pure; 7 unit tests) + `layout-bins.tsv` writer + summary. `hottext::group()` collects
  the other function sections as filler candidates and, under `Layout::Traced`, calls `plan_bins` after the renames:
  it writes `target/product/<profile>/layout-bins.tsv` and prints `so: window bin N: ...` lines and one
  `so: window bins (planned, not applied): ... windows/bound per mode: ...` line. Refuses signatures in which the
  start-up mode runs no grouped key.
* Planner: signature groups widest first; whole group into the existing bin it adds the fewest modes to, else fresh
  bins split only when full; bins no counted mode touches come last, no boundary, no fillers; gaps best-fit from cold
  sections of members holding a start-up section ("always linked").

## Measured (Zen 1 build host; numbers from the first build of this run, before the no-filler rule for uncounted bins)

* Signatures: fast 518 keys (234 `-`, 90 all-modes = 59,074 bytes), release 461 (201 `-`, 91 all-modes = 58,495).
  Counted bytes fast 244,564 (3.7 windows), uncounted 280,759.
* Plan: fast 9 bins, windows per mode `float 3/2 fork 2/1 locale 2/2 localtime 3/2 localtime_mktime 3/2 mktime 3/2
  rebind 4/3 strings 2/2 tool 3/2 true 1/1` (planned/bound; sum 26/19); release 10 bins, the same except rebind 4/4
  (sum 27/20). Residual fill: 60 bytes (fast) and 48 bytes (release) in total over all gaps: **the cold pool fills the
  gaps almost exactly** (6,719 / 3,627 candidates from 15 members).
* Final head (no fillers for bins no mode touches; keke job layout-window-bins-2-startup4): fast 9 bins, fillers
  4,714 bytes, **residual 42 bytes**; release 10 bins, fillers 15,578 bytes, **residual 10 bytes**; windows per mode
  unchanged from above. Release bins 5-9 (about 295 KB) and fast bins 5-8 hold sections no counted mode runs.
* **Head fill, variant (i)** (page_faults linked as hot_text_products links it; `headfill.sh` in
  keke:~/work/LAYOUT-WINDOW-BINS-2): the group starts 64 bytes into `.text` at 0x408040 in both profiles, phase
  32,832, so a 64 KiB anchor adds **32,704 bytes** of fill ahead of the head in this static program (file bytes, not
  touched pages). Other programs differ by their own `.text` prefix; libc.so.6 would take (i) at no per-program cost.
* Planner search (scratch prototype, not committed): a deepening search over the bins' mode sets with a constructive
  pack found **no plan at cost 19..25** for fast; four greedy variants (zero-cost section fill first, bytes-first order)
  were all worse (29-39). So the bound is loose (the all-modes group nearly fills the head bin, and fork/true live only
  there), and the greedy is the planner to apply.

## Next: slice 2 (APPLY), then slice 3 (PROOF), unchanged from run 1

2. **APPLY:** rename members to `.text.sorted.h9<bin 2d><0|1><pos 5d>.<f>` (0 member, 1 filler; extend `position_of`
   for the 9-digit form), 65536 alignment on each gap-bearing bin's next bin's first section (`Bin::gap`), the head
   bin anchored or not behind ONE constant (variant i/ii; libc.so.6 takes i), fillers renamed from the plan
   (`layout-bins.tsv` signature `cold`); bins with no modes keep plain list order after the counted bins. Delete
   `cluster_align`, `head_alignments`' cluster use, `hot-text.clusters` and `hot-text.first-window` once `static` and
   `so` no longer need them. Then hot_text_products both profiles; tighten bounds that drop (ratchet), plants.
   Open: whether the probe link in `crates/core/tests/hot_text_products.rs` uses `--gc-sections` (fillers dropped).
3. **PROOF:** pad sweep (shiki `~/work/LAYOUT-BUDGET-RETHINK/sweep.{sh,py}`), file sizes, head variant numbers.

**Not to redo:** the greedy-vs-search comparison above; the trace already covers every page_faults mode
(regen's --all-modes check). **Cost note:** one `so` per profile is ~15 min on keke; the planner can be iterated
offline from `layout-bins.tsv` + `readelf -SW target/product/<p>/liella-processed.a` (that is how the search ran).
