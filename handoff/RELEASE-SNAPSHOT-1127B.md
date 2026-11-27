# RELEASE-SNAPSHOT-1127B: snapshot validation of 16e03637e (serial 2026-11-27.2: HEADERS-STAGE2-GAPS-H, -I, -J)

COMPLETE: snapshot-ready at 16e03637e78a141164666743d627418749153bdf. Steps 4-7 (blocking) pass: header rows 0 DIFF
(yb11 same=900 diff=0, per-object data; 26.3: same=430 diff=470), cxxp all as stock, dupval 0 (both lists), fixincludes
0 rewritten, units as 1127A, uuid `uuid_time64@@UUID_2.40`, util-linux AND inetutils build as with stock. Steps 8-9 as
26.3. One non-blocking finding needs a ruling (`struct osockaddr` through `<protocols/timed.h>`, below).

## State
* Branch `task/release-snapshot-1127b` = origin/next 16e03637e + handoff/ only (plants list, this note, the json).
  Head under validation: `16e03637e78a141164666743d627418749153bdf` "Regenerate train-owned views", commit time
  2026-11-27T10:58:36Z (epoch 1795777116). First-parent log since 26.3 (0f7aa0180): release-snapshot-1126c (1f8d402ce),
  gcc-fixincludes-probe (9c04be00f), headers-stage2-gaps-h (2dfb5aa3b), regen (68af8ce95), headers-stage2-gaps-i
  (8643d0109), regen (dbb8c346f), release-snapshot-1127a (887d5ab58), headers-stage2-gaps-j (c380b7efe), regen
  (16e03637e). Nothing landed on next during the run.
* Serial **2**: system `2.39+niji20261127.2-1`, coexist `0.0~snapshot20261127.2+16e03637e78a`; both sort above 26.3's
  (`dpkg --compare-versions` on the Version fields; 26.3's coexist file name has `.` for `~`, its Version field `~`).
  Serial 1 was 1127A's staged, unpublished build of dbb8c346f: reusing it would give one version two builds.
* Role mei `~/w/s1` (Skylake-SP), not shiki sh1 (held by HEADERS-STAGE2-GAPS-K; coordinator #8802). libc.so.6 sha256
  `1d9e2e6e25c4a536...`, ld.so `20a09f9f187602e7...`.
* **Staged** on keke `~/release-staging/snapshot-20261127.2/`: 9 debs, SHA256SUMS (9 OK, equal to the built debs'
  sums), MANIFEST. Not published, nothing to GitHub, no tag.
* Job scripts: mei and keke `~/work/RELEASE-SNAPSHOT-1127B/` (1127A's, head/epoch/serial/role/labels changed; new
  `addstock.sh` on keke, `yb11.sh`, `objval.py` on mei). The next snapshot copies them, **including `logs-ystage2-stock/`
  and symlinks** (see INFORMATIONAL).

## Steps (jobs in brackets; "as 26.3" = cmp13/tails masked comparison with 1126C's job)
* 4 header rows: [yb11-1] same=900 diff=0 (both-fail 120: the same stock-side groups as on 26.3's debs, yb11-ref263-2;
  168 recorded rows vs stock 2.41, all SAME). [yb10-1] same=738 diff=6: the 6 are ftp-tables gcc definition order, SAME
  per object in yb11; GAPS-I's TSPTYPES/RIPCMDS gap rows now same=24 diff=0. [yb9-1] diff=20 = sched_attr/sched_setattr
  vs 2.39 (recorded). [cxxp-1] every case as stock: `whod::whoent` PASS both, `DELETE == 2` PASS both (C, C++),
  ANYADDR without NULL now fails on both (1127A finding 4 gone). [dupval-1] 1127A's list 6583 same, diff 0;
  [dupval-2] this head's list 6532 same, diff 0. yb6/yb7/yb8/hp/hp2/hp3 0 DIFF; hp4 14 DIFF = syslog names, as 26.3.
* 5 fixincludes [gi-2]: `fixincl_check` 0 rewritten (installed 242, merged 331).
* 6 units [gi-2] ok=89 fail=3 (c_locale.cc harness, as 1127A), uuid `uuid_time64@@UUID_2.40` yes; [keke units-1]
  15/15; [keke units2-1] fakechroot glob64 and nftables igmp compile; llvm-19 `sanitizer_platform_limits_posix.cpp`
  (non-blocking) first stop now `<netipx/ipx.h>`: `SIOCAIPXITFCRT`, `SIOCAIPXPRISLT`, `SIOCIPXCFGDATA` undeclared
  (clang and g++), as GAPS-J reported; the ax25/netrom/ppp stops of 1127A are gone.
* 7 builds [ybuild-1]: util-linux PASS (43 debs = stock), **inetutils PASS (24 = stock)**, npth, gnupg2, libev,
  keyutils PASS. gcc-14 [g14-2]: libstdc++ 16 SAME, libgomp 50 SAME, 0 libniji-only stops.
* 8 [cmp13a-1, cmp13b-1] 65 jobs: 64 with 0 masked lines differing; roots' 4 are the job directory path only.
  ystage2-niji perl, yapt, ypk 7/7 as 26.3; [ystage2cmp-2] YS2CMP bad=1, log identical to 1127A's.
* 9 [tails-1] all SAME except PIDs, wall seconds and kernel-side counts (user side equal) and fp bad 4 -> 6 (the
  `__USE_TIME_BITS64` rows, D-X.178), as 1127A. Start-up equal to 1127A: Zen 1 pie0 156,365 instr/52 faults, pie3
  241,640/67, time 167,313/56; Skylake-SP 156,607/52, 241,897/67, 167,526/55. [heapnode-1] only #21
  `find_over_200000_entries_bounds_its_address_space` fails, as 26.3. [lint-1] bad=0. [gate-1] clippy -D warnings
  (all features), xtask clippy, static, records check, affected: all rc 0. [plants-1] 6 CAUGHT, 0 SURVIVED, no WARN. [precheck-1] GREEN (34 checks)
  at c866b6148; the later commit is handoff/ only.
  [debs-1] DEBS bad=0. keke smoke, gaimass, install (system debs different 0), pkgdiff, stage as 1127A.
* Header additions vs 26.3 [keke hdrdiff-1, addstock-1, addstock-a-1]: removals unchanged (14 headers, same set as
  1127A); added names 2898 (addstock's union of macro and identifier names; hdrdiff counts 2910; 27.1: 1021), of which not in stock 2.39's same header and mode 435, user-namespace 57:
  sched.h 28 + spawn.h 27 = the recorded 2.41 `sched_attr` family (spawn.h now reaches it through sched.h), and
  `osockaddr` from `<protocols/timed.h>` (2). The rest are reserved names (`__USE_TIME_BITS64`, libniji's `__NIJI_*`
  guards, gcc's size_t guards, kernel UAPI guards).

## RULING NEEDED
* **`<protocols/timed.h>` declares `struct osockaddr`; stock's does not** (since GAPS-I, in 27.1 too, not in 26.3). A
  program that includes it and declares its own `struct osockaddr` fails on libniji only ("redefinition of struct
  osockaddr", C, default and _GNU_SOURCE; stock compiles). Not one of the brief's blocking rows; none of the built
  packages hits it. Recommend: cut the snapshot, list it as a known limitation, fix forward in the next headers story
  (timed.h without the osockaddr declaration, checked by an addstock-style user-namespace audit).

## INFORMATIONAL
* Leg 1's job copy missed `logs-ystage2-stock/` (the stock side 1127A copied from 1125B), so ystage2cmp-1 read no stock
  data (bad=4); copied as-is, ystage2cmp-2 bad=1 = 1127A. Leg 1's new driver ran `gi.sh` and `g14.sh` (bash) under
  `sh`, so gi-1/g14-1 stopped on syntax; rerun under bash as gi-2/g14-2. Both are my defects; neither touched libniji.
* SUPERSEDED: mei gi-1, g14-1 (run under sh), ystage2cmp-1 (stock side missing), yb11-ref263-1 (probe defects).

## Exact next step
Coordinator: rule on osockaddr; cut snapshot 2026-11-27.2 from keke `~/release-staging/snapshot-20261127.2/`; tell
Yohane to retry gcc-14. Plan rows: D-X.178.
