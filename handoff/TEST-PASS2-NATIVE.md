# TEST-PASS2-NATIVE: the second test pass as libniji processes

Branch `task/test-pass2-native` (from `origin/next` 65d9d2cfd). Host keke (Zen 1, 60 vCPU), role r5, every job in a
4-core slot.

## Result in one paragraph

All 317 test binaries of the ordinary (pass-1) build, run as libniji processes (libniji's `ld.so` as the program
interpreter, libniji's `libc.so.6`, `libm.so.6` and placeholder libraries, the toolchain's `libgcc_s.so.1`) ran
6759 tests: **6755 passed, 4 failed, and none of the 4 is a libniji gap**. Rust `std` test binaries start, run
threads, unwind and catch panics, fork, re-execute themselves and exit under libniji's loader and C library. The one
real failure was a test defect (fixed here, with a plant); the others were artefacts of my scratch set-up. The
switch of the gate's pass 2 itself (step 3) is **not done**: it is the story list below, with one RULING NEEDED.

## How the binaries were run (scratch tooling, not in the tree)

1. `cargo xtask so`, then `cargo nextest list --workspace` (the pass-1 binaries; no `export` feature).
2. A loader built with `NIJI_PREFIX=<scratch root>` by `cargo xtask ldso-trace --prefix <root>` (its trace matched
   the products' own: 101 and 111 names). libniji pins its sonames to `<prefix>/<libdir>` and searches only there
   when the prefix is not `/`, so the root's `lib/` holds copies of `target/niji/*.so*` and the host's
   `libgcc_s.so.1` (the GCC unwinder, which every Rust `std` binary needs: `NEEDED libgcc_s.so.1`).
3. A copy of each test binary with its `PT_INTERP` rewritten in place to `<root>/ld.so` (a short path: the field
   holds 27 bytes), and a nextest target runner (`CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER`) that runs the copy.
   A runner that runs `ld.so <binary>` does not work: about 70 test files re-execute `current_exe()`, which would
   then be the loader.
4. `ld.so --list` on a copy: `libc.so.6 => <root>/lib/libc.so.6`, `libgcc_s.so.1 => <root>/lib/libgcc_s.so.1`.
5. The same nextest filter as pass 2 (PASS0_ONLY out, later shards of PASS1_SHARD0 out), `NIJI_TEST_MALLOC_PERTURB=165`.

## The gap list (step 1)

| Test | Cause | Kind | Status |
|---|---|---|---|
| `niji-boundary::s23_time_symbols tz_period_words_per_thread_and_fork` | the test chose its branch from the block's `FLAG_OWN` word; under libniji's loader `libc.so.6` set it, while the binary's own copy of the code did not start the process (D-X.58: the process-level fact decides) | test defect | fixed (b9f3a0711): asks `selftest::runtime_started()`; plant `tz-period-ungated` CAUGHT; passes natively and under the host loader |
| `niji::skip_note_visible a_skip_prints_one_line_and_is_logged` | the test derives its expected `SKIPPED <name>:` from the binary's file name; my copies were named `<binary>-niji` | scratch runner artefact | an in-tree runner must keep the file name (copies in a sibling directory at the same depth) |
| `niji::catalogs_products` (2 tests) | my native run and my "before" run shared `target/test-oracles/` in one role tree at the same time | my own interference | both pass natively when run alone (75/75 with s23) |

Missing symbols or versions: none. Loader features `std` needs and libniji lacks: none found. Behaviours `std`
relies on that differ: none found.

## Numbers (step 4), keke, 4-core slot each

| Run | Tests | Summary (nextest wall) | Job wall | Test-seconds |
|---|---|---|---|---|
| Before: interposed pass as the gate runs it (193 binaries) | 5337 | 1448 s | 1450 s + the `export` workspace build, 337 s | 5015 |
| After: native, the same 193 binaries (from the run below) | 5337 | not separable | no build | 3747 |
| After: native, all 317 pass-1 binaries | 6759 | 1501 s | 1814 s (incl. patching 317 copies), no build | 5077 |

Same host, same slot size, different half-hours (the native run overlapped my stopped first "before" job's build
for 10 minutes, in another slot). On the same suites the native pass costs 25% fewer test-seconds (the
foreign-thread paths are gone from the harness) and needs no second workspace build; covering all 317 binaries costs
about what the interposed pass costs for 193. Jobs: `test-pass2-native-explore2` (native), `test-pass2-native-seq`
(native recheck, then before), `test-pass2-native-before` (stopped: it ran beside the native run; its build time is
the 337 s above). The before run's one failure, `hot_text_products
init_prefixed_interpreter_trace_reads_no_host_file_and_matches_the_products_trace`, builds into the same
`target/product/ld-prefix` that my scratch set-up used and overlapped my plants job; it passed in the native run.
Not chased (outside this change); INFORMATIONAL.

## Story list (step 3 and the rest; sizes S/M/L)

1. **PASS2-NATIVE-RUNNER (M).** In xtask: build the prefixed loader at a short per-tree path (`/tmp/niji2-<8 hex of
   the tree path>`: the gate's private `/tmp` keeps concurrent gates apart), a root `lib/` of links to
   `target/niji/*.so*` plus `cc -print-file-name=libgcc_s.so.1`, `PT_INTERP`-patched copies of the listed test
   binaries in `target/debug/niji-deps/` (same file names, same depth), and nextest with that runner, refusing a copy
   older than its binary. Replace the `export` build of `test_steps` and pass 1 of `affected`. Saves the whole second
   workspace build of pass 2. Unit tests for the patcher; plants.
2. **PASS2-FOREIGN-SUITE (S).** Keep `-p niji-boundary --features export,selftest` for `heap_threads`,
   `s29_rpc_clnt_svc_stubs` (the two targets with `cfg!(feature = "export")` checks), `tb_foreign_words`,
   `tb_foreign_low_words`, plus a new per-call instruction bound on the foreign `malloc`/`free` pair
   (`perf_event_open` self-count as `setlocale_posix_repeat.rs`). Note: any host-started test binary already runs
   libniji's heap on the foreign path when it calls `posix::s03_memory` directly, so the bound needs no `export`.
3. **STD-SMOKE (S).** The README and `docs/toolchain.md` §8 say Rust `std` programs are not supported. This run is
   evidence that `std` test programs run under the loader; a small in-tree smoke (one `std` binary with threads, a
   caught panic and a re-exec, run under `target/niji`'s loader) would let those lines be revised.

## RULING NEEDED

- **Coverage of `export`-only code under a native pass 2.** With `export`, direct calls in niji-boundary's tests
  reach `#[cfg(feature = "export")]` paths (`caller_alloc`'s by-symbol `reallocarray`, the `no_mangle` data and
  functions of `export.rs`/`data.rs`), and niji's `_clauses` tests get the poisoned C-family frees (affected.rs
  `c_family`). A native pass runs the pass-1 build, so those paths would be reached only through the products.
  Recommend: keep all of niji-boundary's test targets and niji's `_clauses` targets in the interposed suite until a
  story lists exactly which targets reach `export`-only code; switch every other suite.

## Not to redo

- `ld.so <binary>` as a runner (re-exec breaks). Unprefixed products (`libc.so.6` pins to `/lib/libc.so.6`, absent
  on the host). Two test passes at once in one role tree (shared `target/test-oracles/`).

## Next step

PASS2-NATIVE-RUNNER after the ruling. Scratch tooling (patcher, runner, set-up) is described above; it is ~40 lines.
