WIP (leg 1 of RELEASE-SNAPSHOT-1121C; refresh): every job started, none judged yet.

# RELEASE-SNAPSHOT-1121C: snapshot validation of e4d7011e6 (GitHub #27 start files, #13 valgrind, DYN-START-RING-2)

**State.** Head to validate: origin/next `e4d7011e669f79de280dc83207d08eb92dd3ce5f` at the start (10:12 UTC; "Merge
task/release-snapshot-1121b into next"; commit time 2026-11-21T10:07:39Z, epoch 1795255659); contains 8cbcc715 and
879c45b3. Merges since 21.3 (`git log --oneline --first-parent fe0899b79..e4d7011e6`): wl-nightly-1121 (43987d4dc),
crt-start-files-3 (8cbcc7151), dyn-start-ring-2 (9c0e5d927), valgrind-ldso-symbols-2 (879c45b3d), train views
(57970ef41), release-snapshot-1121b (e4d7011e6, handoff only). Serial 4: `2.39+niji20261121.4-1`, stage keke
`~/release-staging/snapshot-20261121.4/`, tag snapshot-2026-11-21.4 (not made). Branch `task/release-snapshot-1121c`
= head + handoff/ only. Role mei `~/w/s4` (Skylake-SP); keke = Zen 1. 21.3 = fe0899b79 (RELEASE-SNAPSHOT-1121B).

**Scripts** (mei and keke `~/work/RELEASE-SNAPSHOT-1121C/`; local scratch `~/w/.scratch/RELEASE-SNAPSHOT-1121C/`,
`mirror/` = the keke copy): 1121B's set by copy.sh + subst.sed (names, head, epoch, serial, labels; every input dir
incl. y23p/ dlc/); file lists equal but outputs. tree.tgz = `git archive e4d7011e6` (8751 files = ls-tree; sha
checked on both). Plants file handoff/RELEASE-SNAPSHOT-1121C-plants.txt (1121B's; each pattern present once).
New: **y27.sh** (#27: crtsum.sh start-file facts of D-X.144 and crt_maps.c linked 4 ways, stock vs swapped; util-linux
2.41.5 lsfd + test_mkfds built on stock from trixie's orig tarball, relinked after the swap, tests/run.sh
lsfd/column-xmode), **y13.sh** (#13: memcheck on /bin/true, bad.c, cxx.cc with libniji's whole libc6-dbg, loader
debug file only, none; readelf of the loader's debug file), **sig32.sh** (ruling #7075: raw 32 before/after the first
thread; glibc 2.39, p213, p214), made by mknew3.sh from y21.sh; run-new3.sh sequences them (replaces run-new2.sh).
The lsfd recipe was tried on the host first (ulproto-1: 18/18 sub-tests OK on glibc 2.39; not a named job).

## Jobs started (all labels prefixed release-snapshot-1121c-)

* mei run-mei.sh: smoke-mei-1 (`SMOKE: RESULT bad=0`), gate-1, startup-mei-1, kstart-mei-1, debs-1, then plants-1,
  y24-1, y28-1, y6-1, y25s-1 and chain-mei-debs.sh (roots-1, lint-1, yohane-1, many-1, dyn22-1, y10-1, y19-1, y26-1,
  y26t-1, y23-1, c19-1, y25-1). mei run-new.sh: rstart-mei-1, y21-1, yres-1. mei run-new3.sh: y27-ref213-1,
  y13-ref213-1 (21.3 side, 1121B's debs), sig32-1, then y23p-1, dlc-1, y27-1, y13-1. Outputs: `run-*.out`.
* keke chain-keke-pre.sh (products.tgz copied, sha checked): smoke-keke-1, startup-keke-1, kstart-keke-1.

## Exact next steps

1. keke: after kstart-keke-1, `~/bin/jobrun release-snapshot-1121c-rstart-keke-1 test -- sh
   ~/work/RELEASE-SNAPSHOT-1121C/rstart.sh /dev/shm/claude/release-snapshot-1121c/rstart-1` (detached; 1121B had it).
2. When mei debs-1 has written debs.sha: copy mei `debs/system/*.deb debs/coexist/*.deb` + debs.sha to keke's
   `~/work/RELEASE-SNAPSHOT-1121C/debs/`, then start keke chain-keke-debs.sh (pkgdiff-1, stage-1, install-1).
3. y27/y13 are new: their ref213 runs are their first real runs; a script bug = fix, rerun under -2.
4. Judge: cmp4.sh (`sh cmp4.sh` + `sh cmp4.sh y23p dlc`), kx.sh mei / keke (startup tables vs 21.3 and glibc),
   y27/y13/sig32 logs (head vs ref213), docs check (1121B: doccheck.py from 1120F's scratch), then the report.
5. precheck on the synced final head (jobrun label precheck-1), report json, handback-check.
