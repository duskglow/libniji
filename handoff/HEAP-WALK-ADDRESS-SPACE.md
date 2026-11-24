# HEAP-WALK-ADDRESS-SPACE: handoff (after leg 2)

State: branch `task/heap-walk-address-space`, code head `0540cb9f6` (cut from origin/next `5093cce21`, nothing merged).
Role keke `~/w/n3` (Zen 1, 60 vCPU), synced at `0540cb9f6`. Brief: `~/state/briefs/HEAP-WALK-ADDRESS-SPACE.md`.
Decision in force: D-3.26 (`records/decisions/D-3/D-3.26.toml`). Plants: `handoff/HEAP-WALK-ADDRESS-SPACE-plants.txt`.
Scratch on keke: `~/work/heap-walk-address-space/` (`measure.sh <label> <prod dir|glibc>`; `out-<label>/`; `next-src`, `head-src`
= release-profile source copies used for the bench and the alone rows).

## The change
- `crates/rt/src/heap.rs`: 56 classes, eight per doubling 128..1024 (16-byte steps to 128, then 16/32/64), four above; still
  32 classes <= 1024 (fused `& 31` unchanged). Cache 384 -> 496 words: 12 class lines and free-path slab hints 16 -> 32
  (leg 2: with 16, lifo_batch64 touched 20 classes and lost hint hits: 8937 instr/batch vs next 8342; with 32: 8214).
- Tests: `heap_and_sync` (forecast 162 -> 176; new `a_block_up_to_1024_bytes_exceeds_its_request_by_at_most_an_eighth`),
  `heap_meta_products::the_heap_keeps_address_space_near_its_use` (rule form: page-exact least RLIMIT_AS headroom vs
  the most held without a limit, allowance FIND_PLACEMENT_SLACK), `s03_memory_symbols::realloc_in_place_fit_rule`
  (bound follows the table: class below 1024 is 960, so 481 stays / 480 moves).

## Measurements (keke, Zen 1; dynamic probe = oracle steps through the products' loader; KiB)
Find formula = least `ulimit -v` over 200000 entries minus over 3, page-exact, setarch -R, 8 stack phases:
| tree | formula (8 phases) | find mapped at exit / RSS |
|---|---|---|
| next 5093cce | 21220..21724 | 49000 / 35884 |
| this branch | 16868..17660 | 44648 / 32600 |
| next + LD-START-MAPS-3 | 35608..37284 | 49292 / 36144 |
| this (40e70aa1c) + LD-START-MAPS-3 | 32940..33196 (issue allows 35000) | 44948 / 32728 |
| glibc 2.39 | 29904 (all phases) | 33704 / 29332 |
Issue steps VmSize start/malloc1/alloc/refill/realloc [RSS]:
- glibc 4176/4308/32160/33744/32228 [1272/-/31084/32648/31204] (probe never writes blocks: libniji RSS stays low)
- next 8692/13112/37688/47928/49980 [1328/1408/3324/3468/5328]; this: same VmSize [1328/1408/3316/3460/5320]
- next+LSM3 8984/13404/37980/48220/50272; this+LSM3 8992/13412/37988/48228/50280 (steps 96..159 unchanged by classes)
Where the difference sits: find's 264-byte entries took 320-byte blocks (now 288; reference chunk 272); with ~100000
live the walk spilled 4352 KiB into a third segment, now two. The refill step (+10 MiB vs reference) is freed blocks of
the 96..159 classes not reusable by `malloc(120)`'s class: cross-class reuse, not done (next story).

## Per-call (cbench_malloc, instr/op, next -> final; `cargo xtask bench --compare --counters`)
malloc_free_* 122.2 -> 121.2; lifo_batch64 8342.1 -> 8213.9; realloc_grow_x2 1748.8 -> 1757.4 (+0.5%);
realloc_grow_16step 5722.6 -> 5700.2; calloc_free_64 172.2 = ; calloc_free_4096 416.3 -> 425.2 (+2.1%);
realloc_within_usable 194.2 -> 193.2; realloc_moving 428.5 -> 424.4; realloc_null_24 145.2 = ; heap_mix_real 149.1 -> 144.5.
The two worse rows pass sizes > 1024 through `class_for`'s general path, which now picks between two step widths
(`e < 10`); likely cause, not verified by a trace. Fix-forward: fold the two formulas (e >= 10 is the old one + 12).

## Jobs (leg 2)
Final head 0540cb9f6: `heap-walk-address-space-gate4` (static + affected --run), `-plants3`, `-startup3` (chained after
gate4: cargo xtask test, full-profile so + hot_text_products). Superseded: gate (40e70aa1c, clippy E0689), gate2
(26b90489e, realloc_in_place_fit_rule), gate3 (71f0ca5, that suite only, green), plants2 (26b90489e, 2/2 CAUGHT),
startup2 (71f0ca5, stopped for the hint change). Counts: bench-next, bench-head (71f0ca5), bench-hint32 (= final code).

## Exact next step
Collect gate4/plants3/startup3; on a red: fix, rerun only that suite. Then precheck + handback-check on the final head.
Next story: cross-class reuse for the refill step; fold `class_for`'s two formulas (calloc_free_4096 +9 instr).
