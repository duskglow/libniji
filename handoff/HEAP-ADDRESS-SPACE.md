# HEAP-ADDRESS-SPACE (GitHub issue #21): handoff

State: branch `task/heap-address-space`, WIP head `3daf6e03f` (code written; checkpoint at the context budget).
Built and run on the Zen 1 build host (role `mei:~/w/s3`). Job `heap-address-space-t2` (rt heap tests, clippy,
products, issue probe on the WIP head) was started detached at hand-back: read `~/logs/heap-address-space-t2.log`.

## Mapping table before the change (step 1 of the brief; KiB, VmSize)

Probe: `~/work/HEAP-ADDRESS-SPACE/vmprobe.c` on mei (the issue's steps, no stdio; smaps saved per step under
`~/work/HEAP-ADDRESS-SPACE/before/`). Reference = dynamic, libniji = static (the heap's mappings are the same).

| step | reference | libniji | libniji's new mappings (from smaps) |
|---|---|---|---|
| start | 4176 | 7180 | none from the heap (binary 4532 with all of libc; bss 2424 holds the probe's own 1563 KiB array; a 68 KiB start-up mapping) |
| first malloc(16) | 4308 | 23632 | first reservation 16452 = the whole 16 MiB segment window + 4 KiB registry page + 64 KiB cache chunk |
| 200000 x malloc(96..159) | 32160 | 40016 | + a second whole segment (16384) |
| free every other | 32160 | 40016 | - |
| refill with malloc(120) | 33744 | 56400 | + a third segment (16384): class-128 requests cannot use the class 96/112/160 holes |
| free everything | 33744 | 56400 | - (segments are never unmapped) |
| doubling realloc to 200000 | 32228 | 74832 | + large-table generation 0 (16384 = 2^20 slots x 16 B) + the final 2 MiB array |

So the "+20 MiB at start" is the 16 MiB first segment window (the rest is libniji's larger text/data, not the heap);
the realloc step's +18 MiB is the large table's up-front 16 MiB, not kept outgrown blocks (realloc of large
blocks uses mremap; `LARGE_KEEP_BYTES` only applies to freed large blocks and is unchanged). Resident memory of
libniji stays far lower (it never touches the blocks it hands out).

## The change (WIP, not yet verified)

- `Pages::map_room` / `Pages::grow` (rt trait; boundary: over-map room + alignment then trim; grow with
  `MAP_FIXED_NOREPLACE`; `map_aligned_exactly` gained a head offset for the limited case).
- A segment maps its metadata and 2 slabs (`SEG_FIRST`, `SEG_MIN` = 768 KiB) and grows in place by doubling, at
  most 8 slabs (2 MiB) per step, as its shard hands slabs out (`Shard::mapped`); a taken window ends the segment.
- The first reservation is now [registry page][cache chunk][segment], one mapping (D-3.19 kept).
- `locate_in`: a slab no cache owns sends the address to the large table (another mapping, a large block's
  too, can sit in a window's unmapped part); invalid pointers are still refused there.
- Large table generation 0 at most 256 slots (`FIRST_LARGE`, one page); it grows by generations (T14).
- No-segments threshold under `RLIMIT_AS`: 4 x `SEG_MIN` (3 MiB) instead of 4 x 16 MiB.
- `refill(spill)`: when the shard holds no slab (more address space needed) and the request has default
  alignment, a class at most 1.5x larger with a free block in this cache serves it first.
- Stats `small_mapped` / peaks count mapped segment bytes (`seg_mapped`).
- rt tests adapted: `segment_nodes` finds segments by their 16 MiB boundary inside a mapping; FakePages has
  `map_room`/`grow` (merging like a VMA); the 1 GiB-limit test expects the new first mapping; the two
  borrow-from-another-shard tests first make the donor shard map enough slabs.

## After (head 032fc0883, job heap-address-space-t3 on mei; libniji static, VmSize KiB)

| step | reference | libniji before | libniji after | after, growth over start vs reference's |
|---|---|---|---|---|
| start | 4176 | 7180 | 7184 | - |
| first malloc | 4308 | 23632 | 8020 | +836 vs +132 |
| 200000 x malloc(96..159) | 32160 | 40016 | 36180 | +28996 vs +27984 |
| refill with malloc(120) | 33744 | 56400 | 40020 | +32836 vs +29568 |
| doubling realloc | 32228 | 74832 | 42072 | +34888 vs +28052 (segments are never unmapped) |

Peak growth 34888 KiB (was 67652): just inside the issue's 35000 KiB headroom for this program. Clippy and the
products build passed in t3 (its log); the issue probe ran on the new products.

## Not done / next step (exact)

1. Four rt tests still fail on 032fc0883 (`cargo test -p niji-rt --test heap_and_sync`):
   `an_address_space_limit_sizes_the_registries` (line 406), `the_first_allocation_reserves_one_mapping`
   (478), `segments_are_aligned_and_found_from_any_address` (1115) and
   `a_thread_whose_cache_cannot_be_made_takes_a_parked_cache_of_another_shard` (796, the alloc after the
   parked cache is taken). Find each cause (FakePages `map_room`/`grow` model, segment counting, the spill
   path) before changing any expectation; none was investigated (context checkpoint).
2. Wire `crates/core/tests/oracle/heap_address_space.c` (committed, unused) into a test in
   `crates/core/tests/heap_meta_products.rs`: growth over start at each step <= the reference's measured growth
   (malloc1 132, alloc 27984, refill 29568, realloc 28052 KiB on the Zen 1 host) + a margin set from the after
   numbers; and the run with argument N (RLIMIT_AS = start + N KiB) at the reference's need + margin. Show it
   fails on next.
3. Decision record (`cargo xtask records new decision D-NEW --series D-3 --story HEAP-ADDRESS-SPACE`, amends
   D-3.19 and D-3.14), plants file (`docs/plants.md`: grow step, the locate_in Large route, the spill bound,
   FIRST_LARGE), `cargo xtask plants`, direct gate, the start-up checks (`cargo xtask test`, hot_text_products),
   cbench_malloc / cbench_startup counters before and after.
4. findutils' `find` over 200000 entries under the issue's `ulimit -v` formula: not reproduced yet (no
   libniji-built find on the build hosts; `validation/suite.sh` builds findutils).

## Process notes (own deviations, for the record)

One heredoc (`python3 - <<EOF`) applied mechanical edits to heap.rs, and one `cargo xtask so` ran in the
foreground over ssh outside jobrun (its output was empty; the products were rebuilt by the t1/t2 jobs).
