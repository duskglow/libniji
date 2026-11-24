# TEST-REDUNDANCY-AUDIT: redundant tests, consolidation, and what each step saves

Story TEST-REDUNDANCY-AUDIT (planning; owner request 11-21). An audit only: no test was removed, merged, skipped
or weakened. Every number below names its source (T1-T6). "test-s" = the sum of per-test seconds in a pass;
"wall" estimates divide test-s by the pass's measured throughput and are marked *est.*

## Headline

| | high-confidence items (H1-H3) | all items (H1-H3, M1-M7) |
|---|---|---|
| test binaries removed (of 321 `crates/*/tests/*.rs`) | 0 | 70 (M7: the products targets as one) |
| lines of test code removed | ~20 | ~1,100 *est.* (M5 ~1,000, M2 ~100) |
| pass 1 test execution | -140 test-s, ~-9 s wall *est.* | -1,450 test-s, ~-91 s wall *est.* |
| pass 2 test execution | -965 test-s, ~-91 s wall *est.* | -1,214 test-s, ~-115 s wall *est.* |
| compile + link of test targets | 0 | -240 unit-s cold (T3); ~-7 s of the CI gate's ~34 s test build *est.* |
| **per full run = per train** (one full CI gate per train) | **~-100 s of a 1,505 s gate run (6.6%)** | **~-215 s (14%)** |

The current full gate (T1, T2): the CI gate job is 1,413-2,055 s wall (median 1,838 s over the five gates of T1;
it includes queueing), and its run 1,505 s (aacf0bce8: `--- summary (1505s)`). Of that run: products build 250 s,
pass 1 515 s (test build and listing ~34 s median, nextest 460 s median), pass 2 ~350 s (loader setup 55 s,
binary copies 7 s, nextest 268 s median), test-sets 108 s, interposed suite 55 s, rust-dylib 76 s,
native-examples 62 s, conformance 31 s, clippy 28 s.

What the numbers say: the suite's cost is concentrated (pass 1: the top 50 tests hold 41% of test-s, the
top 200 hold 71%; 5,100 tests under 0.1 s hold 150 s in all). The one change that moves the most seconds is
not a redundancy but a harness effect (M4: xtask's `affected::tests` rebuild the same repository index in
each of ~70 processes, 1,160 test-s). The largest redundancy is pass 2 re-running tests whose subject never
sees pass 2 (H1, M1: ~1,200 test-s). Test binaries cost little gate time: the CI build of all 326 test
targets is ~34 s, so merging binaries is a maintenance and CPU gain, not a wall-time one (M7).

## Timing sources

- **T1** Five CI full gates on the CI host (Zen 1, 48 threads), 2026-11-23 00:24-04:10 UTC (heads ef44c6896,
  1c70414bd, b04f71229, 3db4c4009, aacf0bce8): every nextest `PASS [ s]` line of `~/logs/ci-gate-*.affected.log`,
  per test the median of the five. Each is a full `cargo xtask test`, both passes; a sixth full run on a
  developer role (`printf-no-heap-2-xtask-test.log`, same host) agrees and was not used for the medians.
  (Deviation from the brief, which asked for one full `cargo xtask test` on a role: five real train runs
  measure the same thing with a spread, and a sixth run would have taken ~25 minutes of a shared host.)
- **T2** Step times (`step:` lines) and nextest `Summary` lines of the same logs; CI gate job walls from the
  host's `jobs.tsv`.
- **T3** Job `test-redundancy-audit-timings`: `cargo test --workspace --no-run --timings` in a fresh target
  directory on the Zen 1 build host (48 threads, load ~16), 06:46-06:53 UTC: 440 s wall, 1,495 CPU-s; units
  from cargo's timing report.
- **T4** Job `test-redundancy-audit-affected-once`: xtask's unit-test binary from T3 run as one process,
  `affected::tests --test-threads 16`.
- **T5** Failure history: every `FAIL`/signal line of the 608 CI gates since 10-21 that reached pass 2, and of
  90 developer gate logs on the same host; a pass-2-only failure = failed in pass 2, not in pass 1 of the same run.
- **T6** Code reads cited per item (file:line on base e3b0caf60).

## Where the time goes (T1)

| pass | tests | test-s | wall (nextest) | in flight |
|---|---|---|---|---|
| 1 (host process) | 7,177 | 7,306 | 460 s | 15.9 |
| 2 (as libniji processes) | 4,878 | 2,839 | 268 s | 10.6 |
| interposed suite (niji-boundary) | 129 | 68 | 46 s | |

Share of each pass by crate: pass 1: niji 32%, xtask 27%, niji-boundary 19%, niji-math 8%, niji-ld 6%,
niji-locale 5%, the rest 3%. Pass 2: niji 52%, niji-boundary 43%, niji-math 3%, niji-rt 2%.

Top tests by median seconds (pass 1 / pass 2; "-" = not in that pass):

| test | p1 | p2 |
|---|---|---|
| niji::iconv_codespace_products builtin_codecs_equal_the_reference_over_their_code_space | 322 | 204 |
| xtask headers::slices::s12 network_headers_pass_header_check | 151 | - |
| niji::iconv_charmaps_products generated_charmaps_convert_like_the_reference | 119 | 81 |
| niji::hot_text_products init_prefixed_interpreter_trace_reads_no_host_file_and_matches_the_products_trace | 114 | (in 76 total) |
| niji-locale::reference generated_charmaps_compile_like_the_reference | 114 | - |
| niji::dns_products resolv_equals_the_reference | 111 | 100 |
| niji::regex_subpatterns back_references_over_unary_strings_finish | 106 | - |
| xtask affected::tests (104 tests, 71 over 5 s) | 1,403 total | - |
| niji-boundary::casecmp_avx2 casecmp_equals_the_path_of_before_at_every_alignment_pair_length_and_difference | 88 | 99 |
| niji-boundary::heap_threads statistics_never_read_below_the_blocks_live_throughout | 72 | 66 (+63 interposed) |
| niji-rt::core_contracts mutex_stress_mixed_entry_points_lose_no_update | 69 | 46 |
| niji::aio_products (6 tests) | 286 total | 278 total |
| niji-boundary::strstr_find (22 tests) | 208 total | 244 total |
| niji-ld dlopen_new_objects / dlopen_process_objects / start_objects out_of_memory_is_clean | 59 / 46 / 28 | - |

The full ranked list (12,230 rows) is reproducible from T1 with the method above. Two parts of the premise
have moved: the long-double special-function shards now cost little in the gate (special_ld_products: 9
tests, 3 s; the full sweeps run outside the gate under the verdict cache, D-10.57), and
dlopen_process_objects_out_of_memory_is_clean is 46 s median (it stops once two runs in a row complete).

## Ranked candidates

Confidence: **high** = the covering evidence is structural (the code path cannot differ) and the history
agrees; **medium** = sound argument, but the saving or the sufficiency is estimated; items without evidence
of a covering test are listed under "Investigated, not candidates" and count toward no saving.

| # | candidate | saves (T1, T3, T4) | binaries / lines | conf. | covering test / evidence | risk (what would go uncaught) |
|---|---|---|---|---|---|---|
| H1 | pass 2 of the `*_products` binaries, except hot_text, heap_meta, timers | -965 test-s pass 2 (58 binaries) | 0 / ~5 | high | the same tests in pass 1; see below | a libniji spawn/pipe defect that only one product harness's child handling would meet; still met by the 3 kept products binaries and every pass-2 test that starts a child |
| H2 | aio_products `uring_on`, `eperm_on` (cache on/off where the program makes no lookup) | -95 test-s pass 1 (-93 pass 2, inside H1) | 0 / ~12 | high | `uring_off`, `eperm_off`: same program, same comparison | none found: the "on" run differs from "off" only in a flag no code path of aio_diff.c reads |
| H3 | the second (cache on/off) libniji run inside catalogs, syslog, fstree, posixipc, mntent, mallocdbg products | ~-45 test-s pass 1 *est.* (1/3 of 134 s) | 0 / ~6 | high (saving est.) | the other mode's run in the same test | as H2 |
| M4 | `affected::tests`: one repository index per process (nextest) instead of one per run | -1,160 test-s pass 1 (T4: 1,403 test-s vs ~243 CPU-s in one process) | 0 / 0 | medium (how to share is a design choice) | n/a (no test removed) | a shared index must never be stale (SLOW AND WORKING WINS) |
| M1 | pass 2 of the later length/build shards of strstr_find (m_16..m_70, 3 families) and small_compare (build_1..4) | -249 test-s pass 2 | 0 / ~10 | medium | shard 0 of each family in pass 2; every shard in pass 1 | a pass-dependent path (thread block, heap, locale through the thread block) reached only at some lengths: none such in these kernels |
| M2 | aio_products: one reference run and one pair of builds for all variants | ~-80 test-s pass 1 *est.* | 0 / ~100 | medium | n/a (consolidation) | longer single test (~125 s *est.*), still below iconv_codespace's 322 s |
| M6 | dlopen_new_objects / start_objects OOM sweeps: dlopen_process_objects' stop rule | up to ~-70 test-s pass 1 *est.* | 0 / ~4 | medium | the limits below the stop | a failure only above two completed limits: the rule's own argument (a larger limit only adds room) |
| M5 | one shared differential harness for the products tests | 0 s | 0 / ~1,000 *est.* | medium | n/a | mechanical; a helper bug hits many tests at once (plants on the helper) |
| M7 | the 71 `*_products` targets as one test binary | -240 unit-s cold compile+link (T3: 274 of 1,092); ~-7 s CI *est.*; pass-2 copies -1.5 s | -70 / ~-300 | medium-low | n/a | the direct tier selects by target (needs selection by test name); the `cargo test` fallback runs tests as threads of one process |

### Evidence per candidate

**H1: pass 2 of the products binaries.** Pass 2 runs the same test binaries with their `PT_INTERP` rewritten
to libniji's loader (`xtask/src/pass2.rs:1-24`): only the harness process changes. A products test's subject
is a child: a C program linked statically against `target/niji/libc.a` and the same program against the host
library, each run and compared line by line (`crates/core/tests/aio_products.rs:1-18`, the `build` helpers
below). Pass 2 sets no variable the children read (`NIJI_TEST_PASS` is read only by
`crates/boundary/tests/pass2_native_process.rs`), and 55 of the 71 products files clear the environment of
their children (`env_clear`, e.g. `aio_products.rs:206`). So the compared outputs are the same programs' in
both passes, and pass 2 adds only the harness's own spawn and pipe reads on libniji, which every pass-2 test
that starts a child also does. History (T5): over 608 CI gates and 90 developer gates, 39 failures were
pass-2-only, in 11 binaries; none in the 58 binaries H1 drops. hot_text_products (12), heap_meta_products (4) and
timers_products (1) failed in pass 2 only at least once, so H1 keeps them in pass 2. Precedent: D-X.36 (the
math products sweeps run shard 0 only in pass 2: "a suite runs in pass 2 only to the extent that it exercises
the interposed functions"). Saving: 965 test-s of pass 2's 2,839 (pass 2 median 268 s wall, so ~-91 s
*est.*); iconv_codespace_products alone is 204 s of pass 2. **RULING NEEDED** (it reduces gate coverage).

**H2, H3: the name-service cache on and off where the program makes no lookup.** D-16.10's test clause:
"every differential test of phases 10-11 passes with the cache enabled and disabled". At process start the
cache only stores its switch (`crates/boundary/src/nscache.rs:52-55` calls
`crates/legacy/src/nscache.rs:402-406`: three stores); it attaches to its shared region at the first lookup,
from netdb, users, resolv, patterns (glob/wordexp tilde), setxid, s18_processes and fork only (the callers of
`NSCACHE`). aio_diff.c, catalogs_diff.c, syslog_diff.c, fstree_diff.c, posixipc_diff.c, mntent_diff.c and
mallocdbg_diff.c call none of getpw*/getgr*/gethost*/getaddrinfo/getnameinfo/getserv*/getproto*/getnet*/
getrpc*/res_*/ether_*/innetgr/getspnam/initgroups/getgrouplist/set*id/setgroups/glob/wordexp/grantpt/getlogin
(grep of each program). So their "on" and "off" runs execute the same code apart from one flag no path reads.
aio splits the modes into separate tests: `aio_equals_the_reference_uring_on` and `_uring_off`
(`aio_products.rs:129-139`) run `check("uring-on", "io_uring", None, "on")` and the same with `"off"`; the
eperm pair the same; 47.6 s each (T1). The other six run both modes inside one test
(`for mode in ["on", "off"]`); their saving is estimated as one third of the test (a reference run and two
libniji runs). Kept as they are: process and terminals (grantpt), patterns (tilde expansion), netdb, users,
dns (lookups). **RULING NEEDED**: amend D-16.10's test clause to "every differential test whose program
reaches the cache" (recommend: yes, with the reach checked by a static test listing the cache's callers, so
a program that starts making lookups goes back to both modes).

**M4: `affected::tests`' repository index.** Every selection test calls `repo()`
(`xtask/src/affected.rs:5101-5104`), a `OnceLock` around `Repo::load` of the whole tree. Under nextest each
test is its own process, so ~70 tests each load the index (T4: one test alone 19.7 s; all 104 in one
process 60.5 s wall, ~243 CPU-s; under nextest 1,403 test-s). The per-fix gate's `cargo test -p xtask` already
runs them in one process; the saving is in the full gate's pass 1 (~-73 s wall *est.*). Options for the
follow-up: profile `Repo::load` (19.7 s for one tree is itself the cost every `cargo xtask affected` pays),
make the selection cases one table-driven test over one load, or run xtask's unit tests outside nextest in
the full gate. A cache on disk must be keyed so it can never serve a stale index.

**M1: pass 2 of the later kernel shards.** strstr_find's needle-length shards
(`strstr_matches_a_naive_search_at_every_length_position_and_alignment::m_0_to_15` ... `m_48_to_70`, and the
two strcasestr families) and small_compare's `..._in_each_build_0..4` run the same niji-boundary code in both
passes (the same binary); the test sources call no C function (no `extern "C"`, no `libc::`). What pass 2
changes for niji-boundary code is the thread block, the heap and the locale reached through the thread block
(the reason `PASS0_BANNED` refuses `niji_boundary`, `xtask/src/nextest.rs:34-37`); needle length and build
shard do not vary those, so shard 0 of each family exercises them. T5: no pass-2-only failure in either
binary. Same argument as D-X.36. casecmp_avx2 is one unsharded test and locale-dependent: not included.
**RULING NEEDED** (gate coverage).

**M2: aio_products' repeated reference.** Each of the six tests builds aio_diff.c twice and the seccomp
wrapper once and runs the reference (`aio_products.rs:180-184,210`); the reference output is the same for
every variant (no variant changes the reference run). One test running the reference once and each libniji
configuration in turn keeps every comparison. The saving assumes the reference run is about half of a
variant (not measured: the follow-up measures the split before changing anything). The eperm, enosys and
allow variants all take the worker engine under D-11.11 and differ only if libniji tries io_uring under a
filter; they stay as configurations, not removals.

**M6: the loader OOM sweeps' tail.** dlopen_new_objects_out_of_memory_is_clean
(`crates/ld/tests/dlopen_new_objects.rs:266-270`, limits 1-48 MiB in 256 KiB steps, 58.5 s) and
start_objects_out_of_memory_is_clean (`crates/ld/tests/start_objects.rs:170-174`, the same range, 27.9 s) run
every limit; dlopen_process_objects (`dlopen_process_objects.rs:230`) stops "once two runs in a row complete
(a larger limit only adds room no growth path lacks)" and runs in 46 s over a range twice as wide. The saving
is the share of limits above the second completed run, not measured; the follow-up prints the first
completed limit and records the saving. **RULING NEEDED** (repeat count).

**M5: the products harness.** 71 `*_products.rs` files, 14,586 lines; 1,531 lines are top-level harness
functions. `build` (host vs libniji static link) is defined in 58 files in 18 variants (24, 11 and 8 copies of
the three commonest, 11-12 lines each); `root` in 63, the KNOWN/ADDED/DROPPED filter (`without`,
`with_headers`, the diff loop of `check`) in 11. A link-flag change (the crt object's name, a new library)
touches 58 files today. One `oracle::build_pair(src, libs)` and one `oracle::known_diff` in
`crates/core/tests/support/oracle.rs` would remove ~1,000 lines *est.*

**M7: one products binary.** T3: the 71 products targets cost 274 of the 1,092 unit-s that the 326
integration targets take to compile and link cold (median 2.4 s per target; each compiles its own copy of
`support/oracle.rs`, `skip_note.rs`, ...). In the train the test build is incremental, ~34 s for all
targets (T2: tests-nextest step minus nextest's Summary, median of five), so the wall gain is small
(~-7 s *est.*); the gains are CPU, the pass-2 copy step (7 s for 326 binaries) and one place for the harness.
Process isolation stays (nextest runs each test as its own process). Blocks: the affected tiers select by test
target ("selected: N of 321 test targets"), so a merged target needs selection by test name first; the
`cargo test` fallback (no nextest) would run the products tests as threads of one process.

### Investigated, not candidates (no covering evidence; no saving counted)

- **casecmp_avx2 full sweep** (88 s + 99 s): no plant record says what sweep size the AVX2 kernel needs; a size
  reduction needs mutation runs first (follow-up: plant the kernel's alignment and tail handling, shrink the
  sweep while every plant is caught). Locale-dependent, so it stays in pass 2.
- **network_headers_pass_header_check** (151 s, xtask): not redundant. It is the only place the whole-tree
  `spec/tools/header_check.py` runs, inside slice 12's unit test; a maintenance note (it belongs in a gate step
  of its own, run once, where its cost is visible).
- **niji-locale generated_charmaps_compile_like_the_reference (114 s) vs niji::iconv_charmaps_products (119 s)**:
  different subjects (localedef's compile vs iconv's conversion); not redundant.
- **heap_threads in three passes** (72 + 66 + 63 s): pass 1 and the interposed suite both run libniji's heap on a
  host thread; whether the non-`export` tests add anything in the interposed suite was not settled.
- **Generator self-checks**: the remaining generator tests are cheap (generator_prechecks_run_the_checks 13.5 s,
  headers_gen_regenerates_the_abi_layouts 4.8 s); GATE-TEST-TIME fixed the costly ones. Nothing found.
- **iconv_codespace_products**: not a candidate in pass 1 (the brief: only the whole code space caught both
  plants); its pass-2 copy is in H1.
- **Same-name tests across binaries**: none outside shards and per-format math families (T1 leaf-name scan).

## Consolidation and maintainability (T3, T6)

| crate | test binaries | `#[test]` in tests/ | lines in tests/ | compile+link unit-s (cold, T3) | pass 1 test-s | pass 2 test-s |
|---|---|---|---|---|---|---|
| niji (core) | 151 | 1,734 | 60,073 | 561 | 2,343 | 1,487 |
| niji-boundary | 88 | 3,274 | 59,553 | 297 | 1,381 | 1,218 |
| niji-ld | 35 | 259 | 15,449 | 51 | 445 | - |
| niji-math | 25 | 151 | 8,968 | 138 | 582 | 78 |
| niji-mathtools | 6 | 30 | 1,451 | 12 | 33 | - |
| niji-rt | 6 | 99 | 3,925 | 7 | 87 | 57 |
| niji-abi | 4 | 426 | 5,207 | 3 | 10 | - |
| niji-locale | 2 | 23 | 916 | 3 | 375 | - |
| niji-legacy | 2 | 8 | 150 | 1 | 3 | - |
| niji-charmaps | 1 | 2 | 103 | 3 | 44 | - |
| niji-kernel | 1 | 11 | 228 | 1 | 0 | - |
| xtask | 5 (+ the bin's 603 unit tests) | 23 | 872 | 18 (+309 for the bin's unit tests) | 2,002 | - |
| **total** | **326** | **6,040** | **156,895** | **1,092** (+490 lib/bin unit-test targets) | 7,306 | 2,839 |

(Pass columns include each crate's lib unit tests; T1.)

- **Harness overhead per binary.** Cold compile+link of an integration target: median 2.4 s (T3). In the train:
  ~34 s for the whole incremental test build (T2), plus 7 s of pass-2 binary copies. Per-test process start
  is small in both passes: the 5,100 pass-1 tests under 0.1 s take 150 s together, the 3,596 such pass-2 tests 110
  s (~0.03 s each including the test), and nextest starts a process per test whatever the binary count, so merging
  binaries does not reduce process starts.
- **Critical path of a cold build**: xtask's own unit-test binary, 309 s of the 440 s (T3); every xtask change
  pays it. Out of scope here; noted for the tooling owners.
- **Duplicated helpers across test files** (all crates): 117 helpers have byte-identical bodies in more than one file
  (1,901 extra lines; `root` 81 copies, `ensure_locale` 17, `conformance` 15, `child_report` 13, `target` 25);
  same-name helpers with differing bodies: `root` 97 files, `build` 76, `run` 47, `check` 29. The products harness
  (M5) is the one large family; the rest are 3-10 line helpers where a shared module saves little.
- **Families of near-identical tests that should be one table-driven test**: aio_products' six variants (M2);
  `affected::tests`' selection cases (M4, if the follow-up chooses that form).
- **The generated per-section binaries** (`sNN_*_clauses`, `sNN_*_symbols`, ~60 targets) come from
  `cargo xtask stubs`; merging them is a generator change and was not costed.

## Follow-up stories (each removable or mergeable set alone, so each can be approved and reverted alone)

| story | items | needs | expected saving per full run |
|---|---|---|---|
| PASS2-PRODUCTS-ONCE | H1 | RULING (gate coverage) | -965 test-s pass 2, ~-91 s *est.* |
| NSCACHE-MODE-SCOPE | H2, H3 | RULING (amend D-16.10's test clause) | -140 test-s pass 1 (+ pass 2 if H1 is refused) |
| AFFECTED-TESTS-ONE-LOAD | M4 | design choice; profile `Repo::load` first | -1,160 test-s pass 1, ~-73 s *est.* |
| PASS2-KERNEL-SHARD0 | M1 | RULING (gate coverage) | -249 test-s pass 2, ~-24 s *est.* |
| AIO-ONE-REFERENCE | M2 | measure the reference/libniji split first | ~-80 test-s pass 1 *est.* |
| LD-OOM-SWEEP-STOP | M6 | RULING (repeat count) | up to ~-70 test-s pass 1 *est.* |
| PRODUCTS-HARNESS-SHARED | M5 | plants on the shared helpers | ~-1,000 lines *est.* |
| PRODUCTS-ONE-BINARY | M7 | selection by test name in the affected tiers first | -70 binaries, -240 unit-s cold |

Order: H1 and NSCACHE-MODE-SCOPE first (high confidence, rulings only); AFFECTED-TESTS-ONE-LOAD next (largest single
saving, no coverage change); M5 before M7 (a shared harness makes the merge mechanical). Every story measures its
own before/after from the next CI gates' T1 lines, since the wall estimates here assume each pass stays
throughput-bound.

## Informational (found on the way)

- Pass-2-only failures exist (T5), with no verdict here on whether each was a catch or a flake: CI gates:
  hot_text_products 12 (`init_coreutils_shaped_program_stays_within_its_page_fault_bound`), search_nomem 8,
  heap_meta_products 4, time_gaps_probe 2, five binaries 1 each; developer gates: s03_memory_symbols 7,
  s18_processes_clauses 1. Repeated pass-2-only failures of one test are worth an owner's look (no-flakes rule).
- The comment on `may_hold_boundary_code` (`xtask/src/nextest.rs:186-191`) still describes the exports-interposed
  pass ("changes only niji-boundary's features"); pass 2 is now the native runner (pass2.rs), under which every
  binary, with or without niji-boundary code, runs its harness on libniji. The run-once rule's premise deserves a
  re-check by the pass-2 owners (a binary without niji-boundary code still exercises libniji's start-up, heap and
  threads in pass 2).
