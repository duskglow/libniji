# DL-ATEXIT-OWNER: registering exit/fork handlers without libc_nonshared.a (owner rulings #4860-#4862)

**State (run 1, 2026-11-14, INCOMPLETE: context checkpoint).** Branch `task/dl-atexit-owner` (from origin/next
f53c74611), role shiki:~/w/sh2. The head is in the hand-back and in handoff/reports/DL-ATEXIT-OWNER.json.
Prerequisites checked present on next: DL-LIBC-NONSHARED (the archive, D-27.28), DL-17 dlclose (Finis, counted readers),
DL-16c `_dl_find_object` (lock-free `read_section`), DL-ATFORK-FINALIZE (5-word atfork nodes, `forget_dso`).
Read first: handoff/DL-NONSHARED-OPTOUT.md (design, probes P1-P3), this note. Reserved decision id: **D-27.29**.

## Built in this run (commit `WIP: Add the LIELLA_OWNER_1 ...`; NOT tested yet beyond the build job below)
* **Exports.** `csys/owner/owner.S`: three frame-free shims, `.symver`ed to `atexit@LIELLA_OWNER_1`,
  `at_quick_exit@LIELLA_OWNER_1`, `pthread_atfork@LIELLA_OWNER_1` (non-default, single `@`). Each loads `[rsp]` (the
  caller's return address) into the next argument register and jumps to the library's local `liella_owner_*`
  entry (`crates/boundary/src/owner.rs`, `internal_entry!`). Linked into libc.so.6 only (xtask/src/so.rs step 2).
  Version script: an empty `LIELLA_OWNER_1 { };` node after LIELLA_PRIVATE. `take_owner_node` (so.rs) checks each of
  the three is exported exactly once as a NON-default version and nothing else sits at the node, then removes them
  from the versions.tsv comparison. Node name decided: `LIELLA_OWNER_1`.
* **Attribution (a).** `owner::module_of(ret)`: the loader's `_dl_find_object` hook (`LOADER_FIND`, lock-free reader
  section, so it is safe in a constructor under dlopen's lock). The owner is `dlfo_map_start`, or 0 (the process) when
  no module or no loader. Exit entries get kind `OPTED_EXIT` (5) and quick entries `OPTED_QUICK` (6); see
  `crates/legacy/src/exit_handlers.rs`. `invoke` calls both as `fn()`. `__cxa_finalize(dso)` never matches them,
  because the owner is a map start, not a `__dso_handle`. Atfork nodes store `owner | 1`; bit 0 marks an opted-in node
  and never equals a pointer-aligned `__dso_handle`.
* **Unload with the net (b).** In ld.so, `Finis` now carries each object's `[lo, hi)`. For a `dlclose` only,
  `close_begin` looks up libc's `__liella_owner_unload` (LIELLA_PRIVATE, `start::private::OWNER_UNLOAD`,
  cached in `Loader::owner_unload`). `Finis::run` calls it after each object's finalisers, unlocked, with the code
  still mapped. libc's `owner::unload` then:
  * runs (`finalize_range`) every OPTED_EXIT entry whose owner OR handler lies in the range;
  * forgets the matching OPTED_QUICK entries;
  * forgets the opted-in atfork nodes whose owner or any of the three handlers lies in the range (`fork::forget_opted`).
  Stub, `__cxa_atexit` and plain-kind entries are untouched (ruling #4862: no differences row, no compat flag).
* **Link-stub directory.** `cargo xtask so` writes `target/liella/liella-owner/libc-owner.so` (from
  `csys/owner/stub.S`: soname libc.so.6, the three names at `@@LIELLA_OWNER_1`, bodies `ud2`, never loaded) and
  `liella-owner/libc.so` = `GROUP ( libc-owner.so libc.so.6 AS_NEEDED ( ld-linux-x86-64.so.2 ) )`. An opted-in link
  uses `-L<dir>/liella-owner -L<dir>`.

## Open / to verify first (next run)
1. **First fix.** Job `dl-atexit-owner-build-2` on shiki (`~/work/DL-ATEXIT-OWNER/check1.sh`):
   * `cargo xtask clippy` passed on the head (workspace, all targets, -D warnings).
   * `cargo xtask so` linked libc.so.6 with the empty `LIELLA_OWNER_1` node, and the three names came out as
     `name@LIELLA_OWNER_1`, non-default as intended.
   * It then FAILED in `check_compat_exports` (`crate::compat::export_problems`): "libc.so.6 exports
     atexit@LIELLA_OWNER_1, which spec/abi/versions.tsv has no compat row for" (and the same for the other two).
   * Fix: leave `OWNER_NODE` out of that check, as `take_owner_node` does for `check_version_sets`. Then rerun
     check1.sh.
   Still unverified:
   * an opted-in link gets ONE `DT_NEEDED libc.so.6`, although two inputs have that soname (else merge the stub
     into one interface object);
   * `Loader`'s new field starts at 0 (zeroed state, as `thread_walk`).
   * Build-1 failed on clippy only (unused `OWNER_STUB`, fixed).
2. Tests (products, `crates/boundary/tests/products/`):
   * (i) a default link of a program plus .so: `readelf -V` shows no LIELLA_OWNER_1 need, and it still runs on the
     reference (extend `nonshared_two_way`);
   * (ii) an opted-in program plus libA/libB under Liella: P2 cases 1-3. Case 2, with B closed first, runs B's
     handler at B's unload (the net). The opted atfork and quick-exit handlers are dropped at unload. A constructor
     of a dlopen'ed lib registers (under the loader lock): it neither deadlocks nor misattributes;
   * (iii) P3 against the reference: the opted-in program exits 1, with stderr naming
     `version \`LIELLA_OWNER_1' not found`;
   * (iv) a stub-registered entry is NOT run at its handler module's unload (#4862).
   Port the prototype's 8 tests to the product types, or drop `owner_proto` once the product tests cover them
   (its 9 plants move to `owner.rs`/`exit_handlers.rs`/`fork.rs`).
3. Plants (brief): the prototype's 9 re-aimed at product code, plus:
   * a non-default export made default (`Some(false)` check, or `.symver ...@@`);
   * the net applied to stub entries (drop the `e[0] == OPTED_EXIT` filter);
   * a registration under the loader lock that deadlocks or misattributes (e.g. `module_of` returning 0, or taking
     STATE).
4. #4861: an opt-in build setting for distributions (env var for `cargo xtask so`) that adds the allocated
   `.note.liella.nonshared` note and `.gnu.warning.<sym>` sections to the archive members. Off by default: the default
   emits no warning. Also an ld.so debug report of modules that still use stubs. Neither is built yet.
5. Decision record D-27.29 (`cargo xtask records new decision D-27.29`): the node name, mechanism (a)+(b), the bit-0
   atfork marker, unload after finalisers. Spec: spec/behaviour/dynlink.md, a paragraph on the opt-in. User docs:
   the opt-in, the migration path (OPTOUT note section 5), and that nothing is deprecated by default.
6. Start-up checks (brief, 2026-11-14): this touches the dynamic loader. The final gate also runs `cargo xtask test`
   and the `LIELLA_FULL_PROFILE=1` hot_text_products check.

## Not to redo
The probes P1-P3 (OPTOUT note) and the prerequisite check above.
