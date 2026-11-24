# HEAP-WALK-ADDRESS-SPACE: handoff (leg 1 -> leg 2)

State: branch `task/heap-walk-address-space`, code head `40e70aa1c` (cut from origin/next `5093cce21`, nothing merged).
Role keke `~/w/n3` (Zen 1, 60 vCPU), synced at `40e70aa1c`. Brief: `~/state/briefs/HEAP-WALK-ADDRESS-SPACE.md`.
Decision in force: D-3.26 (`records/decisions/D-3/D-3.26.toml`, id reserved via ops). Plants: `handoff/HEAP-WALK-ADDRESS-SPACE-plants.txt`.
Scratch on keke: `~/work/heap-walk-address-space/` (`measure.sh <label> <prod dir|glibc>`: steps VmSize/RSS + find formula
over 8 phases; `steps_need.sh <label> <prod> <oracle.c>`: static steps, least RLIMIT_AS headroom; `trace.sh`: glibc mtrace
of find's requests). Product copies there: `prod-lsm3` (next + LD-START-MAPS-3), `prod-hlsm3` (this change + LD-START-MAPS-3).
Local scratch worktree (detached, next + origin/task/ld-start-maps-3 merged, measurement only): `$S/lsm3`.

## The change (commit 40e70aa1c)

- `crates/rt/src/heap.rs`: CLASSES 44 -> 56: 16-byte steps to 128, EIGHT per doubling 128..1024 (144..256 by 16, 288..512
  by 32, 576..1024 by 64), four per doubling above. Still 32 classes <= 1024 (fused `& 31` unchanged). `NCLASSES` const;
  CACHE_WORDS 384 -> 480; `class_for` arithmetic: e < 10 -> `8 + (e-7)*8 + ((s >> (e-3)) - 8)`, else `32 + (e-10)*4 + ...`.
- `crates/rt/tests/heap_and_sync.rs`: forecast 162 -> class 176 (was 192); new test
  `a_block_up_to_1024_bytes_exceeds_its_request_by_at_most_an_eighth`.
- `crates/core/tests/heap_meta_products.rs` `the_heap_keeps_address_space_near_its_use`: rule form added BEFORE the
  AS_HEADROOM constant check: fixed layout, bisect least headroom (page-exact, from 1<<17) vs max VmSize growth over the
  steps without a limit (arg "-"), allowance FIND_PLACEMENT_SLACK. AS_STEPS/AS_HEADROOM constants kept (removing = ruling).

## Measurements (keke, Zen 1; dynamic probe `probe_steps.c` = the oracle's steps run through the products' loader)

Find formula = need(200k) - need(3), `ulimit -v` bisection page-exact, setarch -R, 8 stack phases, en_US.UTF-8, tmpfs:
| tree | over 3 entries (8 phases) | need200k | mapped at exit / RSS |
|---|---|---|---|
| next + LD-START-MAPS-3 | 37028 at 0/2048/8192/10240, 35608 at 4096/6144, 37284 at 12288/14336 | 47976..49652 | 49292 / 36144 |
| this + LD-START-MAPS-3 | 32940 x6, 33196 x2 | 45316..45572 | 44948 / 32728 |
| glibc 2.39 | 29904 all phases | 33564 | 33704 / 29332 |
Cause: find requests 264 B x 200002 (glibc mtrace), ~100000 live; old class 320, new 288 (glibc chunk 272). Heap at exit:
before 16452+16384+4352 (third segment), after 16452+16384 (two segments).
Steps (dynamic, VmSize KiB start/malloc1/alloc/refill/realloc; RSS in brackets):
- glibc: 4176/4308/32160/33744/32228 [1272/-/31084/32648/31204] (probe never writes its blocks: libniji RSS stays ~3.7 MiB)
- next+LSM3: 8984/13404/37980/48220/50272 [1572/1716/3632/3776/5572]
- this+LSM3: 8992/13412/37988/48228/50280 [1572/1780/3688/3768/5628] (steps unchanged: sizes 96..159 classes; refill +10 MiB
  needs cross-class reuse, not done)
- static steps (heap_address_space.c, this+LSM3): growth realloc 41288, least headroom 41288 (rule holds with 0 over).
NOT measured yet: next alone and this branch alone (no LSM3) for the table; per-call heap bench rows.

## Jobs started (leg 1, all on 40e70aa1c)

- `heap-walk-address-space-gate` (clippy + static + `affected --run`, ~/bin/gate 16G), then chained
  `heap-walk-address-space-startup` (`cargo xtask test`, full-profile so + hot_text_products).
- `heap-walk-address-space-plants` (2 plants, dir /dev/shm/claude/heap-walk-address-space/plants-1).

## Exact next step (leg 2)

1. `~/bin/jobwait heap-walk-address-space-gate heap-walk-address-space-plants` then `...-startup`; fix reds (a start-up
   fault/window red: compare layout-bins.tsv with the base; the class table is rodata, `class_for` arithmetic changed).
2. Bench ratchet: `cargo xtask bench --compare --counters <heap bench>` on the role (after the start-up job; one build
   per role): per-call malloc/free/realloc rows beside next. Find the bench names in docs/benchmarks.md.
3. Optional table rows: measure.sh on next alone / this alone (products from `cargo xtask so` on each).
4. Report json `handoff/reports/HEAP-WALK-ADDRESS-SPACE.json` (+ "jobs"), precheck, handback-check, hand back.
RULING NEEDED candidates: none yet. Next story suggestion: cross-class reuse for the refill step (+10 MiB vs reference).
