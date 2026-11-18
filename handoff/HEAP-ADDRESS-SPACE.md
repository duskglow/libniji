# HEAP-ADDRESS-SPACE (GitHub issue #21): handoff

State: branch `task/heap-address-space-5` (continues -4 .. -1), head = this handoff's commit; code commit `7c6a4078b`.
Build role keke `~/w/n3` (Zen 1); scripts in keke `~/work/HEAP-ADDRESS-SPACE-5/` (`m2.sh` measurement, `as_need.sh`
issue steps + RLIMIT_AS bisection, `strace_modes.sh` per-mode growth count, `bins.py` layout-bin compare, saved
`layout-bins-{base,seg2,2}.tsv`). Merge base 7f7e6c9e4 built in RAM at `/dev/shm/claude/HEAP-ADDRESS-SPACE-5/base-1`.
Spec: `~/state/briefs/HEAP-ADDRESS-SPACE.md` "## The change"; binding direction: `~/state/briefs/HEAP-ADDRESS-SPACE-5.md`.
Decisions in force: D-3.19, D-3.14, ruling #6325; pending records `records/pending/HEAP-ADDRESS-SPACE-3-1.toml`
(D-3: growth with use, updated for -5) and `-2.toml` (D-X: GOT alignment).

## Run -5: done (status INCOMPLETE: issue #21 cannot pass without the spill, RULING NEEDED)

- Spill removed (spill_class, SPILL_FROM, seems_to_hold_a_slab, spill parameter/recursion; its rt test and the
  heap_threads adjustment). alloc_small and refill are next's code again; take_held keeps `next < mapped`; take_slab
  has one test (`seg != 0 && mapped < SLABS`) calling `grow_segment`, `#[cold] #[inline(never)]`, which holds all
  in-place growth (`Pages::grow`).
- SEG_FIRST 2 -> 16. strace with 2: locale/tool grew twice, mktime/localtime/localtime_mktime/rebind 3 times,
  strings once (true/fork/float 0), and grow_segment (658 B) went into bin 0 and pushed `__libc_free` out. With 16:
  no growth in any of the 10 modes.
- Layout bin 0 (target/product/fast/layout-bins.tsv) vs merge base: heap sections 17089 -> 16793 B (map 728 replaced
  by map_room 455; bins.py also counts a 172 B legacy `RawVecInner::finish_grow`, not heap); total 60627 -> 60623;
  `__libc_free`, `printf_text`, `pair_s`, `Env::text` still in bin 0; grow_segment in bin 6 with no mode signature.
  (pairscan_ci left bin 0; it was a cold filler there on base.)
- hot_text_products (fast profile) 26/26 green, base counts on keke tool 44 / locale 43 / true 11 / float 3;
  rt heap_and_sync 28/28.
- Issue #21 without the spill (VmSize growth over start, KiB; start 7140): malloc1 +4420 (ref +132), alloc +28996
  (ref +27984), refill +39236 (ref +29568, bound 33092), realloc +41288 (ref +28052, bound 35144); least RLIMIT_AS
  42000 (ref 29568; test headroom 35152; issue's 35000). The test stays red at refill.

## Jobs (keke)

Evidence (diagnostic, rc 1 by the issue test): `heap-address-space-5-m1` (SEG_FIRST 2), `-m2` (SEG_FIRST 16), `-base1`.
On the head: `heap-address-space-5-lint1` then `-fullhot` (NIJI_FULL_PROFILE so + hot_text_products), and
`-plants` (handoff/HEAP-ADDRESS-SPACE-5-plants.txt, dir `/dev/shm/claude/HEAP-ADDRESS-SPACE-5/plants-1`): running at hand-back.

## Next steps, in order

1. Read the ruling. (a) If growth-with-use lands without the spill: set refill/realloc/AS_HEADROOM in
   heap_meta_products.rs to the measured values above + 256 (the record already carries the numbers), add the
   first-large-max plant back (it needs that test green), run the gate + `cargo xtask test` + fullhot. (b) Otherwise
   HEAP-SPILL-COLD: a cold extension of grow_segment, never a parameter of alloc_small/alloc_slow.
2. Read lint1 / fullhot / plants; any WARN or survivor is the next run's first fix.
3. Earlier items still open: relro by-section comparison (ruling #6325 step 1); cbench_malloc / cbench_startup
   counters; returning empty segments' address space; findutils `find` under the issue's `ulimit -v` formula.
