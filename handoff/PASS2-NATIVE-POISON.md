# PASS2-NATIVE-POISON: poison C-family blocks in the native pass; most of the interposed suite moves to it

Branch `task/pass2-native-poison` (stacked on `task/pass2-native-runner`, whose head 978bbed is its base), host keke
(Zen 1, 60 vCPU), role n1. Decision record D-X.137 (reserved). Plants: `handoff/PASS2-NATIVE-POISON-plants.txt`.

## What changed

* Step 1, `xtask/src/pass2.rs`: the runner (`pass2-exec`) sets `MALLOC_PERTURB_` from `NIJI_TEST_MALLOC_PERTURB`
  (`products_perturb`) on the test copy it executes; the products' heap reads it at start (§3.2), so a fresh
  C-family block holds the complement of the byte (0x5a) and a freed one the byte, as interposed. Children inherit
  it (libniji's and the host's programs); cargo and rustc never get it (the runner is not in their path). Unit test
  `pass2::tests::the_poison_byte_reaches_the_products_heap_as_malloc_perturb`.
* Proof test: `niji-boundary::pass2_native_process::every_pass_libniji_serves_the_c_family_in_poisons_its_fresh_blocks`
  (natively and in the export build: `selftest::c_family_fresh_fill(n)` == `!byte` for n = 1, 24, 200, 5000; first
  pass: nothing to check). New selftest helper `c_family_fresh_fill` (`crates/boundary/src/selftest.rs`).
  `xtask/src/longsweeps.rs` NOT_SWEEPS row for it (it parses the poison byte, not a size).
* Step 2, `xtask/src/nextest.rs`: `NATIVE_ONLY` replaced by the explicit `INTERPOSED` list (9 targets) plus
  niji-boundary's unit tests; a new target runs natively only. The unit test now scans every test source of both
  packages: one that contains `feature = "export"`, `::reallocarray(` or `::mcheck(` must be listed. Comments in
  `affected.rs` and `nextest::POISON`; `docs/architecture.md` row.
* Step 3, `crates/boundary/tests/s03_memory_symbols.rs` `sym_mallinfo` (finding #6881): compares `mallinfo` with
  `mallinfo2` inside a window where two `mallinfo2` reads around it agree (up to 1000 tries, no clock).

## Jobs (keke, role n1; head with code = 3 WIP commits after the base; later commits are prose only)

* `pass2-native-poison-check3`: `cargo xtask static` ok, xtask `pass2::`/`nextest::`/`longsweeps::` 29 passed.
  (`check2`: clippy ok, static red on long-sweeps, fixed; `check1`: clippy red on `panic!`, fixed.)
* Running when this leg ended (started ~04:50 UTC by `~/work/pass2-native-poison/seq.sh`, one after another):
  `pass2-native-poison-measure1` (`cargo xtask so && cargo xtask test-pass2`), then `-gate1`
  (`cargo xtask affected --base origin/next --run` under `~/bin/gate 16G`), then `-measure2` (same as measure1).
  Beside them: `pass2-native-poison-plants1` (5 plants, `--timeout 900`, dir /dev/shm/claude/pass2-native-poison/plants-1;
  WARN-DEV-SHM printed, harmless here).

## Exact next step

1. `~/bin/jobwait pass2-native-poison-measure1 pass2-native-poison-plants1` (loop while rc 3). measure1 failing =
   a moved target fails natively with poisoning: read the failing test, fix the cause (do not move it back without
   a reason in this note). Plants: every plant CAUGHT (`strdup-no-terminator` and `native-runner-no-perturb` must
   show `FAIL  test -p niji-boundary (native)`); a SURVIVED: fix and rerun as plants2 in a NEW dir.
2. Numbers (step 4): from each measure log, the step times (`tests-native-*`, `tests-interposed-*`) and
   test-seconds (sum of nextest's `[ x.xxxs]`; `~/bin/test-summary <log>` may help). Before = PASS2-NATIVE-RUNNER's
   `measure4`/`measure5` (same host, slot size and role; their table is in handoff/PASS2-NATIVE-RUNNER.md: total
   1326 s / 1203 s wall, 4542 / 4126 test-seconds; interposed suite 525.8 / 540.4 s). Fill the Numbers section here.
3. jobwait `-gate1`, `-measure2`. If the gate says `FULL (escalated` it ran the bounded set: write
   `FULL: the train's (xtask files)` in the hand-back.
4. Commit this note + `handoff/reports/PASS2-NATIVE-POISON.json`, push, sync, `cargo clippy --workspace --all-targets
   --all-features -- -D warnings` and `cargo xtask static` on the role (foreground), then `cargo xtask precheck --jobs
   <labels> --head <sha>`, then `ops handback-check`. JOBS: check3, measure1, gate1, measure2, plants1.

## Numbers (step 4)

(to fill from measure1/measure2)

## RULING NEEDED

* Moving 92 targets out of the export build reduces that build's coverage (brief step 2 orders it): what is
  dropped is those targets running against the test-profile heap and exports in-binary; natively they run against
  the release products, poisoned. Recommend: land (the brief's premise); the saving is in Numbers.

## INFORMATIONAL

* `MALLOC_PERTURB_` reaches the native test's children too (C programs the tests run, under libniji or the host
  library): fills of fresh/freed blocks there as well; no test is known to depend on its absence (measure1 decides).

## Not to redo

* A `panic!` in a test body (clippy `-D clippy::panic` in tests): use `assert!`. A test that parses a number from
  the environment needs a `longsweeps` row.

## The interposed suite, target by target

| Target | Before (PASS2-NATIVE-RUNNER) | Now | Why it stays interposed |
|---|---|---|---|
| `niji-boundary (unit tests)` | native + interposed | native + interposed | the library's own `cfg(feature = "export")` code |
| `niji-boundary::appendix_a_primitives` | native + interposed | native only |  |
| `niji-boundary::casecmp_avx2` | native + interposed | native only |  |
| `niji-boundary::classed_slots` | native + interposed | native only |  |
| `niji-boundary::csys_link` | native + interposed | native only |  |
| `niji-boundary::csys_veneer_abi` | native only | native only |  |
| `niji-boundary::dl_tls_threads` | native only | native only |  |
| `niji-boundary::erf_quick` | native + interposed | native only |  |
| `niji-boundary::exp_quick` | native + interposed | native only |  |
| `niji-boundary::export_macro` | native + interposed | native only |  |
| `niji-boundary::failinject` | native + interposed | native only |  |
| `niji-boundary::failinject_products` | native only | native only |  |
| `niji-boundary::find_seq_scan` | native + interposed | native only |  |
| `niji-boundary::heap_foreign_pair_cost` | native + interposed | native + interposed | foreign path (the foreign malloc/free pair) |
| `niji-boundary::heap_threads` | native + interposed | native + interposed | checks `feature = "export"`; foreign threads |
| `niji-boundary::hyperb_quick` | native + interposed | native only |  |
| `niji-boundary::invhyp_quick` | native + interposed | native only |  |
| `niji-boundary::libm_fma_select` | native only | native only |  |
| `niji-boundary::locale_ctype_l` | native + interposed | native only |  |
| `niji-boundary::log_quick` | native + interposed | native only |  |
| `niji-boundary::machine_scanners` | native + interposed | native only |  |
| `niji-boundary::math_fma_paths` | native + interposed | native only |  |
| `niji-boundary::math_kernel_no_invalid` | native + interposed | native only |  |
| `niji-boundary::modern_calls` | native + interposed | native only |  |
| `niji-boundary::native_v1` | native + interposed | native + interposed | calls `reallocarray` (by symbol only with `export`) |
| `niji-boundary::niji_cc_lm` | native only | native only |  |
| `niji-boundary::nscache` | native + interposed | native only |  |
| `niji-boundary::pass2_native_process` | native + interposed | native + interposed | checks each pass's own process (and the poisoning, interposed too) |
| `niji-boundary::placement_sweep` | native only | native only |  |
| `niji-boundary::pow_quick` | native + interposed | native only |  |
| `niji-boundary::qsort_alloc_fail` | native + interposed | native only |  |
| `niji-boundary::regex_dfa` | native + interposed | native only |  |
| `niji-boundary::regex_dfa_signal_fork` | native + interposed | native only |  |
| `niji-boundary::regex_enomem` | native + interposed | native only |  |
| `niji-boundary::regex_paths` | native + interposed | native only |  |
| `niji-boundary::regex_reentry` | native + interposed | native only |  |
| `niji-boundary::reload` | native + interposed | native only |  |
| `niji-boundary::rseq_node` | native + interposed | native only |  |
| `niji-boundary::s00_conventions_clauses` | native + interposed | native only |  |
| `niji-boundary::s01_startup_symbols` | native + interposed | native only |  |
| `niji-boundary::s02_diagnostics_symbols` | native + interposed | native only |  |
| `niji-boundary::s03_memory_symbols` | native + interposed | native + interposed | `reallocarray` by symbol; `mcheck` (`VIRGIN`) |
| `niji-boundary::s04_strings_symbols` | native + interposed | native only |  |
| `niji-boundary::s05_ctype_symbols` | native + interposed | native only |  |
| `niji-boundary::s06_multibyte_symbols` | native + interposed | native only |  |
| `niji-boundary::s07_locale_symbols` | native + interposed | native only |  |
| `niji-boundary::s08_numeric_symbols` | native + interposed | native only |  |
| `niji-boundary::s09_random_symbols` | native + interposed | native only |  |
| `niji-boundary::s10_math_symbols` | native + interposed | native only |  |
| `niji-boundary::s11_lowio_symbols` | native + interposed | native only |  |
| `niji-boundary::s12_stdio_symbols` | native + interposed | native only |  |
| `niji-boundary::s13_filesystem_symbols` | native + interposed | native only |  |
| `niji-boundary::s14_search_symbols` | native + interposed | native only |  |
| `niji-boundary::s15_sockets_symbols` | native + interposed | native only |  |
| `niji-boundary::s16_nameservices_symbols` | native + interposed | native only |  |
| `niji-boundary::s17_users_symbols` | native + interposed | native only |  |
| `niji-boundary::s18_processes_symbols` | native + interposed | native only |  |
| `niji-boundary::s19_terminals_symbols` | native + interposed | native only |  |
| `niji-boundary::s20_signals_symbols` | native + interposed | native only |  |
| `niji-boundary::s21_contexts_symbols` | native only | native only |  |
| `niji-boundary::s22_threads_symbols` | native + interposed | native only |  |
| `niji-boundary::s23_time_symbols` | native + interposed | native only |  |
| `niji-boundary::s24_system_symbols` | native + interposed | native only |  |
| `niji-boundary::s25_ipc_symbols` | native + interposed | native only |  |
| `niji-boundary::s26_syslog_symbols` | native + interposed | native only |  |
| `niji-boundary::s27_dynlink_symbols` | native only | native only |  |
| `niji-boundary::s28_profiling_symbols` | native + interposed | native only |  |
| `niji-boundary::s29_legacy_symbols` | native + interposed | native only |  |
| `niji-boundary::s29_rpc_auth_stubs` | native + interposed | native only |  |
| `niji-boundary::s29_rpc_clnt_svc_stubs` | native + interposed | native + interposed | checks `feature = "export"` |
| `niji-boundary::scanner_races` | native + interposed | native only |  |
| `niji-boundary::search_nomem` | native + interposed | native only |  |
| `niji-boundary::sem_named_nomem` | native + interposed | native only |  |
| `niji-boundary::setlocale_env_repeat` | native only | native only |  |
| `niji-boundary::setlocale_posix_repeat` | native only | native only |  |
| `niji-boundary::small_compare` | native + interposed | native only |  |
| `niji-boundary::small_copy` | native + interposed | native only |  |
| `niji-boundary::strstr_find` | native + interposed | native only |  |
| `niji-boundary::strto_internal_group` | native + interposed | native only |  |
| `niji-boundary::tb_foreign_low_words` | native + interposed | native + interposed | foreign path (thread blocks of foreign threads) |
| `niji-boundary::tb_foreign_words` | native + interposed | native + interposed | foreign path (thread blocks of foreign threads) |
| `niji-boundary::towctrans_desc` | native + interposed | native only |  |
| `niji-boundary::trig_quick` | native + interposed | native only |  |
| `niji-boundary::uapi_vs_abi` | native only | native only |  |
| `niji-boundary::wcsstr_find` | native + interposed | native only |  |
| `niji-boundary::wordexp_block` | native + interposed | native only |  |
| `niji-boundary::wordexp_reuse` | native + interposed | native only |  |
| `niji::s01_startup_clauses` | native + interposed | native only |  |
| `niji::s02_diagnostics_clauses` | native + interposed | native only |  |
| `niji::s03_memory_clauses` | native + interposed | native + interposed | `reallocarray` by symbol; `mcheck` (`VIRGIN`) |
| `niji::s04_strings_clauses` | native + interposed | native only |  |
| `niji::s05_ctype_clauses` | native + interposed | native only |  |
| `niji::s06_multibyte_clauses` | native + interposed | native only |  |
| `niji::s07_locale_clauses` | native + interposed | native only |  |
| `niji::s08_numeric_clauses` | native + interposed | native only |  |
| `niji::s09_random_clauses` | native + interposed | native only |  |
| `niji::s11_lowio_clauses` | native + interposed | native only |  |
| `niji::s12_stdio_clauses` | native + interposed | native only |  |
| `niji::s13_filesystem_clauses` | native + interposed | native only |  |
| `niji::s14_search_clauses` | native + interposed | native only |  |
| `niji::s15_sockets_clauses` | native + interposed | native only |  |
| `niji::s16_nameservices_clauses` | native + interposed | native only |  |
| `niji::s17_users_clauses` | native + interposed | native only |  |
| `niji::s18_processes_clauses` | native + interposed | native only |  |
| `niji::s19_terminals_clauses` | native + interposed | native only |  |
| `niji::s20_signals_clauses` | native + interposed | native only |  |
| `niji::s21_contexts_clauses` | native only | native only |  |
| `niji::s22_threads_clauses` | native + interposed | native only |  |
| `niji::s23_time_clauses` | native + interposed | native only |  |
| `niji::s24_system_clauses` | native + interposed | native only |  |
| `niji::s25_ipc_clauses` | native + interposed | native only |  |
| `niji::s26_syslog_clauses` | native + interposed | native only |  |
| `niji::s27_dynlink_clauses` | native only | native only |  |
| `niji::s28_profiling_clauses` | native + interposed | native only |  |
| `niji::s29_legacy_clauses` | native + interposed | native only |  |
| `niji::s31_utilities_clauses` | native only | native only |  |

Moved to native only by this story: 92; interposed now: 10 (with the unit tests).
