# SL-CRT: crti.o, crtn.o, grcrt1.o, Mcrt1.o (plan/system-libc.md §6 item 4)

State: branch task/sl-crt-2 (continues task/sl-crt), code head 0702ed168. SL-CRT-2 finished the open steps.
Role mei:~/w/s2 (Skylake-SP). Decision pending: records/pending/SL-CRT-1.toml (D-NEW, series D-27; CI assigns the id).

## Done
* SL-CRT (33bd94b39): `csys/start/crti.S`, `crtn.S`, `Mcrt1.S`; `gcrt1.S` with `-DLIELLA_STATIC_PIE` = grcrt1.o.
  `cargo xtask so` builds `liella-{grcrt1,crti,crtn,Mcrt1}.o`; remap PRODUCTS and contract PRODUCT-PATH-REMAP list
  them; tools/deb/build.sh installs gcrt1/grcrt1/crti/crtn/Mcrt1.o in liella6-dev; docs/install-packages.md lists them.
* GCC 13 driver specs (`gcc -dumpspecs`): crti/crtn in every link incl. -shared; gcrt1 for -pg/-p/-profile;
  grcrt1 for those + -static-pie; Mcrt1.o named nowhere (empty but for .note.GNU-stack).
* SL-CRT-2 (0702ed168):
  - The inherited `cargo xtask static` was red on the pending record (tests must be a list, body one line): fixed.
  - `start_files_follow_the_psabi`: Mcrt1.o check is now "no allocated section of non-zero size" (gas adds empty
    .text/.data/.bss).
  - `-pg` PIE gap found and fixed: `_mcleanup` wrote run-time pcs, so gprof matched nothing in a PIE. Probe of the
    reference (mei, gmon_pg.c, `-pg -fPIE`, dynamic and `-static-pie`): histogram low pc = 0 = `__executable_start`
    (link-time pcs) for both. crates/boundary/src/posix/s28_profiling.rs now subtracts
    `process::executable_bias()` (new helper beside `is_read_only`) in the hist and arc records; non-PIE bias is 0,
    so gmon_products' static case is unchanged. The test checks low pc == `nm __executable_start` and gprof shows `main`.
  - Plants file handoff/SL-CRT-plants.txt: 6 plants (5 from SL-CRT + `gmon-pie-no-bias`).
* crt_products (4 tests) and gmon_products pass; clippy and static clean (job sl-crt-2-t1).

  - The first gate (sl-crt-2-gate, head 0702ed168) went red on `s28_1_c_1`: it asserted run-time pcs from its PIE
    test program (the old gap). 5d13765a6: the child prints its load bias from /proc/self/maps, the parent subtracts it.

## Jobs on the final head 5d13765a6 (mei)
sl-crt-2-gate2 (direct tier: 32 suites, 11820 passed, 0 failed; conformance 137/137; peak 15G), sl-crt-2-plants2
(6 CAUGHT), sl-crt-2-xtest2 (`cargo xtask test`), sl-crt-2-hotprof2 (FULL_PROFILE so + hot_text_products).
Superseded: sl-crt-2-gate (red, see above), sl-crt-2-plants (older head), sl-crt-2-xtest (stopped, older head).

## Not done / for other stories
* Dynamic runs in the test link libc/libm by path (SONAME tag renamed): Liella's loader does not search sonames at
  start yet (DL-8, plan item 1). When DL-8 lands, drop `by_path` from crt_products.rs.
* liella-gcc still refuses -pg/-static-pie and links -nostartfiles (SL-FLAVOUR may switch it to the plain driver).
* No .note.gnu.property on any start file (crt1.o has none either): one later change for all of them.
* No plant on Mcrt1.S: it has no function label for the `.S` plant rule; its content is unchanged since SL-CRT.

## Next step
None for this story once the jobs are green. Do not redo: the probe above (settled), the record format fix.
