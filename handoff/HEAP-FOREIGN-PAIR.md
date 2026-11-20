# HEAP-FOREIGN-PAIR: a foreign thread's malloc/free pair, 395 -> 325 instructions (step 1); step 2 needs a ruling

State: branch `task/heap-foreign-pair` on origin/next c1413a3c4; role keke ~/w/n2. Code head bf19d27a3 (the jobs'
head); this note and the report are committed after it (handoff/ only).
Decisions in force: D-X.60, D-X.58, D-X.134, D-X.139 (new, reserved id, amends D-X.134).

## Step 1 (done): the plain route for FOREIGN alone
`heap::debug_alloc`/`debug_free`: when `begin()` returns exactly `malloc_debug::FOREIGN` (no debugging on), they call
`HEAP.alloc`/`HEAP.free` with `heap::foreign::ForeignPages(slot)`: `KernelPages` with the thread's cache slot from
`thread::heap_cache_slot()` found once per operation, read with one load. No slot (registry full): `alloc_slow`/
`free_slow` with `KernelPages`, as before. Skips `Caller::new`, `pedantic`, `Debug::alloc`/`free` and the per-read
registry probe. All new code sits in the two cold functions; `malloc_entry`, `free_entry`, `malloc_general`,
`free_general` and the `c_*` paths are unchanged. `ForeignPages` is in a nested module so the impl numbers of
`heap` (hot-text keys) do not move.

## Counts (keke, Zen 1 host 60 vCPU; user instructions per pair, loop included)
| row | next c1413a3c4 | step 1 (7c2d284bd, same code as bf19d27a3) |
|---|---|---|
| foreign malloc(64)/free, release, export,selftest | 395 | 325 |
| foreign pair, release, no features | 389 | 319 |
| foreign pair, test profile, export,selftest | 1672 | 1602 |
| foreign pair, test profile, no features | 1662 | 1591 |
| runtime-made pair, cbench_malloc malloc_free_16/32/64/128 (glibc 147.2) | 122.2 | code path unchanged, not re-run |
| dynamic start-up (empty main / 3 libs) | not measured: no DYN-START execcount scripts on this host | gate's `cargo xtask test` + hot_text_products |
Source: `heap_foreign_pair_cost` with `--nocapture` under each profile (scratch script keke
~/work/heap-foreign-pair/count.sh); cbench_malloc by `cargo xtask bench --compare --counters --runs 3 cbench_malloc`.
Profile of the release pair after step 1 (perf record, instructions:u): debug_alloc 32%, debug_free 29%, the
registry probe `foreign_heap_cache_slot` 18% (two calls per pair), malloc_general 11%, malloc 3.5%, cfree 2.9%.
bench/instr_limits.tsv: the malloc_free_* rows are already at 125 for 122.2 (count + 2%): nothing to tighten.
`heap_foreign_pair_cost`'s PAIR_BOUND (2100) is not tightened: its comment's gate counts (1911/2063) differ from
what plain `cargo test` measures here (1672 before), so the gate's own count is needed first.

## Step 2: RULING NEEDED (not started)
325 is 2.7x the runtime-made pair, above the brief's 1.5x (183). What is left is the call chain (export ->
general path -> debugging path -> slot lookup) and one registry probe per operation. Getting near 183 needs the
cache word in one load on a foreign thread: D-X.58 option 4, a PT_TLS block in libc.so.6. Cost not measured: an
attempt to time one extra TLS module at start-up under libniji's loader failed (libniji's ld.so found no libc.so.6
for an ad-hoc program through DT_RPATH/RUNPATH; LD_LIBRARY_PATH leaks into the host tools). Recommendation: rule
first whether a PT_TLS segment in libc.so.6 is acceptable. It changes every libniji-started dynamic process's TLS
layout (one more static TLS module laid out by the loader), so it cannot be free at start-up. Measure it with
DYN-START-PARITY's execcount scripts on the Skylake-SP host before building it.

## Step 3: not done
`foreign_words()` (errno, locale words; a `gettid` per call) was not measured: the budget went to step 1's proof.
Next: `strace -f -c -e trace=gettid` on pass-2 test binaries (strstr_find, regex_dfa, a stdio test).

## Tests and plants
New: crates/boundary/tests/heap_foreign_plain.rs (on the route: calloc zeroes a dirty reused block, a free reaches
the heap, the next same-size block is the freed one; with M_PERTURB set the route is not taken). Plants:
handoff/HEAP-FOREIGN-PAIR-plants.txt (4). First run (`-plants`, head 3f5eb3708): 3 CAUGHT, free-dropped SURVIVED
(the cost bound does not see a leak); the reuse assertion was added for it; `-plants2` runs on bf19d27a3.

## Jobs (keke)
Final: `heap-foreign-pair-gate2` (affected --run --base origin/next, then `cargo xtask test`, NIJI_FULL_PROFILE so,
hot_text_products), `heap-foreign-pair-plants2`. Superseded: `-gate` (stopped: head changed), `-plants` (3 of 4),
`-lint`, `-lint2` (static findings in the new test, fixed), `-lint3` (clippy, static, both tests: rc 0 on 3f5eb3708),
`-base`, `-c1`, `-prof` (counts), `-tlsprobe*` (failed probe, see step 2).

## Next step
If gate2/plants2 are red: fix the named test, rerun that suite only. Then the step-2 ruling, then step 3's
measurement. Not to redo: the counts and profile above.
