# HEAP-FIND-DEPTH-NEED: handoff

State: branch `task/heap-find-depth-need` (base origin/next `3e668ecce`), code head `adb8aae13` (the fix, the test,
D-3.25, plants file); first commit `cd9ed08` is its WIP. Role keke `~/w/n3` (Zen 1). Decision D-3.25 (reserved id).

## Cause (attributed)

`map_aligned_exactly` (crates/boundary/src/heap.rs) bounds blockers per round at `EXACT_BLOCKERS`. It was 8, sized
for a first segment's 4352 KiB (needs 5). HEAP-SECOND-NODE-MAP (D-3.24) made every later segment 2304 KiB (metadata
and 8 slabs), which needs ceil((16384 + 2304) / 2304) = 9. Under RLIMIT_AS the 32 MiB over-mapping of `map_room`
fails, the exact path probes, the aligned up/down slots are taken (by find's library mappings), and with 9 > 8 it
gives up at once: no later segment, the heap maps per request. strace of find under `ulimit -v 48200` (mei): 22.3
places its third segment (4352) after two blocked rounds; 22.4 loops on `mmap 32768 FAIL / 2304 probe / munmap /
2304 FAIL x2`. Without a limit both runs map the same (peak 102360 vs 102380 KiB). STDIO-CALLER-ALLOC and the other
merges were not needed for the attribution (one cause explains the delta; the fix removes all of it).

## Numbers (least `ulimit -v`, KiB; fixed layout; node 0 pinned; host find 4.9.0; dirs on tmpfs)

| build | d3 | d200k | over d3 |
|---|---|---|---|
| 22.3 deb (keke) | 13312 | 48096 | 34784 |
| 22.4 deb (keke) | 13280 | 50624 | 37344 (+2560) |
| next + fix, role build (keke) | 15296 | 48592 | **33296** (-1488 vs 22.3) |
| 22.3 / 22.4 debs on mei | 13296 / 13264 | 48096 / 50624 | 34800 / 37360 |

The snapshot's root run (find 4.10.0) showed +4720; on the hosts' find 4.9.0 the same cause gives +2560.
The role build's d3 need is larger than the debs' (+1984; a different build, not investigated; the brief's metric is
the need over d3). Static oracle (200000 x malloc(264), held) does NOT reach the case: 63824 KiB on both 22.3 and
22.4 and after (its aligned neighbours are free), so it was dropped; the test runs the real find.

## Test and plant

`find_over_200000_entries_bounds_its_address_space` (crates/core/tests/heap_meta_products.rs): system `find`
through the products' ld.so `--library-path` (LD_LIBRARY_PATH would reach the shell), dirs under
`/dev/shm/niji-find-entries-<pid>` (removed on drop; the same walk on ext4 needed 3247 KiB less, so tmpfs), bound
33296 over d3. Skips (printed) without /usr/bin/find or /dev/shm. Plant `blockers-8` (EXACT_BLOCKERS back to 8)
in handoff/HEAP-FIND-DEPTH-NEED-plants.txt. A first run of the test on ext4 (before the tmpfs move) read 30049 over.

## Jobs (keke)

- `heap-find-depth-need-gate`: clippy, static, `affected --run`, heap_meta_products, `cargo xtask test`,
  full-profile `so` + hot_text_products, on `adb8aae13`. Started 02:40Z.
- `heap-find-depth-need-plants`: plants-1 in /dev/shm/claude/heap-find-depth-need/plants-1.
- Diagnostics (not proof): heap-find-depth-need-meas, -cmp, -t2 (the last two on older test text).

## Exact next step

1. `~/bin/jobwait heap-find-depth-need-gate heap-find-depth-need-plants` on keke; fix any red (if the test's own
   number is not 33296 on tmpfs, read its `find needs` line and set the bound to it, then rerun gate/plants).
2. Not yet measured: the node-change point (41552) and single node (41296) — the gate's heap_meta_products runs
   both bounds (the_heap_keeps_address_space_near_its_use, the_heap_bounds_address_space_after_a_node_change; keke
   has 2 nodes). cbench_malloc: the changed constant is read only on `map_aligned_exactly`'s path (an mmap that
   failed under RLIMIT_AS), which no cbench row reaches; if wanted, `cargo xtask bench --compare --counters
   cbench_malloc` as a jobrun `bench` job.
3. Write handoff/reports/HEAP-FIND-DEPTH-NEED.json (+ "jobs"), precheck `--jobs heap-find-depth-need-gate,
   heap-find-depth-need-plants --head <sha>`, handback-check, hand back.
