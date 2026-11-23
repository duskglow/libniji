# HEAP-FIND-DEPTH-NEED: handoff

State: branch `task/heap-find-depth-need` (base origin/next `3e668ecce`), code head `92dd7b303` (fix `adb8aae13`,
clippy fix `85ac05b59`, locale in the test `92dd7b303`); later commits are prose (D-row, handoff, report). Role keke
`~/w/n3` (Zen 1). Decision D-3.25 (reserved id).

## Cause (attributed)

`map_aligned_exactly` (crates/boundary/src/heap.rs) bounds blockers per round at `EXACT_BLOCKERS`. It was 8, sized
for a first segment's 4352 KiB (needs 5). HEAP-SECOND-NODE-MAP (D-3.24) made every later segment 2304 KiB (metadata
and 8 slabs), which needs ceil((16384 + 2304) / 2304) = 9. Under RLIMIT_AS the 32 MiB over-mapping of `map_room`
fails, the exact path probes, the aligned up/down slots are taken (by find's locale and library mappings), and with
9 > 8 it gives up at once: no later segment, the heap maps per request. strace of find under `ulimit -v 48200` (mei):
22.3 places its third segment (4352) after two blocked rounds; 22.4 loops on `mmap 32768 FAIL / 2304 probe / munmap
/ 2304 FAIL x2`. Without a limit both runs map the same (peak 102360 vs 102380 KiB). STDIO-CALLER-ALLOC and the
other merges were not needed for the attribution (one cause explains the delta; the fix removes all of it).

The locale is part of the case (leg 2): with an empty environment (C locale) find over d200k needs 31792 (22.3) and
30080 (22.4) KiB over d3 on keke, and 8 blockers suffice; with `LANG=en_US.UTF-8` 34784 and 37344. The first test
cleared the environment, so plant `blockers-8` SURVIVED (plants-1); the test now sets `LANG=en_US.UTF-8`.

## Numbers (least `ulimit -v`, KiB; fixed layout; node 0 pinned; host find 4.9.0; dirs on tmpfs; en_US.UTF-8)

| build | d3 | d200k | over d3 |
|---|---|---|---|
| 22.3 deb (keke) | 13312 | 48096 | 34784 |
| 22.4 deb (keke) | 13280 | 50624 | 37344 (+2560) |
| next + fix, role build, the test itself (keke, 2 of 2) | 15296 | 48592 | **33296** (-1488 vs 22.3) |
| 22.3 / 22.4 debs on mei | 13296 / 13264 | 48096 / 50624 | 34800 / 37360 |

The snapshot's root run (find 4.10.0) showed +4720; on the hosts' find 4.9.0 the same cause gives +2560. The role
build's d3 need is larger than the debs' (+1984; a different build, not investigated; the brief's metric is the need
over d3). Static oracle (200000 x malloc(264), held) does not reach the case (63824 KiB on 22.3, 22.4 and after).

## Test and plant

`find_over_200000_entries_bounds_its_address_space` (crates/core/tests/heap_meta_products.rs): system `find`
through the products' ld.so `--library-path`, `LANG=en_US.UTF-8` only, dirs under `/dev/shm/niji-find-entries-<pid>`
(removed on drop), bound 33296 over d3. Skips (printed) without /usr/bin/find, /dev/shm or the locale archive.
Plant `blockers-8` (EXACT_BLOCKERS back to 8): handoff/HEAP-FIND-DEPTH-NEED-plants.txt.

## Jobs (keke) and results: see handoff/reports/HEAP-FIND-DEPTH-NEED.json

- Final: `heap-find-depth-need-gate2` (clippy, static, `affected --run`, heap_meta_products, `cargo xtask test`,
  full-profile `so` + hot_text_products) and `heap-find-depth-need-plants2` (plants-2), on `92dd7b303`.
- Superseded: `-gate` (clippy red: arithmetic in the bisection), `-plants` (blockers-8 SURVIVED: C locale).
- Diagnostics: `-meas2` (the test's own number, 2 runs), `-env`, `-env2` (locale probe), `-meas`, `-cmp`, `-t2`.

## Not measured

The node-change point (41552) and single node (41296) are bounded by `the_heap_bounds_address_space_after_a_node_change`
and `the_heap_keeps_address_space_near_its_use` (run in the gate's heap_meta_products). cbench_malloc not run: the
constant is read only in `map_aligned_exactly`, reached after an mmap failed under RLIMIT_AS, which no cbench row does.

## Exact next step

None if gate2 and plants2 are green (hand back COMPLETE). If plants2 survives again: the locale archive on the host
may lack en_US.UTF-8; read the plant log's `find needs` line.
