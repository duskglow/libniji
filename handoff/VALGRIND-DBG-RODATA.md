# VALGRIND-DBG-RODATA: valgrind reads libc6-dbg (GitHub #13, the rest)

Branch `task/valgrind-dbg-rodata` (from origin/next 5e1903cce). Brief `~/state/briefs/VALGRIND-DBG-RODATA.md`.
Role mei `~/w/s4` (Skylake-SP). Job scripts and outputs: mei `~/work/VALGRIND-DBG-RODATA/` (local copies in
`~/w/.scratch/VALGRIND-DBG-RODATA/job/`). Decision D-X.150 (reserved, record written). Plants file
`handoff/VALGRIND-DBG-RODATA-plants.txt`.

## Item 1, find_rodata: cause and fix (done in code)

* valgrind 3.24.0's `coregrind/m_debuginfo/readelf.c` (valgrind source; allowed by the brief): the main object's
  first section whose name starts with `.rodata` sets the read-only data range (later `.rodata*` merged only when
  adjacent); for a separate debug file, `find_rodata` matches only a section named exactly `.rodata` and asserts
  its size (plus adjacent `.rodata*` after it) equals that range. libc.so.6 had `.rodata1` (the start-up group,
  D-X.133/D-X.143, first) then `.rodata` → mismatch → abort for every tool once libc6-dbg is installed.
* Fix: `RODATA1_FIRST` (xtask/src/so.rs) names the output section `.startup_ro` (inputs still `.rodata1`);
  `LIBC_SO_STARTUP_RO` const; anchor and segment checks use it; new `check_debug_reader_rodata` /
  `debug_reader_rodata_errors` on libc.so.6 and the loader in `cargo xtask so`; unit test
  `libc_so_has_one_read_only_data_section_for_valgrind`; `startup_group_first_keeps_one_read_only_segment_at_every_text_size`
  now also asserts the fragment's output has no second `.rodata*` (and that GNU ld's default does).
  `tools/deb/check-system.sh`: packaged shared objects (`*.so`, `*.so.*`) must have no `.rodata*` other than `.rodata`.
* Rejected: debug-file-only rename (debug file section names would differ from the library's); dropping/merging the
  group (undoes D-X.133/D-X.143). See the D-X.150 record body.
* Static programs (getconf, ldd and any static link of libc.a) still have `.rodata` + `.rodata1` (GNU ld default
  script); valgrind's main-vs-debug rounding then disagrees too (INFORMATIONAL, not this story).

## Measured so far

* Job `valgrind-dbg-rodata-acc214` (mei, rc 1 as expected): 21.4 snapshot debs in the swapped trixie root
  (vgacc.sh): stock memcheck true 0 errors, bad 1, cxx 1 (32 lost), leak 1 (9 bytes lost, frames named), helgrind
  and drd on thr 0 errors; libniji 21.4 + whole libc6-dbg: every tool (memcheck x4, helgrind, drd)
  `find_rodata: Assertion 'di->rodata_size == sh_size' failed`.
* Job `valgrind-dbg-rodata-build1` (mei; head a9c01a0d8 = the fix before the check-order commit): head so, base
  so (origin/next's so.rs via `so.patch -R`), `cmp_load.py` base vs head for libc.so.6 and the loader, xtask unit
  tests, system debs, check-system, vgacc on head debs (`acc-head/`, incl. `--track-origins` runs of /bin/true and
  ./leak for item 3). STILL RUNNING at the refresh: read `~/logs/valgrind-dbg-rodata-build1.log`
  (`grep -E "^BUILD1|^==|^cmp_load|^  0x"`) and `~/work/VALGRIND-DBG-RODATA/acc-head.txt`.

## Exact next steps

1. `~/bin/jobwait valgrind-dbg-rodata-build1`; check: cmp_load shows only build-id note + build tag bytes differ
   (D-X.150's body claims this: correct it if not), unit tests ok, check-system PASS, vgacc head: memcheck
   summaries equal stock except item 3's extra loader report.
2. Item 3: read `acc-head/out/trace-true.err` (the `Conditional jump` record with origin); the old report was
   `niji_boundary::ld::link::link` at the `cache.get().and_then(Option::as_ref).map_or` line
   (crates/boundary/src/ld/link.rs ~357; `LazyCache` = `OnceCell<Option<(&[u8], Cache)>>`). Fix if a real
   uninitialised read; otherwise report what memcheck misses.
3. Copy the system products for the pkg plant:
   `pr=$(dirname $(find ~/w/s4/target/vw -name libc.so.6 -path "*/target/niji/*" | head -1)); cp -r $pr /mnt/zram/claude/VALGRIND-DBG-RODATA/products-head`.
4. Sync the final head; clippy + static; start gate (`cargo xtask affected --run`), start-up checks
   (`cargo xtask test`; `NIJI_FULL_PROFILE=1 cargo xtask so && NIJI_FULL_PROFILE=1 cargo test -p niji --test hot_text_products`)
   and plants (`cargo xtask plants handoff/VALGRIND-DBG-RODATA-plants.txt --dir /mnt/zram/claude/VALGRIND-DBG-RODATA/plants-1`).
   Rerun vgacc on the final head's debs if the code changed after build1 (item 3 fix).
5. precheck, report json `handoff/reports/VALGRIND-DBG-RODATA.json`, handback-check, hand back.
