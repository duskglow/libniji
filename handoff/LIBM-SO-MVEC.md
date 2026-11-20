# LIBM-SO-MVEC: -lm brings in libmvec.so.1 as needed (GitHub #28)

State: branch task/libm-so-mvec (from origin/next 0e5a87e92); role keke:~/w/n3. Code head 5509fbd30 ("Run the
packaged loader under env inside the watchdog"); later commits are handoff/report only. Status: COMPLETE.

## Done
* `libm.so` in both flavours is a GNU ld script (GNU ld manual, file commands GROUP/AS_NEEDED; our own text):
  `tools/deb/system/libm-so.ld` (libc6-dev) and `tools/deb/coexist/libm-so.ld` (niji6-dev, sysroot paths), both
  `GROUP ( /lib/x86_64-linux-gnu/libm.so.6  AS_NEEDED ( /lib/x86_64-linux-gnu/libmvec.so.1 ) )`, same form as
  libc.so's. Manifest: the `both` link row is replaced by one text row per flavour.
* Tests: `crates/core/tests/deb_manifest.rs` `libm_so_is_a_script_adding_libmvec_as_needed_in_both_flavours`
  (manifest rows + equal commands, no products) and `coexist_lm_brings_in_libmvec_only_for_a_vector_call`
  (coexist debs unpacked, niji-gcc `-O3 -ffast-math tools/deb/smoke/mvec.c -lm` needs libmvec.so.1 and prints
  0.8415 via the packaged loader; `smoke/nomvec.c` `--no-as-needed -lm` needs libm.so.6, not libmvec.so.1;
  skips with a note when the compiler's headers declare no simd sinf). Package checks: check.sh, check-system.sh
  (links + regular-file check), root-system.sh (runs both in the installed root).
* Pending decision `records/pending/LIBM-SO-MVEC-1.toml` (D-NEW, series D-10, amends D-10.124 and D-27.51).
* Docs: docs/install-packages.md, plan/system-libc.md row, tools/deb/system.sh comment.

## Results (keke, final code head 5509fbd30, role tree d78df90e)
* libm-so-mvec-gate3 (clippy, static, affected --base origin/next --run, direct tier): 6 suites ok, 596 passed,
  0 failed, 0 SKIPPED lines; peak 2.2G, rc 0.
* libm-so-mvec-plants2 (handoff/LIBM-SO-MVEC-plants.txt): 4 plants, 4 CAUGHT, 0 WARN.
* libm-so-mvec-pkg2: the fixed coexist `tools/deb/check.sh` on the coexist debs built by libm-so-mvec-pkg
  (products identical: only check.sh and the test changed after that build): `check: PASS (2 packages)`.
* libm-so-mvec-pkg (first commit 87d6ac65b, RAM copy; SUPERSEDED for the coexist half, rc 1 from the env bug fixed
  in 5509fbd30): system flavour, prefix /: check-system.sh PASS (7 packages); root-system.sh pass=21 fail=0 incl.
  "dynamic mvec (-lm) runs in the root: 0.8415 (needs libmvec.so.1 libc.so.6)" and "nomvec (needs libm.so.6
  libc.so.6)"; check-symbols.sh PASS; lint.sh RESULT errors=203 warnings=54 (no snapshot baseline at hand; the
  only libm.so line is `lacks-unversioned-link-to-shared-library example: .../libm.so`, printed for every library
  in libc6, the libc.so script included).
* SUPERSEDED earlier: libm-so-mvec-gate (static red: record key order, fixed), libm-so-mvec-gate2 (killed: tree
  not updated by a BUSY sync), libm-so-mvec-plants (baseline failed: env bug).

## Informational
* `-static -O3 -ffast-math ... -lm` still cannot resolve `_ZGV*` from libniji's libm.a (libmvec.a is separate);
  a black-box probe shows the reference links it statically. Follow-up story (brief: nothing else changes).
* `cargo xtask affected` classes the two `tools/deb/*/libm-so.ld` files as linker input of libc.a ("Image bounded
  set"), which pulls in the start-up layout product tests; a packaging script is not linked into libc.a.
* Clean-room slip (previous leg): a probe command also printed the first lines of the build host's distribution
  `/usr/lib/x86_64-linux-gnu/libm.a`. Nothing was taken from it.
* gfortran is not on keke and the root has no compiler: no gfortran case run.

## Next step
None for this story: the coordinator lands it. Follow-up candidate: static `-lm` with vector calls (libm.a).
