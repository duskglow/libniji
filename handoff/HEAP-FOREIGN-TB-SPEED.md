# HEAP-FOREIGN-TB-SPEED: foreign threads' heap calls ~2x slower since TB-FOREIGN-WORDS-LOW: proven, fixed forward

State: branch `task/heap-foreign-tb-speed`, code head 6bd094edf (on origin/next d496d4c25); role keke ~/w/n3.
Decisions in force: D-X.58, D-X.60, D-X.134 (new, reserved id, amends D-X.60).

## Proof (step 1; keke, Zen 1, perf stat, one run each; scripts keke ~/work/HEAP-FOREIGN-TB-SPEED/ab.sh, cnt.sh, cnt2.sh)
A = 18835920^1 (2d5625a3e), B = 18835920 (merge of TB-FOREIGN-WORDS-LOW-14), C = next d496d4c25, D = C + the thread.rs
fix only. Tests: strstr_find and heap_threads (named by #6772) plus wcsstr_find, scanner_races, regex_dfa (chosen by
me: #6772's per-test data was not available to this run). Pass 2 = `--features niji-boundary/export,selftest`,
NIJI_TEST_MALLOC_PERTURB=165, as the gate. Instructions user / kernel (G), wall s:

| test, pass 2 | A | B | C (next) | D (fix) |
|---|---|---|---|---|
| strstr_find | 1354 / 18.5, 386 | 1427 / 278.1, 560 | 1435 / 278.4, 553 | 1406 / 18.7, 395 |
| heap_threads | 803 / 51.3, 79 | 953 / 146.8, 81 | 998 / 151.8, 80 | 1110 / 50.5, 82 |
| wcsstr_find | 171 / 0.40, 50 | 183 / 37.2, 73 | 183 / 37.2, 74 | 180 / 0.40, 50 |
| scanner_races | 0.21 / 0.27 | 0.60 / 0.42 | 0.52 / 2.6 | 0.34 / 2.0 |
| regex_dfa | 0.23 / 0.015 | 0.24 / 0.018 | 0.23 / 0.018 | 0.23 / 0.016 |

Pass 1 (same tests): strstr_find, wcsstr_find, regex_dfa unchanged A..D; heap_threads kernel 38.0G (A) -> 105.4G (B),
103.1G (C) -> 34.0G (D) (its selftest heap calls are foreign too). heap_threads user counts move with contention
(spin), +-10% run to run. scanner_races is sub-second and noisy.

## Mechanism (step 2)
In a process libniji did not start, `malloc_debug::FOREIGN` stays set, so every heap call takes the debugging path
(`debug_alloc`/`debug_free`) and `KernelPages`, whose cache word went through `thread::foreign_words()`: a `gettid`
system call per call (the TID test that tells a new thread on a reused block from the old one). Not a correctness
need for the cache word: `clear_words` keeps it on purpose and `own` is only set by `claim_own`..`release_own`.
Per foreign malloc/free pair (zz_foreign_count, scratch only, never committed): debug test build A 1221 user + 2.3
kernel, C 2063 + 757, D 1997 + 4; release build C 773 + 754, D 731 + 1, final head 396 + 0.6. The release profile of
D (perf record): 45% in fill/copy of `heap::Caller`'s 256-byte buffer (zeroed and moved on every debugging call,
used only for MALLOC_TRACE), 12% the registry probe.

## Fix (step 3), commits 4341587e5, bef3dc6f3, 5b780f40c + 6bd094edf (record)
1. `thread::foreign_heap_cache_slot`: registry entry by thread pointer and its `own` flag, no `gettid`.
2. `heap::Caller` holds its buffer only when TRACE is set (trace lines unchanged).
Test: crates/boundary/tests/heap_foreign_nosys.rs (forked child in strict seccomp mode runs warm malloc/free on a
foreign thread; any system call kills it); helper `selftest::in_child_no_syscalls`. TB-FOREIGN-WORDS-LOW tests pass
(tb_foreign_low_words, tb_foreign_words, both passes).

## Bound (step 4): NOT reached
Foreign pair 396 user instructions (release) vs about 122 for a runtime-made thread (bench/instr_limits.tsv
`malloc_free_*`, inherited, not re-measured: the wl_percall job held the role and was stopped). The rest is the
general route (`debug_alloc` -> `Debug::alloc` -> `Heap::alloc` with out-of-line cache-word calls) and the registry
probe. RULING NEEDED: per-thread storage for foreign threads (D-X.58 option 4, ELF TLS, ruling #4212 said "only if
the foreign path's cost ever matters"); recommend deciding it now with these numbers. Cheaper next step without a
ruling (not measured): in `debug_alloc`/`debug_free`, when the flags are exactly FOREIGN, call the heap's fast paths
with `KernelPages` directly (skips `Debug::alloc`), kept in the cold functions.

## Jobs (keke)
Final head 6bd094edf: `heap-foreign-tb-speed-gate5` (affected --run, then `cargo xtask test`, NIJI_FULL_PROFILE so,
hot_text_products), `heap-foreign-tb-speed-plants5` (4 plants), started 20:21Z. Foreground on 6bd094edf: workspace
clippy rc 0 (on 5b780f40c, same code), `cargo xtask static` rc 0. Counts: `-ab3-a`..`-ab3-d`, `-rcount`,
`-rcount2` (rc 0). Plants on 6dfaca990 (`-plants`, 3 of 3 CAUGHT, no WARN; superseded). SUPERSEDED: `-gate`, `-gate2`,
`-gate3`, `-gate4`, `-plants2..4` (stopped: a later commit, or started on a stale sync), `-percall` (stopped: it held
the role), `-ab`, `-ab2-*` (wrong crate names / stopped for time).

## Next step
`~/bin/jobwait heap-foreign-tb-speed-gate5 heap-foreign-tb-speed-plants5`; fix any red; copy plants5's results.txt
into handoff/HEAP-FOREIGN-TB-SPEED-plants.txt; `cargo xtask precheck --jobs <labels> --head <sha>` on n3;
`ops handback-check`; hand back.

## Not to redo
The A/B and the release counts above; the profile. Scratch trees /dev/shm/claude/HEAP-FOREIGN-TB-SPEED/{a,b,c,d,e}.
