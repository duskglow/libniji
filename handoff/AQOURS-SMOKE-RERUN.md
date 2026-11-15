# AQOURS-SMOKE-RERUN: the smoke run again on the complete loader (2026-11-14, AQOURS-SMOKE-RERUN-2)

**State:** branch `task/aqours-smoke-rerun-2` (continues `task/aqours-smoke-rerun`, cut from origin/next
`f53c74611`), role mei `~/w/s1`. Harness and records only; no Liella code changed. The full report is
`validation/smoke/RESULTS.md`, section "Second run".

**Snapshot verdict:** blocked by one (a) finding, F13 (`cxx_dlclose`: a pending `thread_local` destructor of
a closed plug-in; SIGSEGV after `main` returns on Liella, spec `s1_2_c_6`), which is story DL-TLS-DTOR-PIN
(running). Nothing else blocks: F3 is the recorded, ruled difference (#5480); every other difference is (b)
with its story or (c)/host. Once DL-TLS-DTOR-PIN lands, rerun `cxx_dlclose` on the three hosts; if it
matches the reference, the snapshot can proceed.

## Done
* Full runs (`aqours-smoke-rerun-full-{keke,mei,shiki}`, harness 390ebd7, products next f53c74611): every
  program on all three hosts; the hosts agree except F14 (fixed) and F18 (host, both builds).
* Harness fixes, then make, gawk_ext, python3, python3_shared again on all three hosts on the same products
  (`aqours-smoke-rerun-2-part-<host>`, then `-pass2-<host>` for make and gawk_ext after the second fix):
  F14 python's module set no longer depends on host packages (`python3_nohostlibs`); F15 gawk_ext builds
  `extension/` and checks from `test/` (557 / 557 on both builds); F16 make 136 / 1 on both (liella-cc.env;
  make's failure list holds the test names only).
* F3 probe (`repro/setlocale_errno.c`, two calls, all hosts): a repeated failing `setlocale` keeps `errno`
  on the reference and sets ENOENT on Liella: `docs/differences-from-glibc.md` row, ruling #5480. F2 probe
  (`aqours-smoke-rerun-2-probe2-mei`): fixed.
* Plants `handoff/AQOURS-SMOKE-RERUN-plants.txt` (7, harness trust anchor, kind `same`, loader soname,
  liella-cc.env) rerun on the final head. The recipe fixes (F14 to F16) are proven by the before/after runs,
  not by plants (a plant would need a Debian build and the mirror per mutant).

## Not to redo
The full runs; the F3 classification; the module-set and gawk SUBDIRS causes.

## Next step (exact)
After DL-TLS-DTOR-PIN lands: build release products of next, stage them and this harness on keke, mei and
shiki, and run `smoke.sh ... cxx_dlclose` (with `--mirror http://miko.mgmt.internal/debian --keyring
~/keyrings/debian-archive-keyring.gpg`); its output must equal the reference's. Then add one line to
RESULTS.md (F13 fixed) and the snapshot can proceed.

## Plan rows / records to read
`docs/differences-from-glibc.md` ("errno after setlocale returns NULL"), DL-TLS-DTOR-PIN's brief, DL-8,
DL-21, DL-23.
