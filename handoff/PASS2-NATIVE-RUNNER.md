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
  is not the binary's present one. Unit tests: 6 (`pass2::tests`).
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
(unoptimised), job `pass2-native-runner-check1`; bound 2100. Native-pass value: from gate3 (read its log).

## State (leg 1 ended 02:01 UTC, REFRESH)

* HEAD = the pushed head of `task/pass2-native-runner`; role keke:~/w/n1 synced to it (tree 38eda943...).
* Running on keke (started 02:01): `pass2-native-runner-gate3` (gate: `cargo xtask affected --run`, direct tier;
  `affected --list` says `FULL: the train's` for the xtask files, bounded set runs), `pass2-native-runner-plants3`
  (`--dir /dev/shm/claude/pass2-native-runner/plants-3`, 12 plants), `pass2-native-runner-measure3` (waits for gate3,
  then `cargo test --workspace --no-run` timed apart, then `cargo xtask test-pass2`: the after numbers).
* SUPERSEDED: `pass2-native-runner-gate`, `-plants` (the native step ran `cargo xtask nextest` through the runner:
  cargo runs xtask itself via the target runner; fixed by `pass2::RUNNER_VIA`, and the runner now strips both
  variables from the test's environment), `-gate2`, `-plants2`, `-measure` (started on an unsynced tree, killed).
  `pass2-native-runner-check1`: clippy ok, xtask unit tests ok, bound measured (failed at the provisional 400),
  static red on opaque-audit (fixed: `// opaque-ok`).

## Exact next step

1. `~/bin/jobwait pass2-native-runner-gate3 pass2-native-runner-plants3 pass2-native-runner-measure3`; read
   `~/logs/<label>.log` (grep `affected:`, `FAILED`, `foreign malloc`, plant verdicts, `step:` lines, `test-pass2:`).
2. Fix what is red (only this change's tests). Check the native pass's `pass2_native_process` and the bound passed.
3. Numbers for step 3: test-seconds = sum of nextest `[ x.xxxs]` of the native and interposed runs; wall from the
   `step:` lines. Before (TEST-PASS2-NATIVE, keke 4-core slot): interposed pass 1448 s nextest + 337 s export build,
   5015 test-seconds, 193 binaries. Fill the table below, write handoff/reports/PASS2-NATIVE-RUNNER.json.
4. `cargo clippy --workspace --all-targets --all-features -- -D warnings` and `cargo xtask static` in the role,
   then `cargo xtask precheck --jobs <labels> --head <sha>`, push, `ops handback-check`.

## Numbers (step 3): to fill

| Run | Wall | Test-seconds | Build |
|---|---|---|---|
| Before: interposed pass (TEST-PASS2-NATIVE) | 1448 s | 5015 | 337 s export build |
| After: native pass + interposed suite (`test-pass2`) | ? | ? | prefixed loader + copies (in the wall) |

## RULING NEEDED

* The CI host (mei) story gate the brief asks for: needs a role on mei, which this run does not have (CI roles are
  off-limits). Recommend: the dispatcher's CI run of this branch is that gate.
* Coordinator note 01:44 (one prefixed loader build for hot_text_products and the runner): not possible as is: the
  prefix is compiled in (`option_env!("NIJI_PREFIX")`); the coexist test needs `/usr/lib/niji` (read-only, not ours
  to populate with `lib/`), the native pass needs a writable per-tree root. Sharing one build needs a loader that
  takes its prefix at run time (a loader trust-boundary change). Recommend: keep two builds (the native one is
  incremental after the first gate in a tree).

## Not to redo

* `cargo xtask nextest` with `CARGO_TARGET_..._RUNNER` set (cargo runs xtask through it). Starting jobs right after
  a sync without checking rc (rc 4 = nothing synced).
