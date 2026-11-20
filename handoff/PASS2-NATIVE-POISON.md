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
* `pass2-native-poison-measure1` (`cargo xtask so && cargo xtask test-pass2`): rc 0; native 4831 passed, interposed
  suite 126 + 46 passed (+ doc). Ran beside `plants1`.
* `pass2-native-poison-plants1` (5 plants, `--timeout 900`, /dev/shm/claude/pass2-native-poison/plants-1): rc 0,
  5 CAUGHT, 0 SURVIVED, 0 WARN-RC-ONLY, 0 WARN-NO-MSG (WARN-DEV-SHM printed: no plant here uses posixipc_products).
  `native-runner-no-perturb` and `strdup-no-terminator` caught as `FAIL  test -p niji-boundary (native)`.
* `pass2-native-poison-gate1` (`cargo xtask affected --base origin/next --run`, `~/bin/gate 16G`): rc 0, tier direct,
  `FULL: the train's` (xtask files), so the bounded set ran: 15 suites ok, 7375 passed, 0 failed; peak 11G, oom_kill 0.
* `pass2-native-poison-measure2` (same as measure1, nothing of this story beside it): rc 0, all passed.

## Exact next step

Done when the hand-back verifies. A follow-up could look at the 10 targets still interposed (the table below):
the foreign-thread ones stay interposed (brief step 2), and so do the ones that check `feature = "export"`, call
`reallocarray` by symbol, or need the `VIRGIN` bit.

## Numbers (step 4): the second pass, before and after (keke, 4-CPU slot, role n1, test profile)

| Run | Native pass (wall) | Interposed suite (wall: niji-boundary + niji + doc) | Total wall (tests-* steps) | Test-seconds (native + suite) |
|---|---|---|---|---|
| Before `measure5` (PASS2-NATIVE-RUNNER) | 657.6 + 4.6 s | 462.1 + 76.3 + 2.0 s | 1203 s | 4126 (2155 + 1971) |
| Before `measure4` (PASS2-NATIVE-RUNNER) | 796.1 + 4.6 s | 457.5 + 66.2 + 2.1 s | 1326 s | 4542 (2630 + 1913) |
| After `measure2` | 681.1 + 4.4 s | 117.5 + 0.8 + 2.2 s | 806 s (-33% / -39%) | 2471 (2315 + 156) (-40% / -46%) |
| After `measure1` | 857.4 + 416.8 s (includes the test-binary build) | 206.3 + 3.1 + 1.9 s (includes the suite's `export` build) | 1486 s; 983 s without the two builds | 3010 (2849 + 161) |

Test-seconds = the sum of nextest's per-test `[ x.xxxs]` in each pass. The interposed suite drops from 525-540 s to
121 s of wall and from ~1900-2000 to ~160 test-seconds; the native pass now runs 4831 tests (was 4829), poisoned.
`measure1`'s set-up/copies steps (95.0 + 320.4 s) and part of its suite step built test binaries changed by this
branch (the base's samples had them cached), and it ran beside `plants1`, as the base's two samples ran beside
their plants; `measure2` ran with no other job of this story. Compared like for like (builds excluded): 983 s and
806 s against 1203 s and 1326 s, so the saving is 18 to 39% of wall and 27 to 46% of test-seconds. Most of the
native-pass difference between the samples is load (the same 4831 tests, 2849 against 2315 test-seconds).

## RULING NEEDED

* Moving 92 targets out of the export build reduces that build's coverage (brief step 2 orders it): what is
  dropped is those targets running against the test-profile heap and exports in-binary; natively they run against
  the release products, poisoned. Recommend: land (the brief's premise); the saving is in Numbers.

## INFORMATIONAL

* `MALLOC_PERTURB_` reaches the native test's children too (C programs the tests run, under libniji or the host
  library): fills of fresh/freed blocks there as well; no test depends on its absence (measure1, measure2 and gate1 pass with it).

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
