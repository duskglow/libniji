# TEST-SCRATCH-CLEANUP

State: branch task/test-scratch-cleanup (from origin/next 8e34f9c72). Role mei:~/w/s3. Partial: the main
leakers are fixed; the remaining sites below are the exact next step.

## The rule (in force)
`crates/core/tests/support/scratch.rs` `Scratch` is the one way a test makes a per-process file or directory:
removed on drop when the test passes; when the thread is unwinding (the test failed) it is kept and
`scratch kept (the test failed): <path>` goes to stderr. `Scratch::dir_in(base, tag)` makes a directory under
a given base (tests whose corpus must be outside target/tmp or canonical). Other crates include it with
`#[path = "../../core/tests/support/scratch.rs"] mod scratch;`. Tests: `scratch_base.rs`
(`passing_scope_removes_its_directory`, `failing_scope_keeps_its_directory`); plants in
`handoff/TEST-SCRATCH-CLEANUP-plants.txt`.

## Inventory (before), from the build roles on the Skylake-SP host (pid-named entries summed over roles)
Leaked on every PASSING run (fixed here):
- `target/crt-products/<case>-<pid>` (psabi, pg, hello, cxx 144 each; libm-compat 115; sigmask-compat 111; maps 13),
  crates/core/tests/crt_products.rs `work()`: never removed. Now `Scratch::dir_in(target/crt-products, case)`.
- `target/dl-search-<pid>` (285), crates/ld/tests/search_cache.rs: never removed; also `dl-trace-interp-`,
  `ldso-program-name-` in the same file (removed only on success). All three now Scratch.
- `target/ld-names-<tag>-<pid>` (~28 per tag), crates/ld/tests/object_names.rs: never removed. `Corpus.dir` is a Scratch.
- `target/tmp/ld_symbols_<pid>` (33), crates/ld/tests/symbols.rs: never removed (read into memory). Scratch in the init.
- `target/tmp/elf_readelf_parity_<pid>` (24), crates/ld/tests/elf_readelf_parity.rs: a process-wide OnceLock
  corpus (never dropped; under nextest built per test anyway). Now `corpus()` returns `(Scratch, Vec<Member>)`.
- `target/tmp/sym_monstartup.<pid>`, `sym__mcleanup.<pid>` (8 each), crates/boundary/tests/s28_profiling_symbols.rs
  `child_files`: never removed. Now Scratch.
- `target/tmp/skip-precond-{locale,cc}-<pid>` (8 each), crates/core/tests/skip_precondition_fails.rs. Now Scratch.
- `target/forced_unwind-<tag>-<pid>` (1), crates/core/tests/forced_unwind_products.rs: removed only at the end of a
  passing test (left by early returns/failures). Now Scratch.
Left only by FAILING runs (remove at the end of the success path; NOT converted yet: next step):
- crates/ld/tests/{dlclose,dlopen_process_objects,init_order,ldso_run,ldso_search,map,reloc,rendezvous}.rs
  (`ld-dlclose-`, `ld-process-objects-`, `ld-init-order-`, `ldso-run-`, `ldso-search-`, `ld-map-`, `ld-reloc-`,
  `ld-rendezvous-`): their `dir()` helpers -> `Scratch::dir_in(&target(), ...)`.
- crates/core/tests/popen_system_eintr.rs `eintr-<pid>-{sys,pclose}` (8 each): the dir is SHARED with a re-executed
  child that calls the same helper; a child-side Scratch would remove the parent's dir. Needs the parent to own the
  guard and the child to take the path from the environment.
Owed #17 (CARGO_TARGET_TMPDIR inline sites, ~30, `git grep -n CARGO_TARGET_TMPDIR crates`): not done here. Most
have their own guard types; the ones writing without `create_dir_all` (s12_stdio_clauses.rs:107,
native_v1.rs:533, appendix_a_primitives.rs:3435) are the priority.

## Results (head after commit 2; mei s3)
- clippy (workspace, all targets, -D warnings) and `cargo xtask static`: clean.
- plants `test-scratch-cleanup-plants`: 3 CAUGHT of 3 (handoff/TEST-SCRATCH-CLEANUP-plants.txt).
- gate `test-scratch-cleanup-gate` (so + affected --run, direct tier): 3475 passed, 1 FAILED:
  heap_meta_products `the_heap_keeps_address_space_near_its_use` (VmSize bound, heap_meta_products.rs:189). That
  file does not use scratch; it ran alongside the plants job; rerun alone on the same head
  (`test-scratch-cleanup-heapmeta`): 5 passed. INFORMATIONAL (possible load sensitivity), not chased.
- failing test keeps its dir: `failing_scope_keeps_its_directory` (--nocapture, same heapmeta job) printed
  `scratch kept (the test failed): <target>/tmp/niji-scratch-fail-<pid>-0`.
- growth `test-scratch-cleanup-growth` (the converted suites run twice with cargo test): pid-named entries in
  target/, target/tmp, target/crt-products 31 -> 31 -> 31, du target 6468 -> 6468 -> 6468 MiB. Across the gate
  the count went 27 -> 31, all `eintr-N` (popen_system_eintr, not converted yet).
- NOT DONE: the brief's step 3 with a FULL `cargo xtask test` twice (only the converted suites + the gate).

## Next step (exact)
1. ld `dir()` helpers in dlclose, dlopen_process_objects, init_order, ldso_run, ldso_search, map, reloc,
   rendezvous -> `Scratch::dir_in(&target(), "<old-prefix>-<name>")`, include scratch.rs as search_cache.rs does;
   `.arg(&d)` needs `&*d`.
2. popen_system_eintr: parent owns a Scratch, passes its path to the re-executed child via env.
3. Owed #17 sites; then two `cargo xtask test` runs with growth.sh (~/work/test-scratch-cleanup/growth.sh on mei,
   widen its suite list) and du; gate, plants, precheck, handback-check.
