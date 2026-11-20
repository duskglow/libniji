# PASS2-NATIVE-RUNNER: the second test pass as libniji processes, plus a small interposed suite

Branch `task/pass2-native-runner` (stacked on `task/test-pass2-native`), host keke (Zen 1, 60 vCPU), role n1.
Decision record D-X.135 (reserved). Plants: `handoff/PASS2-NATIVE-RUNNER-plants.txt`.

## What changed

* `xtask/src/pass2.rs` (new): per-tree root `/tmp/niji2-<8 hex of sha256(tree path)>` (25 bytes + NUL, fits the
  28-byte `PT_INTERP` of the host interpreter path); `prepare` builds the loader with that prefix
  (`so::build_native_ld`, cargo target `target/product/ld-native`, used by nothing else: finding #6821), links
  `target/niji/*.so*` and `cc -print-file-name=libgcc_s.so.1` into `<root>/lib/`, copies this xtask as `<root>/xtask`;
  `patch_listed` lists the test executables (`cargo test --no-run --message-format json`, test profile only) and
  writes each copy to `target/debug/niji-deps/<same name>` with `PT_INTERP` rewritten and a stamp (binary length +
  mtime ns); `pass2-exec` (the cargo runner) refuses a missing copy, one older than its binary, or one whose stamp
  is not the binary's present one. `pass2::lock`: an exclusive `flock` of `target/.niji2.lock`, taken before
  `prepare` and held to the end of the native test run (both `test_steps` and `affected`), so a second native
  pass in the same tree waits instead of removing the first one's root (leg 2: a `test-pass2` started beside a
  gate did remove it; finding #6821). Unit tests: 7 (`pass2::tests`). Not tested: that `affected` holds the
  lock to the end of its run (no plant on that line).
* `main.rs`: `test_steps` = pass 1, then `native_tests` (steps `tests-native-setup`, `-copies`, `-unchanged-scan`,
  `-nextest`; same PASS0_ONLY / PASS1_SHARD0 / run-once filters as the old interposed pass), then
  `interposed_suite` (`-p niji-boundary --lib --test ...`, `-p niji --test <_clauses> ...`, then niji-boundary's
  doctests, all with `niji-boundary/export,niji-boundary/selftest`). New `cargo xtask test-pass2 [extra]`: the
  second pass alone (used for the numbers below). Native processes get `NIJI_TEST_PASS=native`.
* `affected.rs`: pass 1 split into native (every selected target of the four selftest crates, as before) and
  interposed (the suite's targets among them only); products built first when a second pass runs.
* `nextest.rs`: `INTERPOSED_FEATURES`, `NATIVE_ONLY`, `interposed()`, `interposed_runs()`, `run_with` (env).
* Tests: `crates/boundary/tests/pass2_native_process.rs` (the native pass maps `/tmp/niji2-*/ld.so` and libniji's
  `libc.so.6` and runs the copy; any other pass maps no prefixed loader), `heap_foreign_pair_cost.rs` (the foreign
  `malloc(64)`/`free` pair, counted by a C launcher that attaches a `perf_event_open` counter with
  `inherit|enable_on_exec` to a re-executed child; per pair = count difference of 201000 and 1000 pairs; tests may
  not use `unsafe`, so no in-process counter). `docs/architecture.md` row updated.

## The interposed suite, target by target (step 2)

Criterion (measured, `nm` of every `export`-build binary of both packages, job `pass2-native-runner-classify`): a
target reaches `export`-only code when its source checks `feature = "export"` or its binary links the C-family
forwarders `niji_caller_*` (with `export`, `caller_alloc::reallocarray` goes by symbol and C-family blocks come from
the poisoned test heap in the binary; natively from the products' unpoisoned heap). `heap.rs` `DEBUG`'s `VIRGIN` bit
(export only) matters for `mcheck`: `s03_memory_symbols`/`s03_memory_clauses` link the forwarders anyway. `ld.rs`'s
`export` difference is only which libniji definition of the memory symbols links, so `s27_dynlink_*` move.
niji-boundary's unit tests stay (the lib's own `cfg(feature = "export")` code). New boundary targets join the
suite by default (`heap_foreign_pair_cost`, `pass2_native_process` are in it).

| Target | Links `niji_caller_*` (export build) | Checks `feature = "export"` | Pass |
|---|---|---|---|
| `niji-boundary (unit tests)` | yes | no | native + interposed |
| `niji-boundary::appendix_a_primitives` | yes | no | native + interposed |
| `niji-boundary::casecmp_avx2` | yes | no | native + interposed |
| `niji-boundary::classed_slots` | yes | no | native + interposed |
| `niji-boundary::csys_link` | yes | no | native + interposed |
| `niji-boundary::csys_veneer_abi` | no | no | native only |
| `niji-boundary::dl_tls_threads` | no | no | native only |
| `niji-boundary::erf_quick` | yes | no | native + interposed |
| `niji-boundary::exp_quick` | yes | no | native + interposed |
| `niji-boundary::export_macro` | yes | no | native + interposed |
| `niji-boundary::failinject` | yes | no | native + interposed |
| `niji-boundary::failinject_products` | no | no | native only |
| `niji-boundary::find_seq_scan` | yes | no | native + interposed |
| `niji-boundary::heap_threads` | yes | yes | native + interposed |
| `niji-boundary::hyperb_quick` | yes | no | native + interposed |
| `niji-boundary::invhyp_quick` | yes | no | native + interposed |
| `niji-boundary::libm_fma_select` | no | no | native only |
| `niji-boundary::locale_ctype_l` | yes | no | native + interposed |
| `niji-boundary::log_quick` | yes | no | native + interposed |
| `niji-boundary::machine_scanners` | yes | no | native + interposed |
| `niji-boundary::math_fma_paths` | yes | no | native + interposed |
| `niji-boundary::math_kernel_no_invalid` | yes | no | native + interposed |
| `niji-boundary::modern_calls` | yes | no | native + interposed |
| `niji-boundary::native_v1` | yes | no | native + interposed |
| `niji-boundary::niji_cc_lm` | no | no | native only |
| `niji-boundary::nscache` | yes | no | native + interposed |
| `niji-boundary::placement_sweep` | no | no | native only |
| `niji-boundary::pow_quick` | yes | no | native + interposed |
| `niji-boundary::qsort_alloc_fail` | yes | no | native + interposed |
| `niji-boundary::regex_dfa` | yes | no | native + interposed |
| `niji-boundary::regex_dfa_signal_fork` | yes | no | native + interposed |
| `niji-boundary::regex_enomem` | yes | no | native + interposed |
| `niji-boundary::regex_paths` | yes | no | native + interposed |
| `niji-boundary::regex_reentry` | yes | no | native + interposed |
| `niji-boundary::reload` | yes | no | native + interposed |
| `niji-boundary::rseq_node` | yes | no | native + interposed |
| `niji-boundary::s00_conventions_clauses` | yes | no | native + interposed |
| `niji-boundary::s01_startup_symbols` | yes | no | native + interposed |
| `niji-boundary::s02_diagnostics_symbols` | yes | no | native + interposed |
| `niji-boundary::s03_memory_symbols` | yes | no | native + interposed |
| `niji-boundary::s04_strings_symbols` | yes | no | native + interposed |
| `niji-boundary::s05_ctype_symbols` | yes | no | native + interposed |
| `niji-boundary::s06_multibyte_symbols` | yes | no | native + interposed |
| `niji-boundary::s07_locale_symbols` | yes | no | native + interposed |
| `niji-boundary::s08_numeric_symbols` | yes | no | native + interposed |
| `niji-boundary::s09_random_symbols` | yes | no | native + interposed |
| `niji-boundary::s10_math_symbols` | yes | no | native + interposed |
| `niji-boundary::s11_lowio_symbols` | yes | no | native + interposed |
| `niji-boundary::s12_stdio_symbols` | yes | no | native + interposed |
| `niji-boundary::s13_filesystem_symbols` | yes | no | native + interposed |
| `niji-boundary::s14_search_symbols` | yes | no | native + interposed |
| `niji-boundary::s15_sockets_symbols` | yes | no | native + interposed |
| `niji-boundary::s16_nameservices_symbols` | yes | no | native + interposed |
| `niji-boundary::s17_users_symbols` | yes | no | native + interposed |
| `niji-boundary::s18_processes_symbols` | yes | no | native + interposed |
| `niji-boundary::s19_terminals_symbols` | yes | no | native + interposed |
| `niji-boundary::s20_signals_symbols` | yes | no | native + interposed |
| `niji-boundary::s21_contexts_symbols` | no | no | native only |
| `niji-boundary::s22_threads_symbols` | yes | no | native + interposed |
| `niji-boundary::s23_time_symbols` | yes | no | native + interposed |
| `niji-boundary::s24_system_symbols` | yes | no | native + interposed |
| `niji-boundary::s25_ipc_symbols` | yes | no | native + interposed |
| `niji-boundary::s26_syslog_symbols` | yes | no | native + interposed |
| `niji-boundary::s27_dynlink_symbols` | no | no | native only |
| `niji-boundary::s28_profiling_symbols` | yes | no | native + interposed |
| `niji-boundary::s29_legacy_symbols` | yes | no | native + interposed |
| `niji-boundary::s29_rpc_auth_stubs` | yes | no | native + interposed |
| `niji-boundary::s29_rpc_clnt_svc_stubs` | yes | yes | native + interposed |
| `niji-boundary::scanner_races` | yes | no | native + interposed |
| `niji-boundary::search_nomem` | yes | no | native + interposed |
| `niji-boundary::sem_named_nomem` | yes | no | native + interposed |
| `niji-boundary::setlocale_env_repeat` | no | no | native only |
| `niji-boundary::setlocale_posix_repeat` | no | no | native only |
| `niji-boundary::small_compare` | yes | no | native + interposed |
| `niji-boundary::small_copy` | yes | no | native + interposed |
| `niji-boundary::strstr_find` | yes | no | native + interposed |
| `niji-boundary::strto_internal_group` | yes | no | native + interposed |
| `niji-boundary::tb_foreign_low_words` | yes | no | native + interposed |
| `niji-boundary::tb_foreign_words` | yes | no | native + interposed |
| `niji-boundary::towctrans_desc` | yes | no | native + interposed |
| `niji-boundary::trig_quick` | yes | no | native + interposed |
| `niji-boundary::uapi_vs_abi` | no | no | native only |
| `niji-boundary::wcsstr_find` | yes | no | native + interposed |
| `niji-boundary::wordexp_block` | yes | no | native + interposed |
| `niji-boundary::wordexp_reuse` | yes | no | native + interposed |
| `niji::s01_startup_clauses` | yes | no | native + interposed |
| `niji::s02_diagnostics_clauses` | yes | no | native + interposed |
| `niji::s03_memory_clauses` | yes | no | native + interposed |
| `niji::s04_strings_clauses` | yes | no | native + interposed |
| `niji::s05_ctype_clauses` | yes | no | native + interposed |
| `niji::s06_multibyte_clauses` | yes | no | native + interposed |
| `niji::s07_locale_clauses` | yes | no | native + interposed |
| `niji::s08_numeric_clauses` | yes | no | native + interposed |
| `niji::s09_random_clauses` | yes | no | native + interposed |
| `niji::s11_lowio_clauses` | yes | no | native + interposed |
| `niji::s12_stdio_clauses` | yes | no | native + interposed |
| `niji::s13_filesystem_clauses` | yes | no | native + interposed |
| `niji::s14_search_clauses` | yes | no | native + interposed |
| `niji::s15_sockets_clauses` | yes | no | native + interposed |
| `niji::s16_nameservices_clauses` | yes | no | native + interposed |
| `niji::s17_users_clauses` | yes | no | native + interposed |
| `niji::s18_processes_clauses` | yes | no | native + interposed |
| `niji::s19_terminals_clauses` | yes | no | native + interposed |
| `niji::s20_signals_clauses` | yes | no | native + interposed |
| `niji::s21_contexts_clauses` | no | no | native only |
| `niji::s22_threads_clauses` | yes | no | native + interposed |
| `niji::s23_time_clauses` | yes | no | native + interposed |
| `niji::s24_system_clauses` | yes | no | native + interposed |
| `niji::s25_ipc_clauses` | yes | no | native + interposed |
| `niji::s26_syslog_clauses` | yes | no | native + interposed |
| `niji::s27_dynlink_clauses` | no | no | native only |
| `niji::s28_profiling_clauses` | yes | no | native + interposed |
| `niji::s29_legacy_clauses` | yes | no | native + interposed |
| `niji::s31_utilities_clauses` | no | no | native only |

Moved to native only (14): niji-boundary `csys_veneer_abi`, `dl_tls_threads`, `failinject_products`,
`libm_fma_select`, `niji_cc_lm`, `placement_sweep`, `s21_contexts_symbols`, `s27_dynlink_symbols`,
`setlocale_env_repeat`, `setlocale_posix_repeat`, `uapi_vs_abi`; niji `s21_contexts_clauses`, `s27_dynlink_clauses`,
`s31_utilities_clauses`. Every other niji-boundary target and niji `_clauses` target stays interposed (they link the
forwarders). Follow-up that would move most of the rest: set `MALLOC_PERTURB_` from `NIJI_TEST_MALLOC_PERTURB` in
the runner so C-family blocks are poisoned natively too (needs one native run to validate; not done).

## Foreign malloc/free bound

`heap_foreign_pair_cost`: 1911 instructions per pair (pass-1 build), 2063 (export build), Zen 1, test profile
(unoptimised), job `pass2-native-runner-check1`; bound 2100. In a foreign process `heap::DEBUG` is active, so every
`malloc` goes `malloc_general` -> `debug_alloc` (the fast paths are never reached): the plant `foreign-pair-slower`
slows `malloc_general` (plants4 slowed `KernelPages::cache_slot`, off that path, and SURVIVED). The native pass's
value is not in any log (nextest keeps a passing test's output; not read).

## Numbers (step 3): the second pass, before and after (keke, 4-CPU slot, test profile)

| Run | Wall (steps) | Test-seconds | Tests / binaries | Build |
|---|---|---|---|---|
| Before: interposed pass (TEST-PASS2-NATIVE) | 1448 s | 5015 | 193 binaries | 337 s whole-workspace `export` build |
| After: native pass (`measure4`) | 796 s nextest + 4 s set-up, copies, scan | 2630 | 4829 / 193 | none (loader incremental: 0.5 s) |
| After: interposed suite (`measure4`) | 457.5 s niji-boundary + 66.2 s niji + 2.1 s doc | 1913 | 4437 (+doc) | two packages with `export`: not measured (cached) |
| After: total (`cargo xtask test-pass2`) | 1326 s (-8%) | 4542 (-9%) | | the 337 s workspace build is gone; the suite's own build is unmeasured |

Test-seconds = the sum of nextest's per-test `[ x.xxxs]` of each run. `measure4` ran beside `plants4` on the same
host (the before run's load is not recorded), so the wall difference is indicative only; the test-seconds are the
firmer number. Most of the interposed suite stays because its binaries link the `niji_caller_*` forwarders; the
follow-up above (poisoning C-family blocks natively) would move most of it to the native pass.
`measure4` itself exited 1: one test failed in the interposed suite, see INFORMATIONAL.

## INFORMATIONAL

* `niji-boundary::s03_memory_symbols sym_mallinfo` failed once in the interposed suite (`measure4`):
  `mallinfo().uordblks` 4496 vs `mallinfo2().uordblks` 4528, two calls apart. The test compares two reads in a
  process that is not quiescent (32 bytes were allocated between them); `sym_mallinfo2` in the same file already
  allows for that noise. The target and features are the same as in the old interposed pass; this change does not
  touch it. Recommend: compare within one quiescent window (retry until two `mallinfo` reads around the
  `mallinfo2` read agree) in a separate story.

## State (leg 2, 03:45 UTC)

* HEAD = the pushed head of `task/pass2-native-runner`; role keke:~/w/n1 synced to it.
* Proving jobs (keke): `pass2-native-runner-gate4` (direct tier, `affected: ok`, rc 0; `FULL: the train's` for the
  xtask files, the bounded set ran), `pass2-native-runner-check4` (clippy all targets/features, static, xtask
  `pass2::`/`nextest::` tests: ok), `pass2-native-runner-measure4` (the numbers above; rc 1: the INFORMATIONAL
  failure), `pass2-native-runner-plants6` (13 plants, after the plant fix; plants4 had 12 CAUGHT, 1 SURVIVED;
  plants5 applied none: the fixed line had lost a field separator, baseline rc 127).
* SUPERSEDED: leg 1's `-gate`, `-plants`, `-gate2`, `-plants2`, `-measure` (leg 1: runner via `cargo xtask nextest`,
  unsynced tree), `-check1` (static red, fixed); `-gate3` (green, head before the lock); `-plants3` (killed for the lock change, 8 CAUGHT);
  `-measure3` (its `jobwait` returned 3 after ~10 min and it ran beside gate3: products half-built, the native
  set-up failed and removed the gate's root; this is what the lock now prevents); `-plants4` (1 SURVIVED); `-plants5` (no plant applied: a malformed plants line).

## Exact next step

1. `~/bin/jobwait pass2-native-runner-plants6` (started 03:43). If not green: read its SURVIVED line, fix, rerun.
2. `cargo xtask precheck --jobs <labels> --head <sha>` on the role, then `ops handback-check`.

## RULING NEEDED

* Switching the gate's pass 2 to this (`test_steps` and `affected` do it in-tree; any liella-ops gate or CI
  configuration change is the coordinator's): numbers above. Recommend: land; then the train's FULL gate is the
  first full native pass under CI.
* The CI host (mei) story gate the brief asks for: needs a role on mei, which this run does not have (CI roles are
  off-limits). Recommend: the dispatcher's CI run of this branch is that gate.
* Coordinator note 01:44 (one prefixed loader build for hot_text_products and the runner): not possible as is: the
  prefix is compiled in (`option_env!("NIJI_PREFIX")`); the coexist test needs `/usr/lib/niji` (read-only, not ours
  to populate with `lib/`), the native pass needs a writable per-tree root. Sharing one build needs a loader that
  takes its prefix at run time (a loader trust-boundary change). Recommend: keep two builds (the native one is
  incremental after the first gate in a tree).

## Not to redo

* `cargo xtask nextest` with `CARGO_TARGET_..._RUNNER` set (cargo runs xtask through it). Starting jobs right after
  a sync without checking rc (rc 4 = nothing synced). A job script that calls `~/bin/jobwait` once: it returns 3
  after about 10 minutes while the job runs; loop until it is not 3.
