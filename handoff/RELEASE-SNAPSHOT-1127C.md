# RELEASE-SNAPSHOT-1127C: snapshot validation of af6f8acaf (serial 2026-11-27.3: HEADERS-STAGE2-GAPS-K, -L)

COMPLETE: snapshot-ready at af6f8acaf2fb8666c108a71f01a76f4a804aa356. Steps 4-6 (blocking) pass: GAPS-K/-L's 218
header_check units ok on the packaged headers (system, coexist, tree), yb11 900/900 SAME, dupval 0 (both lists),
fixincludes 0 rewritten, libstdc++-v3's configure says yes to C99 `<math.h>` for C++98 and the C++11 generic macros
on libniji (gcc-14's rows 10 -> 2, both the known L2 `strerror_r` row), wrapc.c compiles as C++. Step 7: llvm-19's
sanitizer_platform_limits_posix.cpp compiles (clang and g++). Step 8 as 27.2. No new finding needs a ruling.

## State
* Branch `task/release-snapshot-1127c` = origin/next af6f8acaf + handoff/ only (plants list, this note, the json,
  the configure-diff two-side mode c99118740). origin/next still af6f8acaf at 01:06Z on 11-28.
* Head under validation `af6f8acaf2fb8666c108a71f01a76f4a804aa356` "Regenerate train-owned views", commit
  2026-11-27T18:57:58Z (epoch 1795805878). First-parent log since 27.2 (16e03637e): release-snapshot-1127b
  (28f509c98), headers-stage2-gaps-k (35d10ff8b), regen (6295b2301), configure-diff (ef56c5635),
  headers-stage2-gaps-l (81cce672f), regen (af6f8acaf).
* Serial **3**: system `2.39+niji20261127.3-1`, coexist `0.0~snapshot20261127.3+af6f8acaf2fb`; both sort above
  27.2's (`dpkg --compare-versions`). libc.so.6 sha256 `1d9e2e6e25c4a536...`, ld.so `20a09f9f187602e7...` = 27.2's
  (a headers-only change).
* **Staged** on keke `~/release-staging/snapshot-20261127.3/`: 9 debs, SHA256SUMS (9 OK, equal to the built debs'
  sums), MANIFEST. Not published, nothing to GitHub, no tag.
* Role keke `~/w/n1` (Zen 1); mei (Skylake-SP) ran its checks on relayed products and debs. Job scripts and logs:
  keke and mei `~/work/RELEASE-SNAPSHOT-1127C/` (1127B's, head/epoch/serial/labels changed; new `hc.sh` on keke,
  `cdpart.sh`, `run-cdiff2.sh` on mei).

## Steps (jobs in brackets; keke unless "mei")
* 4 header rows: [mei yb11-1] same=900 diff=0 (27.2's fixed rows; it has no GAPS-K/-L rows), so [hc-2]: the 218
  header_check units GAPS-K (142) and GAPS-L (76) added, every C and C++ configuration, on the PACKAGED headers:
  system 218 ok, coexist 218 ok, tree control 218 ok, per-unit lists equal. [mei yb9-1] diff=20 = the recorded
  sched_attr family vs 2.39; [mei yb10-1] diff=6 = ftp-tables definition order (as 27.2); [mei cxxp-1] = 27.2's log;
  yb6/yb8/hp2/hp3 0 DIFF, hp4 14 = syslog names (as 27.2). [mei dupval-1] 6583 same, diff 0; [mei dupval-2] 6517
  same, diff 0. [addstock-1] added vs 27.2's libc6-dev 364, not in stock 329, user-namespace-not-in-stock 0,
  osockaddr 0: 1127B's RULING is resolved by GAPS-K ([hdrdiff-1]: protocols/timed.h's AF_* and sys/socket.h names
  REMOVED; hdrdiff rc 1 = removals present, as 27.2's run). Added names include `__environ` (unistd.h, L10),
  `__iscanonicall`, the `__NIJI_D_*` guards.
* 5 fixincludes [mei gi-1]: 0 rewritten (installed 242, merged 331); log = 27.2's gi-2 (masked diff empty).
* 6 gcc-14, blocking part: [mei cdiffg14-1] libstdc++-v3's configure, `g++-14 -nostdinc++`, trixie chroot, four
  sides: "ISO C99 support in <math.h> for C++98... yes" and "... generic macro support in <math.h> for C++11... yes"
  on libniji, stock 2.41 and stock 2.39. Differing 2 = HAVE_STRERROR_R / _GLIBCXX_HAVE_STRERROR_R (the known L2 row,
  libniji-vs-both); 27.2's 10 gcc-14 rows -> 2, every C99 math row gone. [g14m-1] wrapc.c as C++ and a bare
  `<math.h>` probe, C++98..20: same=10 diff=0, libniji failing 0. [mei g14-1] = 27.2's g14-2 (libstdc++ 16 SAME,
  libgomp 50 SAME).
  Non-blocking part, the bootstrap with Debian's flags: [g14b-1] stopped after 81 s in stage-1 libcpp on
  `-Werror=format-security`; [g14bstock-1], the same job without the libc swap (stock 2.41), stops identically:
  the driver's flags, not libniji. [g14b-2] (DEB_BUILD_MAINT_OPTIONS=hardening=-format) stopped after 84 s, rc 2:
  its first real error is NOT READ: keke stopped resolving at about 00:05Z (flag #8824) before leg 3 could open
  `logs-g14b2/out/make-errors.txt`. Known only (leg 2): the run had Debian's format hardening off; "Python3 is
  missing" and the pod errors in its log are ignored ones; where it stopped is not known. Non-blocking; next reader: grep `error:` in that file
  and b.txt around Makefile:25068; a stock control is g14bs.sh with the same DEB_BUILD_MAINT_OPTIONS.
* 7 llvm-19 [units2-1]: sanitizer_platform_limits_posix.cpp clang 0/0 and g++ 0/0 failing (27.2: libniji 1, on
  SIOCAIPX*); units2 4/4. No next stop in that unit.
* 8 [mei ybuild-1] util-linux 43 = 43 debs, inetutils 24 = 24, npth, gnupg2, libev, keyutils PASS. [units-1]
  15/15; [mei gi-1] units ok=89 fail=3 (as 27.2); `uuid_time64@@UUID_2.40` yes. [mei cmp13a-1] 19 jobs, 0
  differing (roots 4 = job dir path, as 27.2); [mei cmp13b-1] 46 jobs, 0 differing (perl ystage2-niji, yapt, ypk
  included). [mei tails-1] 22 SAME; DIFF only yapt (seconds), kstart/rstart-mei (kernel side only, user side
  equal), sig32 (product label), ystage2cmp bad=1 (= 27.2's valid run). fp SAME.
* 9 configure diff (non-blocking) [mei cdiff-1], two stock sides (coordinator note 22:52Z): stock 2.41 (trixie)
  and stock 2.39 (Ubuntu noble `2.39-0ubuntu8.12` libc6, libc-bin, libc6-dev, libc-dev-bin; sha256 in mei
  `$W/stock239/SHA256SUMS`); tool = this branch's handoff/configure-diff (c99118740, `-s`). Still running at
  hand-back (438 packages, -j 20, started 23:27Z). Partial looks on copies of the finished packages' out/ dirs
  (the live run untouched):
  [mei cdpart-1] 41 packages: 5 differing rows, all libniji-vs-both (apr and bash `d_fileno`, L6), = 27.2's rows for
  those packages; control 0. [mei cdpart-2] 266 packages, 95177 probes: noisy 2, differing 69;
  **TWO-SIDE 2.41-only=2 libniji-vs-both=67 no-s239=0 as241=1**; control (stockA vs stockB) 2 = the 2 noisy rows.
  27.2's rows for the same 266 packages: 78; this run's: 69 = 27.2's minus 9, **no new row**. The 9 gone: gcc-14's 8
  C99 math rows (`glibcxx_cv_c99_math_cxx98/cxx11`, `_GLIBCXX98/11_USE_C99_MATH`, `_GLIBCXX_USE_C99` in config.h
  and c++config.h: L1) and google-perftools' `HAVE___ENVIRON` (L10). The 2.41-only rows: libsigsegv `sv_cv_host`
  (the triplet carries the libc version) and libsodium `ax_cv_check_cCATCHABLE_ABRT` (libniji answers yes, as
  2.39; the repeated-abort behaviour GAPS-M takes per the owner's #8823 ruling, not in this head). as241 1 =
  diffutils `am_cv_filesystem_timestamp_resolution` (a timing measurement, 0.2 vs 0.1). The remaining 60 rows are
  the CONFIGURE-DIFF report's (L2-L9, L11-L15), unchanged.
  The full result is the coordinator's to collect: mei `/mnt/zram/claude/release-snapshot-1127c/cdiff-1/`
  (`summary.txt` last line `TWO-SIDE ...`, `diff.tsv`; `diff.py <run> --sides stockA,stockB` for the control).
* 10 [kcmp-1] keke jobs vs 27.2: start-up user side equal (Zen 1 pie0 156,365 instr, 52 faults; Skylake-SP in
  tails), only max/kernel jitter; gaimass bad=0; pkgdiff small size deltas, no package missing; install fails=0,
  system debs different 0; [heapnode-1] only #21 fails (as 27.2); [mei lint-1] = 27.2.
* Product and role: [smoke-n1-1] SMOKE bad=0, [debs-1] DEBS bad=0, [gate-1] clippy -D warnings all-features,
  xtask clippy, static, records check, affected: all rc 0. [plants-1] 6 CAUGHT, 0 SURVIVED (WARN-DEV-SHM only).
  Precheck: run by `ops handback-check` on keke n1 after sync_role (both reach keke; this session's ssh does not,
  below): first run red on head only (role not synced), after the sync handback-check ok. The branch's commits since
  gate-1 and plants-1 are handoff/ only (no code); gate-1 and plants-1 ran on af6f8acaf.

## INFORMATIONAL
* dupval-2 6532 -> 6517 same: the 5 TTYDEF_* rows left dup-listed.tsv with GAPS-K (defined once now); GAPS-K's
  handoff names only TTYDEF_SPEED.
* Leg 2's two ssh calls that chained a `cd` before a detached start hung the local ssh (killed by the harness);
  the remote jobs were setsid and ran on. Harness defect, libniji untouched.
* **keke does not resolve from this session** ("ssh: Could not resolve hostname keke", `getent hosts keke` and
  `keke.mgmt.internal` empty; mei resolves) from about 00:05Z to at least 01:16Z on 11-28 (flag #8824); the ops
  tools (sync_role, handback-check's precheck) still reached it. g14b-2's error was not read in leg 3. The staged
  snapshot was checked in leg 2 (9 OK).
* Evidence jobs with rc != 0 by design (not in JOBS, as 1127B): keke g14b-2 and g14bstock-1 (the bootstrap stops),
  hdrdiff-1 (removals present), heapnode-1 (#21 only); mei yb9-1 (sched_attr), yb10-1 (ftp-tables order).
* SUPERSEDED: hc-1 (multiarch include dir not merged: harness), g14b-1 (driver flags: `-Werror=format-security`,
  identical on stock).

## Exact next step
Coordinator: cut snapshot 2026-11-27.3 from keke `~/release-staging/snapshot-20261127.3/`; tell Yohane to retry
gcc-14 (libstdc++'s C99 math configure says yes now). Collect cdiff-1 when it ends (above).
