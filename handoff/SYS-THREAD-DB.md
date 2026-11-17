# SYS-THREAD-DB: libthread_db.so.1 (handoff after SYS-THREAD-DB-3)

State: branch `task/sys-thread-db-3` (continues `task/sys-thread-db-2`; merge base origin/next `ebb670553`). Role
mei:~/w/s1. Report: handoff/SYS-THREAD-DB-3-report.md (jobs, results, start-up rows); earlier run:
handoff/SYS-THREAD-DB-2-report.md.

## Done

* `libthread_db.so.1` (C layer, `crates/libc-so/placeholders/libthread_db.c`; 40 exports, GLIBC_2.2.5/2.3/2.3.3
  from versions.tsv) reading `__niji_thread_db@NIJI_PRIVATE` (`niji_rt::thread::Debug`, defined in
  `crates/boundary/src/data.rs`: the unsafe audit allows `UNSAFE(export)` only in the export files).
* The check that the descriptor's dtv header equals the loader's (`niji_ld::tls::DTV_HEADER`) lives in
  `crates/boundary/src/bootstrap.rs` (products only), NOT data.rs: naming `niji_ld` in data.rs gave every test
  build of boundary (niji-math's `selftest`) a loader edge and made a loader change select 25 niji-math suites
  (SYS-THREAD-DB-2's red gate; fixed in run 3).
* Both package flavours ship it and the `libthread_db.so` development link (`tools/deb/build.sh`, `system.sh`,
  `check-system.sh`).
* Ruling #6167: the `crates/thread-db` stub crate is removed; nothing depended on it.
* Proof (static programs, ruling #6166): `crates/core/tests/thread_db_products.rs` + `tests/oracle/thread_db_target.c`:
  gdb loads the products' library, `info threads` shows 4 threads, each backtrace reaches its function,
  `thread apply all print tv` gives 7/111/122/133; a core file written by gdb reads back the same.
* Decision record: `records/pending/SYS-THREAD-DB-2-1.toml` (D-NEW, series D-22; CI assigns the id).
* Plants: handoff/SYS-THREAD-DB-3-plants.txt (6 plants, all CAUGHT on the run-3 head).

## Decisions in force

The pending record above. Rulings #6166 (static only here; dynamic pending on DL-23), #6167 (crate removed),
#6168 (3 RELATIVE relocations accepted within start-up bounds: rows in the run-3 report).

## Open / not to redo

* DYNAMIC programs: pending on DL-23 (`_r_debug` / `_dl_debug_state` in libniji's `ld.so`). After DL-23: extend
  `thread_db_products.rs` with a dynamic target, and implement `td_thr_tls_get_addr` (link_map -> module id).
* gdb needs `set libthread-db-search-path <dir>` to pick the products' library in tests; an installed package
  puts it next to libc where gdb looks by default.
* Do not move the dtv-header assertion back into a module the test builds compile (see Done).
* `tools/deb/parity.sh` not rerun yet; the gap row is libthread_db.so.1 (ref-elf.tsv:285).

## Next step (exact)

1. Land this branch (jobs in the run-3 report). 2. After DL-23 lands: the dynamic proof above.
3. Rerun `tools/deb/parity.sh` with both flavours built and confirm the libthread_db gap row is gone.

Plan rows: plan/package-parity-audit.md row 11; spec §22.10; plan/ph14/design.md §17.
