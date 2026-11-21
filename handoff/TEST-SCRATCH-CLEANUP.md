# TEST-SCRATCH-CLEANUP

State: branch task/test-scratch-cleanup (from origin/next 8e34f9c72). Role mei:~/w/s3. Code done (legs 1-2);
proof below. HEAD: see `git log -1` (handoff/report commit on top of 3d45a86c9).

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
None for the story if the results above are green; follow-up (optional): the success-path-removal sites listed
under owed #17 to `Scratch`.
