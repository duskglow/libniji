# HEAP-ADDRESS-SPACE (GitHub issue #21): handoff

State: branch `task/heap-address-space-2` (continues `task/heap-address-space`), WIP head after `ee9377a71` (this
handoff's commit). Build role keke `~/w/n1`; probe scripts in keke `~/work/HEAP-ADDRESS-SPACE-2/` (`probe.sh`: issue
probe, reference vs libniji, RLIMIT_AS bisection; `base.sh`: base products in a RAM copy).
Spec: `~/state/briefs/HEAP-ADDRESS-SPACE.md` "## The change". Decisions in force: D-3.19, D-3.14 (to be amended).

## Done (runs -1 and -2)

- Segments mapped as far as used, grown in place by doubling (at most 8 slabs per step); first reservation is
  [registry page][cache chunk][segment]; large table generation 0 at most one page; no-segments threshold 4 x
  `SEG_MIN`; nearby-class reuse (`spill_class`, cold, checked lookups) before the heap maps more; `locate_in`
  sends addresses in a window's unmapped part to the large table. Details: run -1's commits.
- Run -2: the 4 failing rt tests fixed by cause (all were expectations of whole 16 MiB windows, or a premise the
  new growth broke; no assertion weakened): job `heap-address-space-2-rt2`, 27 passed.
- Regression test `the_heap_keeps_address_space_near_its_use` (crates/core/tests/heap_meta_products.rs) written
  with the numbers below; NOT RUN yet (products blocked, below). On next it must fail (next grew 16448 / 32836 /
  49220 / 67652 KiB): not yet shown.

## Issue probe, keke, identical over 5 runs (VmSize growth over start, KiB)

| step | reference (dynamic) | libniji next (run -1, mei) | libniji this branch |
|---|---|---|---|
| malloc1 | +132 | +16448 | +836 |
| alloc 200000 x 96..159 | +27984 | +32836 | +28996 |
| refill malloc(120) | +29568 | +49220 | +32836 |
| realloc to 200000 | +28052 | +67652 | +34888 |
| least RLIMIT_AS headroom | 29568 | 67652 | 34896 |

Remaining gap, by cause: refill (+3268): 25000 refills cannot use class-112 holes (blocks of 97..112 bytes; the
reference's 16-byte rounding puts 105..111 in a chunk that fits 120); realloc (+6836): empty segments are never
unmapped and the 2 MiB array gets its own mapping.

## BLOCKER: `cargo xtask so` fails STARTUP-DATA-WINDOWS (8 written start-up pages > rw-budget-fast 7)

Jobs `heap-address-space-2-probe1/probe2`. The `.got` (0xc8 bytes, start-up entries at +0x30..+0xc0) follows
`.data.rel.ro` (64 KiB aligned) and now straddles a page. Base (`heap-address-space-2-base`, merge base 7f7e6c9e4,
same host): `.data.rel.ro` 0xb180, `.got` at 0x86b180, 7 pages, contract ok. Branch: `.data.rel.ro` 0xaf68
(536 bytes SMALLER), `.got` 0x86af68..0x86b030, 8 pages. The start-up relro cluster is identical to the base after
`spill_class` (79 sections, 11384 bytes; the inline closure had made it 81 / 11446). So the page is lost by a
shrink of relro outside the start-up cluster: layout jitter. It fits again if `.data.rel.ro` is <= 0xaf40 or
>= 0xafd0. Not chased (no padding or section moves): coordinator/owner call (RULING NEEDED in the report).

## Next steps (exact, in order)

1. Ruling on the blocker. If it is to be fixed here: find the 536-byte relro difference (symbol names are
   anonymous per build: compare `.data.rel.ro` input sections by object with a linker map of both builds).
2. `cargo xtask so`, then `cargo test -p niji --test heap_meta_products` on the branch; show the new test fails
   on next (link the same oracle against next's products, e.g. keke `/dev/shm/claude/HEAP-ADDRESS-SPACE-2/base-1`).
3. Decision record (`cargo xtask records new decision D-NEW --series D-3 --story HEAP-ADDRESS-SPACE-2`, amends
   D-3.19 and D-3.14), plants file (grow step size, the locate_in Large route, the spill bound, FIRST_LARGE,
   SEG_FIRST), `cargo xtask plants`, direct gate, start-up checks (`cargo xtask test`, hot_text_products with
   NIJI_FULL_PROFILE=1), cbench_malloc and cbench_startup counters before/after.
4. Then the gap: returning the address space of wholly empty segments (needs a design for lock-free
   `find_seg` readers; decision record) would remove most of the realloc-step gap.
5. findutils `find` over 200000 entries under the issue's `ulimit -v` formula: not reproduced yet.

## Process notes

Run -1: one heredoc and one foreground `cargo xtask so` outside jobrun (its own note). Run -2: none.
