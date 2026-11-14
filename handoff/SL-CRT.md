# SL-CRT: crti.o, crtn.o, grcrt1.o, Mcrt1.o (plan/system-libc.md §6 item 4)

State: branch task/sl-crt, code commit 33bd94b39 (pushed). INCOMPLETE: the run reached its context checkpoint.
Role keke:~/w/n2 (products built with the new objects).

## Done
* `csys/start/crti.S`, `crtn.S`, `Mcrt1.S`; `gcrt1.S` with `-DLIELLA_STATIC_PIE` = grcrt1.o (PC-relative hook).
  `cargo xtask so` builds `liella-{grcrt1,crti,crtn,Mcrt1}.o` (grcrt1 checked like rcrt1); remap PRODUCTS and
  contract PRODUCT-PATH-REMAP list them; tools/deb/build.sh installs gcrt1/grcrt1/crti/crtn/Mcrt1.o in liella6-dev;
  docs/install-packages.md lists them. Decision pending: records/pending/SL-CRT-1.toml (D-NEW, series D-27).
* GCC 13 driver specs (`gcc -dumpspecs`): crti/crtn in every link incl. -shared; gcrt1 for -pg/-p/-profile;
  grcrt1 for those + -static-pie; Mcrt1.o named nowhere (so it is empty but for .note.GNU-stack).
* Test crates/core/tests/crt_products.rs (plain gcc/g++ with `-B<lib>/`, `-Wl,-t` trace proves every start file and
  libc file is Liella's). Job sl-crt-t2: `plain_cc_links_and_runs_hello` (PIE, -no-pie, -static, -static-pie, a
  shared object's own _init/_fini, each output equal to the reference's) and
  `plain_cxx_runs_static_constructors_and_destructors` (dynamic and static) PASS.

## Open (exact next steps)
1. `start_files_follow_the_psabi` fails on its own check: Mcrt1.o has the empty .text/.data/.bss gcc adds (size 0).
   Change the check to "no allocated section of non-zero size" (`size` of the object, or the readelf size column).
2. `plain_cc_pg_writes_gmon_out`: the dynamic `-pg` PIE program (gcrt1.o) writes gmon.out but `gprof -b -q` shows no
   `main`; the reference's PIE -pg build of the same object does (probe 23:2x on keke, `grep -c main` = 2). Not yet
   diagnosed: compare Liella's gmon.out header (lowpc/highpc of a PIE) with the reference's (hist_range in
   gmon_products.rs shows how); the static-pie case (grcrt1.o) has not run yet (the loop stops at the first case).
   This is profiling of PIE programs under Liella's libc.so (gcrt1.o is unchanged by this story), possibly a §28.1
   gap outside the crt objects: if so, report it and keep the dynamic -pg case as a known gap rather than loosen it.
3. Then clippy/static (sl-crt-t2 never reached them), plants `cargo xtask plants handoff/SL-CRT-plants.txt`
   (5 plants), the direct-tier gate, and the start-up checks (brief: `cargo xtask test`, FULL_PROFILE hot_text_products).
* Dynamic runs link libc/libm by path (SONAME tag renamed), as conformance/run.sh does: Liella's loader does not
  search sonames at start yet (DL-8, plan item 1). When DL-8 lands, drop `by_path` from the test.
* Not done (other stories' files): liella-gcc still refuses -pg/-static-pie and links -nostartfiles (SL-FLAVOUR may
  switch it to the plain driver now); no .note.gnu.property on any start file (crt1.o has none either).

Report: handoff/reports/SL-CRT.json.
