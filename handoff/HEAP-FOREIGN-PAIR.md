# HEAP-FOREIGN-PAIR: a foreign thread's malloc/free pair, 395 -> 348 instructions (step 1); step 2 needs a ruling

State: branch `task/heap-foreign-pair` on origin/next c1413a3c4; role keke ~/w/n2. Jobs' head 69e15b572 (code
b458752ef + record); this note, the report and the plants results are committed after it (handoff/ only).
Decisions in force: D-X.60, D-X.58, D-X.134, D-X.139 (new, reserved id, amends D-X.134).

## Step 1 (done): the plain route for FOREIGN alone
`heap::debug_alloc`/`debug_free`: when `begin()` returns exactly `malloc_debug::FOREIGN` (no debugging on), they call
`foreign_alloc`/`foreign_free` (cold, out of line). Those call `HEAP.alloc`/`HEAP.free` with
`heap::foreign::ForeignPages(slot)`: `KernelPages` with the thread's cache slot from `thread::heap_cache_slot()`,
found once per operation and read with one load. No slot (registry full): `alloc_slow`/`free_slow` with
`KernelPages`, as before. The route skips `Caller::new`, `pedantic`, `Debug::alloc`/`free` and the registry probe
at every cache-word read. `malloc_entry`, `free_entry`, `malloc_general`, `free_general` and the `c_*` paths are
unchanged. `ForeignPages` sits in a nested module so the impl numbers of `heap` (hot-text keys) do not move.
Out of line on purpose: inlined into `debug_alloc` (a start-up function: the first allocation clears VIRGIN) the
route was 23 instructions cheaper per pair (325), but gate2 failed WINDOW-BIN-BYTES (float 2/1, locale 3/2, tool
3/2 windows) in `cargo xtask so`.

## Counts (keke, Zen 1 host 60 vCPU; user instructions per pair, loop included)
| row | next c1413a3c4 | inlined (7c2d284bd) | final (b458752ef) |
|---|---|---|---|
| foreign malloc(64)/free, release, export,selftest | 395 | 325 | 348 |
| foreign pair, release, no features | 389 | 319 | 341 |
| foreign pair, test profile, export,selftest | 1672 | 1602 | 1641 |
| foreign pair, test profile, no features | 1662 | 1591 | 1632 |
| runtime-made pair, cbench_malloc malloc_free_16/32/64/128 (glibc 147.2) | 122.2 | path unchanged | path unchanged, not re-run |
| dynamic start-up (empty main / 3 libs) | not measured | - | gate4's WINDOW-BIN-BYTES, `cargo xtask test`, hot_text_products |
Source: `heap_foreign_pair_cost` with `--nocapture` under each profile (scratch keke ~/work/heap-foreign-pair/count.sh);
cbench_malloc: `cargo xtask bench --compare --counters --runs 3 cbench_malloc`. No DYN-START execcount scripts exist on
this host. Release profile after the inlined version (perf record, instructions:u): debug_alloc 32%, debug_free 29%,
the registry probe `foreign_heap_cache_slot` 18% (two calls per pair), malloc_general 11%.
bench/instr_limits.tsv: the malloc_free_* rows are already at 125 for 122.2 (count + 2%), so nothing to tighten.
`heap_foreign_pair_cost`'s PAIR_BOUND (2100) is not tightened. Its comment's gate counts (1911/2063) differ from
what plain `cargo test` measures here (1672 before), so the gate's own count is needed first.

## Step 2: RULING NEEDED (not started)
348 is 2.9x the runtime-made pair, above the brief's 1.5x (183). What is left is the call chain (export -> general
path -> debugging path -> route -> slot lookup) and one registry probe per operation. Getting near 183 needs the cache
word in one load on a foreign thread: D-X.58 option 4, a PT_TLS block in libc.so.6. Its start-up cost was not
measured. My attempt to time one extra TLS module under libniji's loader failed: libniji's ld.so found no libc.so.6
for an ad-hoc program through DT_RPATH/RUNPATH, and LD_LIBRARY_PATH leaks into the host tools. Recommendation: rule
first whether a PT_TLS segment in libc.so.6 is acceptable. It adds one static TLS module to every libniji-started
dynamic process, so it cannot be free at start-up. Measure that with DYN-START-PARITY's execcount scripts (Skylake-SP
host) before building it.

## Step 3: not done
`foreign_words()` (errno and locale words, a `gettid` per call) was not measured: the budget went to step 1's proof
and the layout fix. Next: `strace -f -c -e trace=gettid` on pass-2 test binaries (strstr_find, regex_dfa, a stdio test).

## Tests and plants
New: crates/boundary/tests/heap_foreign_plain.rs. On the route it checks that calloc zeroes a dirty reused block,
that a free reaches the heap (the next same-size block is the freed one), and that with M_PERTURB set the route is not
taken. Plants: handoff/HEAP-FOREIGN-PAIR-plants.txt (4). `-plants` (3f5eb3708): 3 CAUGHT, free-dropped SURVIVED (a leak
does not move the cost bound), so I added the reuse assertion. `-plants2` (bf19d27a3, route inlined): 4 of 4 CAUGHT,
no WARN. `-plants4` runs on the final head.

## Jobs (keke)
Final: `heap-foreign-pair-gate4` (affected --run --base origin/next, then `cargo xtask test`, NIJI_FULL_PROFILE so,
hot_text_products) and `heap-foreign-pair-plants4`, started 10:10Z on 69e15b572. Superseded: `-gate` (stopped: head
changed), `-gate2` (RED: WINDOW-BIN-BYTES with the route inlined; fixed), `-gate3`/`-plants3` (stopped after a record
edit), `-plants`, `-plants2` (older heads), `-lint`/`-lint2` (static findings in the new test, fixed), `-lint3`
(clippy, static, both tests rc 0 on 3f5eb3708), and the counts `-base`, `-c1`, `-c2`, `-prof`, `-tlsprobe*` (failed
probe).

## Next step
If gate4 or plants4 is red, fix what it names and rerun that suite only (precheck was not run: gate4 holds the role).
Then the step-2 ruling, then step 3's measurement. Do not redo the counts or the profile above.
