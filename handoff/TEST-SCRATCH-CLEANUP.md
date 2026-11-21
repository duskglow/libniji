# TEST-SCRATCH-CLEANUP

State: branch task/test-scratch-cleanup-3 (TEST-SCRATCH-CLEANUP-3, continuing task/test-scratch-cleanup-2 7bc7ac116),
origin/next 4f32aa946 merged in. Role mei:~/w/s2. Code done; proof under "Leg 4". HEAD: see
`git log -1` (handoff/report commit on top of ddb0699d5).

## Leg 4 (TEST-SCRATCH-CLEANUP-3): CI red E0277 fixed
- CI red on 7bc7ac116 was a semantic conflict: next's new `map_malformed_dependency_refused_at_start`
  (crates/ld/tests/map.rs, COMPAT19-LDSO-COST-2) passed the `dir()` result (now a Scratch guard) to
  `Command::env`, which needs `AsRef<OsStr>`. Merged origin/next 4f32aa946; ddb0699d5 passes `d.as_os_str()`
  and drops the test's own `remove_dir_all` (the guard removes on pass, keeps and names on failure).
- Step-1 grep of every line next added since the last merge (process::id / CARGO_TARGET_TMPDIR / temp_dir /
  create_dir / remove_dir): only that one test.
- Results on ddb0699d5 (mei s2): inherited head 7bc7ac116 clippy + static clean (test-scratch-cleanup-3-lint0).
  Merged head: `cargo build --tests --workspace` + clippy -D warnings + `cargo xtask static` clean, no E0277
  (test-scratch-cleanup-3-lint1). Gate test-scratch-cleanup-3-gate (so + affected --run --base origin/next,
  direct tier): 8393 passed, 0 failed, 776 ignored, SKIPPED lines 0, peak 5.1G, no `scratch kept` line.
  Plants test-scratch-cleanup-3-plants (handoff/TEST-SCRATCH-CLEANUP-plants.txt): 3 CAUGHT of 3, no WARN.
  du before/after the lint1 build + gate: pid-named entries in target/, target/tmp, target/crt-products
  0/0/0 -> 0/0/0; crt-products 1 -> 1 MiB, tmp 1 -> 1 MiB; du target 3862 -> 8507 MiB, of which debug/
  2352 -> 6796 MiB (cargo build state of the full `--tests --workspace` build and the gate's build).
  NOT VERIFIED: per-directory attribution of the remaining +201 MiB outside debug/ (no full listing before).

## Leg 3 (TEST-SCRATCH-CLEANUP-2): CI merge conflict resolved
- 709f3c1ea merge of origin/next: nscache.rs keeps next's `Dir::under` (region dir on a chosen base);
  `Dir::new` and the off-tmpfs candidates take `scratch::base()`; Drop already `scratch::release`.
  dlopen_process_objects.rs keeps next's `products()`/`named_libc()` split; `corpus()` keeps the Scratch guard;
  next's new `many_corpus()` (`target/ld-process-objects-many-<pid>`, removed only at the end of a pass) now
  returns a Scratch guard too.
- bf745ec74: rendezvous.rs `rendezvous_reference_sized_copy_says_version_1` (new on next,
  `target/ld-rendezvous-refcopy-<pid>`, removed only on pass/skip) -> Scratch guard.
- Step-1 grep (pid-named / `CARGO_TARGET_TMPDIR` / `temp_dir()` lines next brought in): only the three above,
  plus xtask/tests/tz_tools.rs `scratch()` under `/tmp` (not target/), removed at the end of each passing
  test: left as is (same class as the success-path-removal sites below).
- Results on bf745ec74 (mei s2): clippy -D warnings + `cargo xtask static` clean (test-scratch-cleanup-2-lint1;
  inherited head 41304e7f5 also clean, -lint0). Gate test-scratch-cleanup-2-gate (so + affected --run --base
  origin/next, 197 targets): 8392 passed, 0 failed, 776 ignored, SKIPPED lines 0, peak 5.7G. Plants
  test-scratch-cleanup-2-plants (handoff/TEST-SCRATCH-CLEANUP-plants.txt): 3 CAUGHT of 3, no WARN.
  Full `cargo xtask test` test-scratch-cleanup-2-xtest: rc 0 (7017 + 4862 + 128 + 46 passed, conformance 142),
  pid-named entries in target/, target/tmp, target/crt-products 0 -> 0 (crt-products/ and tmp/ empty after);
  du target 7032 -> 11263 MiB, of which debug/ is 9402 MiB after (cargo build state for the builds
  `cargo xtask test` makes and the gate does not, as in leg 2's run 1). NOT VERIFIED: a per-directory du
  before the run (only the total was taken).

## The rule (in force)
`crates/core/tests/support/scratch.rs` is the one way a test makes a per-process file or directory:
- `Scratch` (`path`, `dir`, `path_in`, `dir_in`, `sock_dir`, `adopt`): removed on drop when the test passes; when
  the thread is unwinding (the test failed) it is kept and `scratch kept (the test failed): <path>` goes to stderr.
- `scratch::release(p)`: that drop rule, for the test files' own guard types (Tmp, Dir, Fx, ChildPath, ...), which
  now call it from their `Drop`.
- `scratch::base()`: `CARGO_TARGET_TMPDIR`, made if missing. No test builds a path from `CARGO_TARGET_TMPDIR`
  inline any more except: appendix_a_primitives `child_path` (a forked child, no allocation; its parent's
  `ChildPath::new` makes the base), skip_note_visible and support/sync_trap (both make the base themselves).
Other crates include it with `#[path = "../../core/tests/support/scratch.rs"] mod scratch;`; every crate root
that includes `products/mod.rs` includes it (products uses `crate::scratch::base()`).

## Inventory (before), pid-named entries summed over the Skylake-SP host's build roles
Leaked on every PASSING run (fixed in leg 1, 0ee1555e0):
- `target/crt-products/<case>-<pid>` (psabi, pg, hello, cxx 144 each; libm-compat 115; sigmask-compat 111;
  maps 13), crt_products.rs `work()`. `target/dl-search-<pid>` (285) search_cache.rs, plus `dl-trace-interp-`,
  `ldso-program-name-`. `target/ld-names-<tag>-<pid>` (~28/tag) object_names.rs. `target/tmp/ld_symbols_<pid>`
  (33) symbols.rs. `target/tmp/elf_readelf_parity_<pid>` (24). `target/tmp/sym_{monstartup,_mcleanup}.<pid>`
  (8 each) s28_profiling_symbols.rs. `target/tmp/skip-precond-{locale,cc}-<pid>` (8 each).
  `target/forced_unwind-<tag>-<pid>` (1).
- `target/tmp/eintr-<pid>-{sys,pclose}` (8 each) popen_system_eintr.rs (leg 2, 3d45a86c9): the directory is shared
  with a re-executed copy; the test process owns it (`Scratch::adopt`), the copy only remakes it.
Left only by failing runs or early returns (now Scratch, leg 2, 831f4bbe8): crates/ld/tests/{dlclose,
dlopen_process_objects,init_order,ldso_run,ldso_search,map,reloc,rendezvous}.rs.
Owed #17 (leg 2, 3d45a86c9): ~30 inline `CARGO_TARGET_TMPDIR` sites in 21 files -> `scratch::base()`; their own
guards' Drop -> `scratch::release` (they removed even on failure; now kept and named). s13_filesystem_clauses'
`Scratch` keeps its own permission/deep-tree removal. Sites that remove at the end of the success path
(products/mod.rs x5, exec_stage, noproc_products, oracle_freshness, secure_env, no_proc_lane) keep that
(failure keeps the directory, without the stderr line): converting them is optional follow-up.

## Results (final code head 3d45a86c9, mei s3)
- clippy (workspace, all targets, all features, -D warnings) + `cargo xtask static`: clean
  (test-scratch-cleanup-clippy4).
- plants `test-scratch-cleanup-plants2` (handoff/TEST-SCRATCH-CLEANUP-plants.txt): 3 CAUGHT of 3, no WARN.
  The keep/remove plants hit the panicking check, which now lives in `release()` (used by Scratch and every guard).
- gate `test-scratch-cleanup-gate2` (so + affected --run --base origin/next): 8379 passed, 0 failed, 776 ignored; SKIPPED lines 0; peak 4.3G.
- two full `cargo xtask test` runs with counts, `test-scratch-cleanup-xtest`: both rc 0 (run 1: 6995 + 4857 + 128 + 46
  passed). pid-named entries in target/, target/tmp, target/crt-products: 31 -> 31 -> 31, the same set each time,
  all left by runs before this branch (names such as ld-process-objects-many and ld-rendezvous-refcopy no longer
  exist in the tree; eintr-N from leg 1's gate, before popen_system_eintr was converted). du target (MiB):
  7491 -> 10566 -> 10638. Run 1's +3075 is cargo build state and products for the builds `cargo xtask test`
  makes and the gate does not (the role had run only gates); run 2's +72 is in cargo's own directories
  (debug/niji-deps relinked 543 MiB of files in run 2); fixed-name test dirs (test-oracles/, test-scratch/,
  product/) are rewritten in place, and test-oracles/outputs holds only the latest nextest run's files.
  NOT VERIFIED: a file-by-file attribution of run 2's +72 MiB (no listing was taken before run 2).
- failing test keeps its dir (leg 1): `failing_scope_keeps_its_directory` printed
  `scratch kept (the test failed): <target>/tmp/niji-scratch-fail-<pid>-0`.
- leg 1: gate 3475 passed, 1 failed (heap_meta_products VmSize bound, alongside plants; 5 passed alone):
  INFORMATIONAL, not chased.

## Next step (exact)
None for the story (leg 4 green); land task/test-scratch-cleanup-3. Follow-up (optional): the
success-path-removal sites listed under owed #17, and xtask/tests/tz_tools.rs `scratch()`, to `Scratch`.
