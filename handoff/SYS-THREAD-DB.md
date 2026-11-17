# SYS-THREAD-DB: libthread_db.so.1 (handoff after SYS-THREAD-DB-2)

State: branch `task/sys-thread-db-2` (on origin/next `ebb670553`, continues `task/sys-thread-db` 01530fdd5). Role
keke:~/w/r1. Report: handoff/SYS-THREAD-DB-2-report.md (jobs, results, start-up rows).

## Done

* `libthread_db.so.1` (C layer, `crates/libc-so/placeholders/libthread_db.c`; 40 exports, GLIBC_2.2.5/2.3/2.3.3
  from versions.tsv) reading `__niji_thread_db@NIJI_PRIVATE` (`niji_rt::thread::Debug`, now defined in
  `crates/boundary/src/data.rs`: the unsafe audit allows `UNSAFE(export)` only in the export files).
* Both package flavours ship it and the `libthread_db.so` development link (`tools/deb/build.sh`, `system.sh`,
  `check-system.sh`).
* Ruling #6167: the `crates/thread-db` stub crate is removed (workspace, lock, arch/bench/affected lists, plan lists);
  nothing depended on it.
* Proof (static programs, ruling #6166): `crates/core/tests/thread_db_products.rs` + `tests/oracle/thread_db_target.c`:
  gdb loads the products' library, `info threads` shows 4 thread-library threads, each backtrace reaches its
  function, `thread apply all print tv` gives 7/111/122/133; a core file written by gdb reads back the same.
  Passed on keke (gdb 15.1) in job sys-thread-db-2-lint2.
* Decision record: `records/pending/SYS-THREAD-DB-2-1.toml` (D-NEW, series D-22; CI assigns the id; code comments
  name "SYS-THREAD-DB-2-1").
* Plants: handoff/SYS-THREAD-DB-2-plants.txt (6: fs_base index, thr_iter state filter, tlsbase direction, magic,
  LWP = pid, descriptor next offset).

## Decisions in force

The pending record above (design unchanged from run 1). Rulings #6166 (static only here; dynamic pending on DL-23),
#6167 (crate removed), #6168 (3 RELATIVE relocations accepted within start-up bounds: rows in the report).

## Open / not to redo

* DYNAMIC programs: pending on DL-23 (`_r_debug` / `_dl_debug_state` in libniji's `ld.so`). After DL-23: extend
  `thread_db_products.rs` with a dynamic target, and implement `td_thr_tls_get_addr` (link_map -> module id).
* gdb needs `set libthread-db-search-path <dir>` to pick the products' library in tests; an installed package
  puts it next to libc where gdb looks by default.
* `tools/deb/parity.sh` rerun not done this run (budget); the gap row is libthread_db.so.1 (ref-elf.tsv:285).

## Next step (exact)

1. Read the jobs in the report; if all green the branch lands. 2. After DL-23 lands: the dynamic proof above.
3. Rerun `tools/deb/parity.sh` with both flavours built and confirm the libthread_db gap row is gone.

Plan rows: plan/package-parity-audit.md row 11; spec §22.10; plan/ph14/design.md §17.
