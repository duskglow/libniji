# HEAP-SPILL-COLD (issue #21): handoff and report

State: branch `task/heap-spill-cold` = origin/next `71597d2a4` + this note and `handoff/reports/HEAP-SPILL-COLD.json`;
no code change. Build role keke `~/w/n3` (Zen 1, 60 vCPU). Scripts and outputs on keke `~/work/heap-spill-cold/`
(`measure.sh <tree> <tag>`, `shift.c`, `count.c`; base products copied to `base/target/niji`; outputs `out-base/`).
Spec: `~/state/briefs/HEAP-SPILL-COLD.md` + coordinator note answering challenge #7189 (11-21 06:02Z).

## Outcome

Challenge #7189 (accepted by the coordinator): a spill hands back a block of another class, but the cold
`grow_segment` returns a slab id through `take_slab` (inlined in `refill`, bin 0, 3857 B fast) to `alloc_small`
(inlined in `alloc_slow`, bin 0, 1280 B), which allocates at stride `CLASSES[c]`; no zero-byte route exists (the
failure route `refill` -> `None` -> `alloc_large` runs `take_elsewhere`, which locks all 63 other shards per spilled
`malloc`). The request's alignment is known only in `alloc_fast`/`alloc_slow`; the safe rule without it,
lowbit(`CLASSES[c2]`) >= lowbit(`CLASSES[c]`), forbids 128 -> 160 (the issue probe's whole refill saving) but allows
112 -> 128, 96 -> 128, 160 -> 192, 192 -> 256. Coordinator: measure which cause dominates `find` first; if
nearby-size reuse is not it, hand back the per-cause numbers instead of building the spill. It is not: no spill built.

## Measurements on next (`71597d2a4`, fast products; job heap-spill-cold-measure-base2)

`find`: findutils 4.11 objects of the validation build relinked statically against the products (the workload
kit's method); reference = the system `find`. Need = least `ulimit -v` (KiB, 16 KiB bisection) that succeeds.

| | 3 entries | 200000 entries (5 runs) | growth | issue formula (3-entry need + 35000) |
|---|---|---|---|---|
| libniji | 6064 (5/5) | 44560, 45104, 45296, 45312, 45616 | 38496..39552 | 41064: fails by 3496..4552 |
| reference | 3664 (5/5) | 33568 (5/5) | 29904 | 38664: passes |

What `find` allocates (an LD_PRELOAD counter on the glibc-linked findutils 4.11 build): 200003 `malloc`s, every one
of 257..272 bytes (one FTSENT per entry); it reads the directory in batches of 100000 entries and frees each entry
after visiting it, so 100000 frees (same sizes) come before the peak; peak live 25836 KiB; 4 `realloc`s, 4 blocks
over 1024 bytes. libniji's strace of the same run: no `mremap`; segment growth (2 MiB maps), three 32 MiB
transient aligned maps trimmed back, one window cut short by the locale-archive map.

Per cause, `find` (KiB of the 38496..39552 growth; the gap to the formula is 3496..4552):
- **Size-class rounding (dominant):** 257..272 bytes land in class 320 (classes 256, 320): 100000 live blocks x
  320 B = 32000 KiB against 25836 for the reference, +6164. A class of 272 bytes would serve them exactly: by
  that arithmetic up to 100000 x 48 B = 4688 KiB, enough to bring the growth under 35000 (estimate, not measured).
- **Nearby-size reuse: 0.** One size range only; the second batch reuses the first batch's freed blocks of the
  same class. Neither a 128 -> 160 spill nor any other changes `find`.
- **Realloc growth: 0.** 4 reallocs, no `mremap`.
- **Up-front reservation:** inside the 3-entry need (6064 against 3664, +2400: the static image and the first
  segment's 16 slabs), so the formula cancels it.
- **Remainder** (overhead beyond live blocks: libniji 6496..7552, reference 4068, so 2428..3484 more): per-segment metadata (256 KiB
  each, 3 segments), the 2 MiB growth step, the cut-short window.

Issue probe (`crates/core/tests/oracle/heap_address_space.c`, VmSize growth over start, KiB, 3/3 identical):

| step | libniji | reference (2.39) |
|---|---|---|
| malloc1 | 4420 | 132 |
| alloc | 28996 | 27984 |
| refill | 39236 | 29568 |
| realloc | 41288 | 28052 |

RLIMIT_AS need: fixed layout 41296; randomised 20 runs 41296..43440 (41296 x6, 41312 x4, 41328, 41408, 41568,
41680, 41792, 42512 x2, 42624, 42976, 43440 x2). Per cause (probe): refill +9668 over the reference is nearby-size
reuse: 75000 of the 100000 `malloc(120)` (class 128) need new blocks while 50000 freed class-160 blocks
(129..159 bytes) sit idle; a 128 -> 160 spill (run -4's, inside the hot path) measured 33092. That needs the request
alignment (lowbit rule forbids it). realloc +2052 over refill (the 1.6 MB array's growth; the reference reuses its
freed heap). Up-front reservation: malloc1 +4420 (`SEG_FIRST` = 16 slabs = 4 MiB + metadata), counted in full by
the RLIMIT need.

Class-shift workload (`shift.c`: 200000 x `malloc(128)`, 3 of 4 freed, then 200000 x `malloc(104)`; VmSize / RSS
KiB over start, 3/3 identical): libniji phase1 +26948 / +2544, phase2 +49220 / +2720; reference +28248 / +30136,
+33660 / +35604. Phase 2 grows libniji's VmSize by 22272 KiB against the reference's 5412: the 112 -> 128 spill (allowed
by the lowbit rule) would serve it from the 150000 freed 128-byte blocks. Not built (coordinator note).

## RULING NEEDED / recommendations

1. Issue #21's `find` acceptance needs a size class for 257..272 bytes (for example 272 or 288 between 256 and
   320), not the spill. Recommend a story HEAP-CLASS-272: add the class (CLASSES is 44 entries: the cache layout,
   `MAGIC`, `SMALL_CLASS` and the per-class tables move; the fast path's class count stays under 64), measure find
   under the formula, the release bins and the start-up rows.
2. The spill still has value for the probe's refill step (target 33092) and for class-shifting programs (+22 MB
   above). Recommend keeping HEAP-SPILL-COLD open, ranked after HEAP-CLASS-272, built as the coordinator allows
   (refill returns the class; release bins within budget; hot-function instruction counts before and after) with
   the lowbit rule. The probe's 128 -> 160 needs the alignment: a cheap carrier was not found (`alloc_slow` alone
   knows it; the aligned C entries are several: `debug_alloc`, `foreign_alloc`, the Rust `GlobalAlloc` paths).

## Next step

Coordinator: rule on 1 and 2. Nothing to resume on this branch.
