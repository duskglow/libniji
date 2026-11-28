# RELEASE-SNAPSHOT-1127C: snapshot validation of af6f8acaf (serial 2026-11-27.3: HEADERS-STAGE2-GAPS-K, -L)

WIP (leg 2 ended by a context refresh at about 00:05Z on 11-28). Procedure = `handoff/RELEASE-SNAPSHOT-1127B.md`.
Every blocking step is judged below and passes; open: cdiff-1 (step 9, non-blocking, slow), g14b-2's stop (step 6
part 2, non-blocking), then the report, precheck and hand-back.

## State
* Branch `task/release-snapshot-1127c` = origin/next af6f8acaf + handoff/ only (this note, the plants list, the
  configure-diff two-side mode c99118740). origin/next still af6f8acaf at 22:50Z.
* Head under validation `af6f8acaf2fb8666c108a71f01a76f4a804aa356`, commit 2026-11-27T18:57:58Z (epoch 1795805878).
  First-parent log since 27.2 (16e03637e): release-snapshot-1127b (28f509c98), headers-stage2-gaps-k (35d10ff8b),
  regen (6295b2301), configure-diff (ef56c5635), headers-stage2-gaps-l (81cce672f), regen (af6f8acaf).
* Serial 3: `2.39+niji20261127.3-1` and `0.0~snapshot20261127.3+af6f8acaf2fb`; `dpkg --compare-versions` gt 27.2's
  both (checked). libc.so.6 sha256 `1d9e2e6e25c4a536...`, ld.so `20a09f9f187602e7...` = 27.2's (headers-only change).
* Staged keke `~/release-staging/snapshot-20261127.3/`: 9 debs + SHA256SUMS + MANIFEST, 9 OK, sums = built debs.
* Role keke `~/w/n1` (Zen 1); mei (Skylake-SP) ran its checks on relayed products/debs. Job dirs keke and mei
  `~/work/RELEASE-SNAPSHOT-1127C/`.

## Steps (labels `release-snapshot-1127c-<x>`; keke unless "mei")
* 4 header rows: [mei yb11-1] same=900 diff=0 (27.2's fixed rows: it has NO GAPS-K/-L rows) -> added [hc-2]: the
  218 header_check units GAPS-K (142) and GAPS-L (76) added, all C and C++ configurations, on the PACKAGED headers:
  system 218 ok, coexist 218 ok, tree control 218 ok, per-unit lists equal (hc.sh; hc-1 superseded: multiarch dir
  not merged, harness). [mei yb9-1] diff=20 = recorded sched_attr; [mei yb10-1] diff=6 ftp-tables order;
  [mei cxxp-1] identical to 27.2's log; yb6/yb8/hp2/hp3 0 DIFF, hp4 14 = syslog names (as 27.2).
  [mei dupval-1] 6583 same diff 0; [mei dupval-2] 6517 same diff 0 (27.2: 6532; the 5 TTYDEF_* rows left
  dup-listed.tsv with GAPS-K: defined once now; GAPS-K's handoff names only TTYDEF_SPEED: INFORMATIONAL).
  [addstock-1] added vs 27.2's libc6-dev 364, not in stock 329, user-namespace-not-in-stock 0; osockaddr 0
  (1127B's RULING resolved: [hdrdiff-1] REMOVED protocols/timed.h AF_* etc., the sys/socket.h names; hdrdiff rc 1 =
  removals present, as 27.2's run). Added: `__environ` (unistd.h, L10), `__iscanonicall`, __NIJI_D_* guards.
* 5 fixincludes [mei gi-1]: 0 rewritten (installed 242, merged 331); gi-1 log = 27.2's gi-2 (masked diff empty).
* 6 gcc-14 blocking part: [mei cdiffg14-1] libstdc++-v3 configure, `g++-14 -nostdinc++`, trixie chroot, four sides:
  "ISO C99 support in <math.h> for C++98... yes", "generic macro support ... C++11... yes" on libniji (and stock
  2.41, stock 2.39). Differing 2 = HAVE_STRERROR_R / _GLIBCXX_HAVE_STRERROR_R (known L2 row, libniji-vs-both);
  27.2's 10 gcc-14 rows -> 2 (all c99 math rows gone). [g14m-1] wrapc.c as C++ + bare math.h probe C++98..20:
  same=10 diff=0, niji failing 0. [mei g14-1] = 27.2's g14-2 (libstdc++ 16 SAME, libgomp 50 SAME).
  Part 2 (non-blocking): [g14b-1] bootstrap stopped after 81 s in stage-1 libcpp on -Werror=format-security;
  [g14bstock-1] the same job without the libc swap (stock 2.41) stops identically -> driver flags, not libniji.
  [g14b-2] rerun with DEB_BUILD_MAINT_OPTIONS=hardening=-format: stopped after 84 s, rc 2 (Makefile:25068
  bootstrap); first real error NOT YET READ ("Python3 is missing" / pod errors are ignored ones).
* 7 llvm-19 [units2-1]: sanitizer_platform_limits_posix.cpp clang 0/0 and g++ 0/0 (27.2: niji=1 on SIOCAIPX*);
  units2 4/4.
* 8 [mei ybuild-1] util-linux 43=43, inetutils 24=24, npth, gnupg2, libev, keyutils PASS. [units-1] 15/15,
  [mei gi-1] units ok=89 fail=3 (as 27.2), uuid_time64@@UUID_2.40 yes. [mei cmp13a-1] 19 jobs 0 differing, roots 4 =
  job dir path (as 27.2); [mei cmp13b-1] 46 jobs 0 differing (perl ystage2-niji, yapt, ypk incl.).
  [mei tails-1] 22 SAME; DIFF: yapt (seconds), kstart/rstart-mei (kernel side only, user equal), sig32 (product
  label), ystage2cmp bad=1 vs 27.2's superseded -1 (bad=4; 27.2's valid -2 had bad=1). fp now SAME (6 -> 6).
* 9 configure diff [mei cdiff-1]: running, two stock sides (coordinator note 22:52Z), tool = branch's c99118740
  unpacked in mei `$W/cdtool`, stock 2.39 = Ubuntu noble 2.39-0ubuntu8.12 debs in `$W/stock239` (SHA256SUMS).
  28 of 438 packages done at 00:02Z (started 23:27Z, -j20): will NOT finish before hand-back.
* 10 [kcmp-1] keke jobs vs 27.2: start-up user side equal (Zen 1 pie0 156,365 instr/52 faults; Skylake-SP via
  tails), only max/kernel jitter; gaimass bad=0; pkgdiff small size deltas, no package missing; install fails=0,
  system debs different 0; [heapnode-1] only #21 fails (as 27.2); [mei lint-1] = 27.2 (cmp13a).
* Product/role: [smoke-n1-1] SMOKE bad=0, [debs-1] DEBS bad=0, [gate-1] clippy -D warnings all-features, xtask
  clippy, static, records check, affected: all RC 0. [plants-1] 6 CAUGHT, 0 SURVIVED, no WARN-RC-ONLY/NO-MSG
  (WARN-DEV-SHM only, informational).

## Exact next step (leg 3)
1. g14b-2: read the first real error (`~/work/RELEASE-SNAPSHOT-1127C/logs-g14b2/out/make-errors.txt`, `b.txt`
   around the Makefile:25068 line; grep `error:`), judge libniji-only or not (a stock control = g14bs.sh with the
   same DEB_BUILD_MAINT_OPTIONS); report as far as it got. No third attempt.
2. cdiff-1 at about 01:20Z (partial; cdiff.sh runs extract.py and diff.py only at its end): as a jobrun check job
   on mei, `python3 -I $W/cdtool/configure-diff/extract.py <run> <the packages whose out/status.txt has "cdiff:
   done">` then `diff.py <run>` (packages still running show NODATA; the final run rewrites probes/ and diff.tsv
   anyway). Compare its diff.tsv with `handoff/configure-diff/results-20261127.2/diff.tsv` for the same packages
   (L1 and L10 rows gone; any new row a finding), report the TWO-SIDE line (libniji-vs-both / 2.41-only). Leave
   cdiff-1 running, named in JOBS as a long job; say the full result is the coordinator's to collect.
3. Report (1127B's layout, first line `COMPLETE: snapshot-ready at af6f8acaf...` if nothing new), json
   (`handoff/reports/RELEASE-SNAPSHOT-1127C.json`), sync role, precheck on keke n1 with the JOBS labels, push,
   handback-check. JOBS: keke smoke-n1-1, debs-1, gate-1, plants-1, heapnode-1, hc-2, g14m-1, units-1, units2-1,
   addstock-1, hdrdiff-1, kcmp-1, stage-1, install-1, pkgdiff-1, gaimass-1, startup*-keke-1, g14b-2,
   g14bstock-1; mei yb11-1, yb9-1, yb10-1, cxxp-1, dupval-1/-2, gi-1, g14-1, ybuild-1, cmp13a-1, cmp13b-1,
   tails-1, cdiffg14-1, cdiff-1 (running). SUPERSEDED: hc-1 (multiarch not merged), g14b-1 (driver flags).
4. INFORMATIONAL for the report: dupval-2 TTYDEF rows; leg 2's two ssh calls that chained a cd before a detached
   start hung the local ssh (killed by the harness; the remote jobs were setsid and ran on).
