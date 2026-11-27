# RELEASE-SNAPSHOT-1127A: snapshot validation of dbb8c346f (serial 2026-11-27.1: HEADERS-STAGE2-GAPS-H, -I)

**State (leg 1 end, 06:28Z; REFRESH).** Head under validation `dbb8c346f932dbc5abed577e2a1ca333467aa32e` (origin/next at
the start, "Regenerate train-owned views"; commit time 2026-11-27T04:43:28Z, epoch 1795754608). First-parent log since
26.3 (0f7aa0180): release-snapshot-1126c (1f8d402ce), gcc-fixincludes-probe (9c04be00f), headers-stage2-gaps-h
(2dfb5aa3b), train regen (68af8ce95), headers-stage2-gaps-i (8643d0109), train regen (dbb8c346f). crates/ + csys/: 2
files changed (so libc.so.6 may change too: nothing skipped). **Serial 1** for 2026-11-27 (keke staging has no 27.x):
system `2.39+niji20261127.1-1`, coexist `0.0~snapshot20261127.1+dbb8c346f932`. Branch = head + handoff/ only.
Hosts as 1126C: builds in the shiki role `~/w/sh1` (Haswell-EP); counts and checks on keke (Zen 1) and mei
(Skylake-SP, stock 2.39). Job dirs shiki, keke, mei `~/work/RELEASE-SNAPSHOT-1127A/` = 1126C's via copy.sh + subst.sed
(local copies in the scratch dir `~/w/.scratch/RELEASE-SNAPSHOT-1127A/`; data file names untouched: y23p/ref241.txt
kept, leftover check clean). tree.tgz = `git archive` of the head (sha256 e6f84cd5...), on all three hosts, checked.
mei `logs-ystage2-stock` copied from 1126C (as before). Plants file handoff/RELEASE-SNAPSHOT-1127A-plants.txt (1126C's
six, paths/ids/version changed; tools/deb unchanged since 0f7aa0180).

## Running (started 06:17Z, detached)
* shiki `run-shiki.sh` (-> run-shiki.out): smoke-shiki-1 **done, SMOKE: RESULT bad=0**; then products.tgz/sha,
  gate-1, debs-1 (+ debs.sha), plants-1, heapnode-1, in that order.
* keke `run-keke2.sh`, mei `run-mei.sh`, `run-hdr.sh`, `run-extra.sh`: wait (poll) for `products.sha` and `debs.sha`
  in their own `~/work/RELEASE-SNAPSHOT-1127A/` (sha256-checked), then start 1126C's job set with 1127a labels.
  **They wait for a relay that is not automatic**: see next step 1.

## New job scripts (installed, tested on 26.3's debs as the before picture)
* mei `fp.sh`: names = 1126C's names.txt + the `stage2-gaps-i` probe family (the only family GAPS-H/I added).
* mei `yb10.sh COEXIST SYSTEM` (step 4): every GAPS-H/GAPS-I row: compile rows and value rows (constants, sizeof,
  offsetof from -S) vs stock 2.39; sched_attr family and __USE_TIME_BITS64 vs stock 2.41 (`x/r241`, relayed from
  keke GAPS-H's excalibur root, x86-64 parts). TSPTYPES/RIPCMDS tables counted apart (GAPS-I's reported gaps).
  On 26.3 (yb10-ref263-2): same=412 diff=332, exactly the GAPS-H/I names (aiocb64, comparison-fn, ftp-*, glob64*,
  igmp, isbit, printf-h-stdarg, routed*, rwhod, sched-*, talkd, termio, time-bits64*, timed, uapi-then-sched 6,
  sched-with-uapi 6). Expect diff=0 on 27.1; if uapi-then-sched / sched-with-uapi still DIFF, read the "stock error"
  line (the 2.41 side compiles its own UAPI; libniji gets the same UAPI via -idirafter).
* mei `gi.sh` (steps 5, 6): validation/fixincl_check.sh (this head's; prints whether equal to GAPS-I's) with
  GCC-FIXINCLUDES-PROBE's gcc-14 build over the installed system tree (installed form and merged form), GAPS-I's
  units.sh unit part (taken at run time), uuid2 (must show `uuid_time64@@UUID_2.40`). Not yet run.
* mei `g14.sh` (step 7, gcc-14 part): libstdc++ config/locale/gnu/*.cc with the Aqours gcc-14 log's flags (cow:
  gnu++98 + _GLIBCXX_USE_CXX11_ABI=0; cxx11: gnu++11), stock vs libniji; libgomp C units after a standalone libgomp
  configure (extracts libgomp from the orig once). The Aqours log (/srv/aqours/logs/stage2/gcc-14_14.2.0-19.log.zst,
  copied to g14/gcc14.log) stops in monetary_members_cow.cc on `__MON_DECIMAL_POINT`, `__MON_THOUSANDS_SEP`,
  `__MON_GROUPING`, `__POSITIVE_SIGN`, `__NEGATIVE_SIGN`. Not yet run (untested: fix and rerun if it errors).
* keke `units2.sh [SYSDEBDIR]` (step 6 extra units): fakechroot glob64.c, compiler-rt 19.1.7
  sanitizer_platform_limits_posix.cpp (clang-19 and g++), nftables 1.1.3 src/proto.c (igmp); sources in
  `u2/pkgs`. On 26.3 (units2-ref263-1): stock=0 for all 4, niji=1 (glob64_t members, struct termio, igmp members;
  sanitizer also ax25_parms_struct, nr_parms_struct, ppp_stats incomplete).

## Exact next steps
1. `~/bin/jobwait` shiki gate-1 / debs-1 as they come (products.tgz appears right after smoke). Relay with
   `bash ~/w/.scratch/RELEASE-SNAPSHOT-1127A/put.sh --get shiki '~/work/RELEASE-SNAPSHOT-1127A/<f>' <localdir>/` then
   `put.sh <keke|mei> '~/work/RELEASE-SNAPSHOT-1127A' <files>`, `sha256sum -c` at every hop: first products.tgz,
   products.sha, products-so.sha; later `debs/system/*.deb`, `debs/coexist/*.deb` into `debs/` plus debs.sha (put
   the debs dir with rsync -a so debs/system and debs/coexist keep their names; the waiters check `cd debs &&
   sha256sum -c ../debs.sha`). Put debs.sha LAST (it is the trigger).
2. After the debs reach mei: start (setsid nohup ~/bin/jobrun ... </dev/null, `-n` ssh with `timeout 15`, see
   below) `release-snapshot-1127a-yb10-1 test -- sh $W/yb10.sh $W/debs/coexist $W/debs/system`,
   `-gi-1 test -- bash $W/gi.sh`, `-g14-1 test -- bash $W/g14.sh`, `-yb9-1 test -- sh $W/yb9.sh C Y` (yb9's
   sched rows compare with 2.39 and are expected DIFF = the recorded difference; yb10 holds the 2.41 verdict).
   On keke: `-units2-1 test -- bash $W/units2.sh`.
3. Step 7: mei ybuild-1 (in run-extra) builds util-linux and inetutils (blocking): read its verdicts.
4. cmp13 on mei: cmp12.sh with the mask widened (`202611(2[0-7])`, `dbb8c346f|0f7aa0180|...` in the HEAD
   alternation, `112[0-7]`), both job lists as cmp12-a/-b.
5. Steps 8-10 as 1126C (startup numbers vs 26.3, lint, docs grep, stage on keke `~/release-staging/snapshot-20261127.1/`).
6. Report (this file, rewritten), handoff/reports/RELEASE-SNAPSHOT-1127A.json, precheck on shiki, handback-check.

## Notes
* ssh: a detached start over plain ssh kept the session open (shiki); use `timeout 15 ssh -n '... (setsid nohup X
  > out 2>&1 < /dev/null &)'`.
* Jobs so far: shiki smoke-shiki-1 (rc 0). Evidence/debug (26.3 debs): mei yb10-ref263-1 (superseded: sh variable
  clobbering in my probe(), fixed), yb10-ref263-2; keke units2-ref263-1.
