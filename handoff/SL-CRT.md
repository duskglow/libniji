# SL-CRT: crti.o, crtn.o, grcrt1.o, Mcrt1.o (plan/system-libc.md §6 item 4)

State: branch task/sl-crt-3 (continues task/sl-crt-2 and task/sl-crt), merged with origin/next 8261e29b2.
Role keke:~/w/n1 (Zen 1). Decision D-27.52 (records/decisions/D-27/D-27.52.toml).

## Done
* SL-CRT (33bd94b39): `csys/start/crti.S`, `crtn.S`, `Mcrt1.S`; `gcrt1.S` with `-DLIELLA_STATIC_PIE` = grcrt1.o.
  `cargo xtask so` builds `liella-{grcrt1,crti,crtn,Mcrt1}.o`; remap PRODUCTS and contract PRODUCT-PATH-REMAP list
  them; tools/deb/build.sh installs gcrt1/grcrt1/crti/crtn/Mcrt1.o in liella6-dev; docs/install-packages.md lists them.
* GCC 13 driver specs (`gcc -dumpspecs`): crti/crtn in every link incl. -shared; gcrt1 for -pg/-p/-profile;
  grcrt1 for those + -static-pie; Mcrt1.o named nowhere (empty but for .note.GNU-stack).
* SL-CRT-2 (0702ed168, 5d13765a6): record format fix; Mcrt1.o check "no allocated section of non-zero size";
  `-pg` PIE gap fixed (gmon.out holds link-time pcs: `process::executable_bias()` subtracted in s28_profiling.rs;
  probe of the reference settled it: low pc = `__executable_start`, dynamic and `-static-pie`). Plants
  handoff/SL-CRT-plants.txt (6 CAUGHT on 5d13765a6).
* SL-CRT-3 (this run):
  - CI could not merge next: tools/deb/build.sh conflicted (SL-CRT's start-file loop vs next's longer archive list:
    libBrokenLocale.a, libg.a, libmcheck.a, libpthread_nonshared.a). Merge 478d5fb12 keeps both. The other files
    changed on both sides (process.rs, so.rs, install-packages.md) merged without overlap.
  - Real interaction: next brought the system flavour (tools/deb/system.sh, check-system.sh: its own libc6-dev),
    which said "no crti.o/crtn.o yet (SL-CRT)". 52a4b75c3: the system libc6-dev ships gcrt1/grcrt1/crti/crtn/Mcrt1.o;
    check-system.sh accepts crti.o/crtn.o in the layout and links its dynamic and static hello with the packaged
    crti.o/crtn.o in the driver's order; control text and docs/install-packages.md updated.
  - Plants handoff/SL-CRT-3-plants.txt (2, on system.sh; their command uses the prefix-/ products of job
    sl-crt-3-pkg, in /dev/shm/claude/SL-CRT-3/pkg-1 on keke).

## Jobs (keke) on the final head
All rc 0, on the code of 0c1dd855f (later commits change handoff/ only):
* sl-crt-3-lint: `cargo xtask clippy`, full `cargo clippy --workspace --all-targets --all-features -D warnings`, static.
* sl-crt-3-pkg: both flavours rebuilt from a RAM copy (build.sh's own env), check.sh PASS (2 packages),
  check-system.sh PASS (6 packages); liella6-dev and libc6-dev each list the 8 start files.
* sl-crt-3-gate: `cargo xtask affected --run` (base origin/next from the sync). The change set includes
  xtask/src/so.rs, so it escalates to FULL and runs `cargo xtask test` itself: 6621 + 5235 passed, 0 failed;
  conformance 137/137; peak 13G. One SKIPPED line (LIELLA_REGEX_CORPUS unset; unrelated).
* sl-crt-3-hotprof: `LIELLA_FULL_PROFILE=1 cargo xtask so` + hot_text_products (26 passed), in a RAM copy so it
  did not build in the role while the gate did.
* sl-crt-3-plants: 2 plants, 2 CAUGHT, no WARN lines.
* sl-crt-3-lint0: step 0 on the inherited head 30f1c0bda (green).
No separate `cargo xtask test` job: the gate ran it (a queued sl-crt-3-xtest was stopped before it started).

## Not done / for other stories
* Dynamic runs in the test link libc/libm by path (SONAME tag renamed): Liella's loader does not search sonames at
  start yet (DL-8, plan item 1). When DL-8 lands, drop `by_path` from crt_products.rs.
* liella-gcc still refuses -pg/-static-pie and links -nostartfiles (SL-FLAVOUR may switch it to the plain driver).
* tools/deb/root-system.sh (the installed-root proof) still links its hello with -nostartfiles and without
  crti/crtn; check.sh (coexisting flavour) does not check that liella6-dev holds the start files (crt_products checks
  the products, not the package). Both are small follow-ups, not needed for this story.
* plan/system-libc.md line 71 still says crti/crtn/grcrt1/Mcrt1 are "missing" (planning view; left to the planner).
* No .note.gnu.property on any start file (crt1.o has none either): one later change for all of them.
* No plant on Mcrt1.S: it has no function label for the `.S` plant rule; its content is unchanged since SL-CRT.

## Next step
None for this story once the jobs are green. Do not redo: the merge, the probe above (settled), the record format fix.
