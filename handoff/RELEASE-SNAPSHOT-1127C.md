# RELEASE-SNAPSHOT-1127C: snapshot validation of af6f8acaf (serial 2026-11-27.3: HEADERS-STAGE2-GAPS-K, -L)

WIP (leg 1 ended by a context refresh at 22:45Z; every job below was started and is running or queued). Procedure =
`handoff/RELEASE-SNAPSHOT-1127B.md`; this note lists only what differs and what the next leg does.

## State
* Branch `task/release-snapshot-1127c` = origin/next af6f8acaf + handoff/ only (this note, the plants list).
  Head under validation: `af6f8acaf2fb8666c108a71f01a76f4a804aa356` "Regenerate train-owned views", commit time
  2026-11-27T18:57:58Z (epoch 1795805878). First-parent log since 27.2 (16e03637e): release-snapshot-1127b
  (28f509c98), headers-stage2-gaps-k (35d10ff8b), regen (6295b2301), configure-diff (ef56c5635), headers-stage2-gaps-l
  (81cce672f), regen (af6f8acaf). Nothing else landed on next at the start.
* Serial **3**: system `2.39+niji20261127.3-1`, coexist `0.0~snapshot20261127.3+af6f8acaf2fb` (27.1 staged not
  published, 27.2 published; no 27.3 in keke `~/release-staging/`). TODO: `dpkg --compare-versions` both against 27.2's.
* Role **keke `~/w/n1`** (Zen 1). Unlike 1127B (role on mei s1), the role jobs (product build, .debs, plants, gate,
  heapnode) run on keke; mei runs its non-role checks on products and .debs relayed from keke (keke ssh's to mei
  directly: `relay.sh`, sha256 checked on arrival by the waiting drivers). tree.tgz = `git archive af6f8acaf`
  (sha256 a46915ed0070...), uploaded to both hosts.

## Job dirs: keke and mei `~/work/RELEASE-SNAPSHOT-1127C/`
Made by `copyrun.sh` = 1127A's `copy.sh OLD=1127B NEW=1127C` with this run's `subst.sed` (head, epoch, serial 3, role
n1, labels 1127b->1127c, comparison side 26.3->27.2: `RELEASE-SNAPSHOT-1126C`->`1127B`, `20261126.3`->`20261127.2`,
`-ref263`->`-ref272` (labels and log dirs only; no data file renamed: the only ref-named data file, y23p/ref241.txt,
is the same in both), products labels p229->p231 (27.2) and p231->p232 (head)), links restored, logs-ystage2-stock
copied (23 files, as 1127B's INFORMATIONAL asks). Fixed after the copy (new files written, never sed -i):
* `cmp13.sh`, `tails.sh` (mei, via `cmpfix.sed`): old side = `release-snapshot-1127b-<job>-1` (the copy still said
  1126c), mask also covers 16e03637e and `-ref27x`.
* `run-hand.sh` (mei): gi.sh and g14.sh run under bash (1127B's leg-1 defect).
New scripts: `run-role.sh` (keke: 1127B's mei run-role with role n1, RAM dir /dev/shm), `relay.sh` (keke),
`run-keke3.sh` (keke: g14m, units2, addstock vs 27.2's libc6-dev), `g14m.sh` (step 6: gm2-libs-ch/wrapc.c as C++,
GAPS-L's units3 stub method, on the packaged libc6-dev; plus a bare `<math.h>` C99-macro probe in C++, C++98..20),
`run-g14b.sh` + `g14b.sh` + `g14b/b.sh` (step 6 part 2: trixie chroot, system debs swapped in, upstream gcc 14.2.0
from Debian's orig tarball, Debian's `gcc-14 -v` configure line with languages c,c++,m2, no multilib/offload/LTO
config, dpkg-buildflags, `make -j32 bootstrap`, 140 min cap), `run-cdiff.sh` (mei: CONFIGURE-DIFF tool from the
tree, system debs: gcc-14 alone = `cdiffg14-1`, then the 438-package `~/work/CONFIGURE-DIFF/stage2.list` -j16 =
`cdiff-1`, run dirs `/mnt/zram/claude/release-snapshot-1127c/`). Not run, as in 1127B: run-ref.sh.

## Drivers started (outputs `<driver>.out` in the job dir); labels `release-snapshot-1127c-<x>`
* keke: run-role (smoke-n1-1, debs-1, plants-1, gate-1, heapnode-1), run-keke2 (smoke-keke-1, startup-keke-1,
  kstart-keke-1, startup215-keke-1, rstart-keke-1, gaimass-1, units-1, pkgdiff-1, stage-1, install-1, hdrdiff-1),
  relay, run-keke3 (g14m-1, units2-1, addstock-1), run-g14b (g14b-1).
* mei: run-mei, run-hdr, run-hand, run-extra (the 1127B mei jobs, same names, `-ref272` for the 27.2-debs reruns),
  run-cdiff (cdiffg14-1, cdiff-1).

## Exact next step
1. Wait (`~/bin/jobwait`, one call per host) for the drivers; read results with grep/tail only.
2. mei, once run-mei/run-hdr/run-hand/run-extra are done: `jobrun release-snapshot-1127c-cmp13a-1 check -- sh
   ~/work/RELEASE-SNAPSHOT-1127C/cmp13.sh <1127B's cmp13-a.list>`; cmp13b-1 with `../RELEASE-SNAPSHOT-1127B/cmp13-b.list`
   with `-ref263` -> `-ref272`; tails-1 on the job list 1127B's tails-1 log names (`grep -o 'tails: [A-Z]* [^:]*'`).
3. Judge per brief step against 1127B's numbers (its handoff "Steps"). New expectations: yb11 rows incl. GAPS-K/-L's
   rows 0 DIFF; units2 llvm-19 `sanitizer_platform_limits_posix.cpp` now niji=0 (the SIOCAIPX* stop gone) or the next
   stop; g14m RESULT diff=0, failing 0; cdiffg14: `glibcxx_cv_c99_math_cxx98`/`_cxx11` yes on the libniji side;
   cdiff-1 vs `handoff/configure-diff/results-20261127.2/diff.tsv` (cols pkg class probe stock libniji): only the L1
   (gcc-14 c99 math rows) and L10 (google-perftools `HAVE___ENVIRON`) rows gone, any new row a finding; `diff.py
   --sides stockA,stockB` control; CONFIGURE-DIFF needed a retry list for packages whose chroot never started.
   1127B's osockaddr RULING: GAPS-K says timed.h no longer includes sys/socket.h: addstock should no longer list it.
4. Copy this note's form to the final report (1127B's layout), JOBS = every label above that ran on the final head,
   precheck on keke n1 (`cargo xtask precheck --jobs <labels> --head <sha>`), report json, handback-check.
