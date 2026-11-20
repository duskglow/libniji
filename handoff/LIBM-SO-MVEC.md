# LIBM-SO-MVEC: -lm brings in libmvec.so.1 as needed (GitHub #28)

State: branch task/libm-so-mvec (from origin/next 0e5a87e92); role keke:~/w/n3. Head: see `git log -1` (the fix
commit "Run the packaged loader under env inside the watchdog").

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

## Results so far (keke)
* libm-so-mvec-pkg (pkgproof: ~/work/libm-so-mvec/pkgproof.sh, RAM copy of the FIRST commit): system flavour
  built with prefix /: check-system.sh PASS (7 packages); root-system.sh RESULT pass=21 fail=0 incl. "dynamic mvec
  (-lm) runs in the root: 0.8415 (needs libmvec.so.1 libc.so.6)" and "nomvec ... (needs libm.so.6 libc.so.6)";
  check-symbols.sh PASS (4440 symbol lines, 13 libraries). lint.sh and the coexist check.sh were still running.
  The coexist check.sh in that run has the env bug below (expected FAIL on the mvec run line only).
* SUPERSEDED: libm-so-mvec-gate (static red: record key order, fixed), libm-so-mvec-gate2 (killed: started on a
  tree the BUSY sync had not updated), libm-so-mvec-plants (baseline failed: the test set LD_LIBRARY_PATH on
  `timeout`, so the host's loader loaded libniji's libc.so.6; fixed by `timeout ... env LD_LIBRARY_PATH=...`).

## Exact next step
1. `~/bin/jobwait libm-so-mvec-pkg` (read its lint.sh lines: lintian counts vs the last snapshot; my change only
   turns one libc6-dev symlink into a text file).
2. Sync (check rc, 4 = BUSY), then start detached: gate `libm-so-mvec-gate3` (clippy, static, affected --base
   origin/next --run: 5 targets, direct tier), plants `libm-so-mvec-plants2` (handoff/LIBM-SO-MVEC-plants.txt,
   --dir /dev/shm/claude/libm-so-mvec/plants-2), and the coexist check.sh rerun on the fixed tree
   (`libm-so-mvec-pkg2`: copy pkgproof.sh with d=.../pkg-2, coexist half only is enough).
3. precheck --jobs, report json, handback-check.

## Informational for the hand-back
* `-static -O3 -ffast-math ... -lm` still cannot resolve `_ZGV*` from libniji's libm.a (libmvec.a is separate);
  a black-box probe shows the reference links it statically. Follow-up story, not this one (brief: nothing else
  changes).
* Clean-room slip to report: a probe command also printed the first lines of the build host's distribution
  `/usr/lib/x86_64-linux-gnu/libm.a` (a reference packaging file). Nothing was taken from it; the scripts follow
  the GNU ld manual and our libc.so script.
* gfortran is not on keke and the root has no compiler: no gfortran case run.
