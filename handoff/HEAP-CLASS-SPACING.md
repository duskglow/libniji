# HEAP-CLASS-SPACING: handoff (leg 1 -> leg 2, REFRESH at ctx 152k / 91 calls)

State: branch `task/heap-class-spacing`, base origin/next `7e5d8b3fa` (heap.rs, heap_and_sync.rs and
heap_meta_products.rs are identical to origin/next `ef7bc6fd3`). Commits: `520e74474` (the change), `91cfc81d5`
(plants file), then this note. Role keke `~/w/n1` (Zen 1, 60 vCPU), synced at `91cfc81d5`. Brief:
`~/state/briefs/HEAP-CLASS-SPACING.md` (coordinator addition: measure on RELEASE products). Reserved id: D-3.23 (not
yet written: `records/decisions/D-3/D-3.23.toml`, ann "—", spec "§3.1", no amends; no earlier record fixed the table).

## The change (520e74474)
`crates/rt/src/heap.rs`: CLASSES 44 -> 52: 16-byte steps to 128, four per doubling, EIGHT per doubling for the
doublings in [256, 1 KiB) (`EIGHTHS_FROM` = 256, `EIGHTHS_TO` = `FORECAST_CAP`), four after, to 64 KiB. A const block
checks the table against the rule; `NCLASSES` replaces every literal 44; `CACHE_WORDS` 384 -> 448 (C_CLASS 8 +
52*8 + 2*LARGE_KEEP + 16 hints); `class_for` takes sizes <= 1024 from SMALL_CLASS for every alignment, and above 1 KiB
`ABOVE_1K + (e - 10) * 4 + ...` (`ABOVE_1K` = 28, computed). Fused path: classes <= 1 KiB are indexes 0..27 (< 32,
mask 31 ok). Test added: `rounding_loses_at_most_one_step_of_the_class_spacing` (+ aligned cases in
`classes_cover_every_small_size_with_the_fundamental_alignment`), `crates/rt/tests/heap_and_sync.rs`.

## Measured so far (keke, Zen 1)
- Model (LD_PRELOAD counter `~/work/heap-class-spacing/hc.c` on the glibc kit; per process sum over classes of
  roundup(peak live blocks x class size, 4 KiB) = touched-page model; table `~/work/heap-class-spacing/model-table.txt`,
  scratch copy here: .scratch/HEAP-CLASS-SPACING): total over all 27 workloads + find + probe, KiB vs today:
  e8 128..1K -4744, 128..4K -4732, 256..1K -3860, 256..4K -3848, 256..2K -3856, 256..512 -3904, lin256 -884,
  e2 (two per doubling) -309644 (all printf-loop: 20000 procs x -16 KiB) but find +6240, du +1800, probe +3028.
  Gains: find-200k -3120 (200009 entries of 257..272 B -> 320), du -800 (10 runs), make-n -316 / probe -776 only
  with 128..256 refined. Costs: every class a process uses touches its own page: 128..1K costs +4..+20 KiB on ~20 small
  workloads; 256..1K +4..+8 KiB on ~9 (gawk x4, grep-E/F, sed-utf8, bash-loop, find-3); >1K refinement: no gain.
  Chosen 256..1K: near-best total, fewest regressing rows, cap at the fused-path cap (512 cap: 44 KiB better in the
  model but no general reason to stop there).
- RELEASE base (findrel.sh, products ~/wl/src/7e5d.../target/niji): find 200k niji u=263.3M k=136.9M 8036 faults
  34080 KiB; glibc static 288.5M/128.3M/6708/27840; glibc dyn 292.5M/129.0M/6741/28800; 3 entries niji 149k/1103k/93
  faults/2400 KiB. Probe VmSize: start 6528 malloc1 10948 alloc 35524 refill 45764 realloc 47816; RLIMIT need fixed
  41296, random 41488 41296 41312 41568 41312. => rounding cost IS present on release products (premise holds).
- Base workload kits: `~/wl/kits/7e5d8b3fabb489794a46fe277fcb4fbf9c9d0b7f-release/table.txt` (job
  heap-class-spacing-kits-base, 2716 s; dynamic verdict FAIL 23 rows > 1.10, geomean 0.8941: base state, not ours).

## Jobs
Running at refresh: `heap-class-spacing-rt` (cargo test -p niji-rt --test heap_and_sync && cargo xtask clippy).
Done: kits-base, find-base2, hist2, findhist. Killed (mine): malloc-base2 (blocked the sync).
SUPERSEDED: heap-class-spacing-find-base (no libm.a yet), heap-class-spacing-malloc-base (chain broke on git log).

## Exact next steps
1. `~/bin/jobwait heap-class-spacing-rt`; fix any failure (re-sync after each fix).
2. `wl_percall` with base: `~/bin/jobrun heap-class-spacing-percall count -- sh -c 'cd ~/w/n1 && bash
   validation/wl_percall.sh --base 7e5d8b3fabb489794a46fe277fcb4fbf9c9d0b7f .'` (malloc_free, realloc_chain rows =
   the hot-function counts before/after; it also copies n1 to ~/wl/src/<head>).
3. Candidate kits FROM THAT COPY so n1 stays free: `cd ~/wl/src/<head> && bash validation/wl_kits.sh <head>` (~45 min),
   then `bash ~/work/heap-class-spacing/findrel.sh <head> cand` (find + probe after).
4. On n1, one at a time: gate `cargo xtask affected --run` (gate kind), `cargo xtask test`, `NIJI_FULL_PROFILE=1 cargo
   xtask so && NIJI_FULL_PROFILE=1 cargo test -p niji --test hot_text_products`; diff
   target/product/fast/layout-bins.tsv vs base (start-up code: heap). Plants: `cargo xtask plants
   handoff/HEAP-CLASS-SPACING-plants.txt --dir /dev/shm/claude/heap-class-spacing/plants-1 --timeout 600` (plants kind).
5. Small programs VmSize/RSS + start-up faults (kits step 3 `--version` rows cover per-start faults) before/after.
6. Write D-3.23 (`cargo xtask records new decision D-3.23`, numbers above + after), report json, precheck, handback-check.
Not measured / out of reach: Skylake-SP class (only keke assigned) -> say so.
