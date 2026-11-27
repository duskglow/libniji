# RELEASE-SNAPSHOT-1127A: snapshot validation of dbb8c346f (serial 2026-11-27.1: HEADERS-STAGE2-GAPS-H, -I)

**State (leg 2 end, ~07:30Z; REFRESH).** Head under validation `dbb8c346f932dbc5abed577e2a1ca333467aa32e` (origin/next
at the start, "Regenerate train-owned views"; commit time 2026-11-27T04:43:28Z, epoch 1795754608). First-parent log since
26.3 (0f7aa0180): release-snapshot-1126c (1f8d402ce), gcc-fixincludes-probe (9c04be00f), headers-stage2-gaps-h
(2dfb5aa3b), train regen (68af8ce95), headers-stage2-gaps-i (8643d0109), train regen (dbb8c346f). **Serial 1**: system
`2.39+niji20261127.1-1`, coexist `0.0~snapshot20261127.1+dbb8c346f932` (both `dpkg --compare-versions` gt 26.3's,
checked). Branch = head + handoff/ only. Hosts as 1126C: builds in the shiki role `~/w/sh1`; counts/checks on keke
(Zen 1) and mei (Skylake-SP). Job dirs `~/work/RELEASE-SNAPSHOT-1127A/` on all three; local copies of my scripts in
the scratch dir `~/w/.scratch/RELEASE-SNAPSHOT-1127A/{mei,keke}/` (edit there, put with `put.sh <host> <dir> <file>`).
libc.so.6 `7ebc2614c2e9462a` (26.3 `1e9123e44e15e601`), ld.so `b75237e0db57db1f` (26.3 `c1b0814f16ab99e3`).

## Done (do not redo)
* shiki: smoke-shiki-1 bad=0; gate-1 GATE bad=0; debs-1 DEBS bad=0 (check PASS 7+2 packages, check-symbols PASS 4492
  lines, 14 libraries = 26.3); plants-1 **6 CAUGHT, 0 SURVIVED**, 0 WARN-RC-ONLY/NO-MSG (one WARN-DEV-SHM line: the dir
  is /dev/shm on the Haswell-EP host; list it). heapnode-1 runs after plants (not yet read; 26.3: 6 passed / 1 FAILED
  `find_over_200000_entries_bounds_its_address_space`).
* Relay done: products + debs on keke and mei, sha256 -c at every hop.
* keke: startup/kstart/rstart/startup215 done: **start-up unchanged from 26.3** (every libniji row equal: Zen 1 pie0
  156,365/52, pie3 241,640/67, time 167,313/56; Skylake-SP 156,607/52, 241,897/67, 167,526/55; kstart/rstart user
  side equal). gaimass-1 bad=0; install-1 `inst: RESULT fails=0`; stage-1: keke `~/release-staging/snapshot-20261127.1/`
  9 debs + SHA256SUMS + MANIFEST; pkgdiff-1 file counts equal to 26.3 for all 9 packages; hdrdiff-1 (rc 1 by finding,
  read: 25 headers changed size; removals are internal `__niji_*`/`__NIJI_D_*`/`__aligned__` names only, as GAPS-H/I).
  units-1 rc 0 (GAPS-G's 15 units; verdict lines not yet read).
* **smoke-keke-1 FAILED** (1 s, "a negative control does not build"; its log dir was later removed: gaimass.sh by design
  re-extracts 27.1's then 26.3's products into tree/target and runs cargo there). smoke-keke-2 = SUPERSEDED (ran on
  26.3's products left by gaimass). **smoke-keke-3** (re-extracts 27.1 products, sha-checked) **bad=0, selftest PASS**.
  Report: single unexplained selftest failure, not reproduced (INFORMATIONAL).
* keke units2-1 (step 6 extra units): fakechroot glob64 stock=0 niji=0 and nftables igmp 0/0 (both niji=1 on 26.3);
  llvm-19 sanitizer_platform_limits_posix.cpp niji=1 (clang and g++): further header gaps (non-blocking): `<netax25/ax25.h>`
  struct ax25_parms_struct, `<netrom/netrom.h>` struct nr_parms_struct, `<net/if_ppp.h>` struct ppp_stats incomplete and
  PPPIOCGASYNCMAP undeclared. keke ppp.sh probe: both missing via `<net/if_ppp.h>` on 26.3 AND 27.1, stock has both ->
  not a regression (26.3's unit log did not list PPPIOCGASYNCMAP; not chased).
* mei gi-1 (steps 5, 6): **fixincl_check 0 rewritten** (installed form 242 headers, merged 331; GAPS-I reported 243),
  units ok=89 fail=3 = c_locale.cc gcc/clang on libniji AND on the host libc (harness, as GAPS-I's report), uuid
  `uuid_time64@@UUID_2.40` yes.
* mei yb9-1: same=28 diff=20, all DIFF = sched-attr/sched-setattr vs 2.39 (expected: recorded difference).
* **yb10 fixed** (my leg-1 probe was wrong; yb10-1 and yb10-ref263-1/-2 SUPERSEDED): x/r241's asm/ was 66 dangling
  symlinks into `lib/linux/uapi/x86/asm` (relayed now from keke excal/usr/lib/...; mei `x/r241/lib/`); value rows with
  both sides failing now count as both-fail SAME; sizeof(time_t) dropped for features.h/stdio.h (no time_t there).
  **yb10-2 (27.1): same=708 diff=36** (both-fail 114; gaps TSPTYPES/RIPCMDS diff=24, counted apart); yb10-ref263-3
  (26.3): same=418 diff=326. Every sched_attr / __USE_TIME_BITS64 row (ref 241) now SAME.
* g14-1 SUPERSEDED (shell precedence bug `A && B || C && D` in the verdict: both-0 rows printed DIFF). g14.sh fixed,
  **g14-2 started** (not read).

## Exact next steps
1. yb10-2's 36 DIFF (read `~/logs/release-snapshot-1127a-yb10-2.log`, the `values` / `error` lines under each DIFF):
   ftp-tables gcc (both flavours, 3 modes) values differ; talkd gcc+g++ values differ; rwhod g++ and timed-members
   gcc+g++: stock 2.39 FAILS, libniji compiles (read the stock error: a probe body defect, or libniji wider than stock?).
   Decide per class: probe defect (fix yb10, rerun as yb10-3) or a real finding (step 4 is blocking: report it).
2. jobwait mei g14-2; read RESULT and the first libniji-only stop (step 7 gcc-14 part, non-blocking).
3. mei main job set still running/queued (ybuild-1 = util-linux/inetutils, blocking; ystage2-niji perl; ~20 more).
   When done: `sh ~/work/RELEASE-SNAPSHOT-1127A/cmp13.sh $(cat cmp13-a.list)` and `... $(cat cmp13-b.list)` (cmp13 =
   cmp12 mask widened to 27.x/0f7aa0180/-ref26x; X-ref263 compared with 1126C's X) -> masked diffs vs 26.3. Also fp-1,
   hp*, hp4 (14 syslog-names-alone DIFF as 26.3), yb6/yb7/yb8 verdicts, exp-1, gai6-1/gai6s-1.
4. ybuild-1: util-linux and inetutils must build (stock 43 / 24 debs).
5. Read shiki heapnode-1, keke units-1 verdicts. Then report (rewrite this file), handoff/reports/RELEASE-SNAPSHOT-1127A.json
   (`jobs` + `evidence_jobs_rc_nonzero_by_finding` as 1126C), precheck on shiki (sync first), handback-check.
6. Note for the report: I ran `sed -i` on a remote data list (cmp13-b.list, -ref262 -> -ref263) before noticing the
   no-`sed -i` rule; say so.

## Jobs so far (27.1 build)
shiki: smoke-shiki-1, gate-1, debs-1, plants-1 (heapnode-1 pending). keke: smoke-keke-3, startup-keke-1, kstart-keke-1,
rstart-keke-1, startup215-keke-1, gaimass-1, units-1, units2-1, pkgdiff-1, stage-1, install-1, hdrdiff-1 (evidence).
mei: yb10-2, yb9-1 (evidence), gi-1, g14-2 (pending), + the main set. SUPERSEDED: keke smoke-keke-1 (unexplained
selftest fail), smoke-keke-2 (wrong products); mei yb10-1, yb10-ref263-1/-2 (probe defects), g14-1 (verdict bug);
keke units2-ref263-1 and mei yb10-ref263-3 are 26.3 before-pictures (evidence).
