# AFFECTED-COVERAGE: test sets no gate or train ran, and two unmapped paths

Branch `task/affected-coverage` (from origin/next 4f32aa946). Code: `xtask/src/coverage.rs` (new), `xtask/src/affected.rs`,
`xtask/src/main.rs`. Plants: `handoff/AFFECTED-COVERAGE-plants.txt`. Report record: `handoff/reports/AFFECTED-COVERAGE.json`.

## Runners (what builds which tests)

- **Direct gate** `cargo xtask affected --run`: per crate `cargo test -p <c>` (workspace features: niji-boundary gets
  `selftest` from its self dev-dependency), pass 1 native and pass 2 interposed (`export,selftest`).
- **Train** `cargo xtask affected --tier full --run` (ops ci_gate_cmd) = clippy `--all-features` (builds, runs nothing)
  + `cargo xtask test`: `cargo test --workspace`, `native_tests` (pass 1 over `--workspace`), `interposed_suite`,
  conformance, native-examples, rust-dylib, symgap.
- **Nightly / release**: the same plus the `#[ignore]` long sweeps (`longsweeps.rs`, which has its own static check).

## Coverage table (before -> after)

| Set (crate, features, targets) | Tests | Before | After |
|---|---|---|---|
| niji-boundary `ld` --lib (src/ld, arch, bootstrap) | 0 on next (VALGRIND-LOADER-VIEW-2 adds the first) | none (clippy compiles only) | direct: ld/, ld.rs, arch/, bootstrap.rs, lib.rs, s27_dynlink.rs, dispatch.rs; train: step `test-sets` |
| niji-boundary `portable-scan` --lib --test machine_scanners | 26 lib + 52 | none | direct: strz, ffi, dispatch, selftest, build.rs, machine_scanners; train |
| niji-math `fastpath-stats` --lib | 1 (`stats::tests`) | none (by hand, docs/libm-fastpath-plants.md) | direct: stats.rs; train |
| niji-native `dylib` --lib | 3 (`dylib::tests`); 15 in the run | none (rust-dylib builds the example only) | direct: dylib.rs, lib.rs, ffi.rs; train |
| fuzz library (`fuzz/`, own workspace) | 1 | none (fuzz-check type-checks the bins) | direct: fuzz/src/lib.rs; train |
| niji-boundary targets, pass 1+2, direct tier | all selected | only when mem/rt/kernel/abi/boundary changed | whenever selected (`second_pass`) |
| niji-boundary pass 2, train | all | already run (native_tests --workspace; the binaries hold niji_boundary, so CI-GATE-DIET keeps them) | unchanged; the check fails if the train drops it |
| `csys/util/iconv-names.h` | - | FULL (train's) | niji::iconv_cmd_names + `cargo xtask so` |
| `spec/encoding/reference/` | - | FULL (train's) | encodings-gen-check + the mapping of `crates/core/src/iconv/cjk_tables.rs` |
| product-only features (boundary runtime, libm, libresolv, ldconfig; abi export) | 0 gated tests | - | the check proves none |
| crates outside the workspace (libc-so, libm-so, libresolv-so, ld-so, ldconfig-bin) | 0 | - | the check proves none |

Found by the check itself: the niji-native `dylib` tests (3) and that `dispatch.rs` compiles code under `ld`.

## Cost (seconds; Skylake-SP host mei, warm dependencies, each set its own build)

ld 47, portable-scan 55, fastpath-stats 45, fuzz lib 29, dylib ~0-few (small crate). Per train (`test-sets` step, all
five): about 176 s when every set rebuilds (a train that touches boundary rebuilds ld and portable-scan; math and
native only when they change). Per direct gate: only the sets a changed file covers (0 s for most changes). The pass-2
rule adds the native and interposed runs of the selected boundary targets plus the products build when no other
selection needed it (gates whose change is outside mem/rt/kernel/abi/boundary yet selects boundary targets).

## Self-check

`cargo xtask static` step `test-coverage` (`coverage::check`, also the unit test
`the_tree_has_no_test_set_without_a_runner`): every member feature is in exactly one of RUNS / RUN_BY / PRODUCT_ONLY;
a product-only (or examples-only) feature gates no test code (`cfg(all(test, feature))`, gated `mod x;` with tests,
gated inline module with tests); every RUNS row names a real crate, feature, target and existing covers, and covers
every file of what it builds (src/, tests/, build.rs) that names its feature or cfg (cfg_attr lines excepted); a
non-member crate under crates/ holds no test; the fuzz workspace's tests have a row; `cargo xtask test` keeps
native_tests over `--workspace`, interposed_suite and `coverage::run_all`.

## Jobs (mei, role s4)

check3 (clippy, 109 xtask tests, static: green), gate3, plants3 (9 plants). SUPERSEDED: gate, plants, gate2, plants2
(stopped: a wrong host class in a comment; gate2/plants2 had started on an unsynced tree, sync rc 4).

## Open / next

- RULING NEEDED: none required. Recommendation: a D-X row for the pass-2 rule and the test-sets step (pending id: D-NEW).
- INFORMATIONAL: the rest of `spec/encoding/` (unicode/, whatwg/ data) is still unmapped (FULL, train's).
- INFORMATIONAL: `~/bin/kill-label <gate label> --kill` left the inner `cargo xtask affected --run` of `~/bin/gate`
  running; `kill-mine s4 --kill` ended it.
- Next: when VALGRIND-LOADER-VIEW-2 lands, its ld unit test runs in the `niji-boundary+ld` set (no change needed).
