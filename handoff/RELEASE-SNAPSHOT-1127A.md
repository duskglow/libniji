# RELEASE-SNAPSHOT-1127A: snapshot validation of dbb8c346f (serial 2026-11-27.1: HEADERS-STAGE2-GAPS-H, -I)

COMPLETE: NOT snapshot-ready at dbb8c346f (serial 2026-11-27.1): blocking, step 4 and step 7: `<protocols/talkd.h>`
`DELETE` is 0 (stock 2), so inetutils' talkd `process.c` stops on "duplicate case value"; `<protocols/rwhod.h>` in C++
has no `whod::whoent`; `<scsi/scsi.h>` `ABORT`/`NOP` carry telnet's values (238/241, stock 6/8). Everything else
matches 26.3 or improves on it: fixincludes 0 rewritten, util-linux builds (43 debs), uuid `uuid_time64@@UUID_2.40`,
gcc-14 units no libniji-only stop, start-up unchanged.

## State
* Branch `task/release-snapshot-1127a` = origin/next `dbb8c346f932dbc5abed577e2a1ca333467aa32e` + handoff/ only
  (plants list from leg 1, this note, the json). Head under validation: dbb8c346f "Regenerate train-owned views",
  commit time 2026-11-27T04:43:28Z (epoch 1795754608). First-parent log since 26.3 (0f7aa0180): release-snapshot-1126c
  (1f8d402ce), gcc-fixincludes-probe (9c04be00f), headers-stage2-gaps-h (2dfb5aa3b), train regen (68af8ce95),
  headers-stage2-gaps-i (8643d0109), train regen (dbb8c346f). Nothing landed on next during the run.
* Serial 1: system `2.39+niji20261127.1-1`, coexist `0.0~snapshot20261127.1+dbb8c346f932`; both sort above 26.3's
  (`dpkg --compare-versions`). libc.so.6 `7ebc2614c2e9462a` (26.3 `1e9123e44e15e601`), ld.so `b75237e0db57db1f`.
* Staged on keke `~/release-staging/snapshot-20261127.1/` (9 debs, SHA256SUMS, MANIFEST). **Do not cut it**: the
  findings below block it.
* Hosts as 1126C: builds in the shiki role `~/w/sh1` (Haswell-EP); checks on keke (Zen 1) and mei (Skylake-SP). Job
  scripts `~/work/RELEASE-SNAPSHOT-1127A/` on all three (the next snapshot copies them).

## Blocking findings (steps 4 and 7)
1. **`<protocols/talkd.h>` `DELETE` = 0; stock 2.39 = 2** (GAPS-I row). Cause in the generator inputs:
   `spec/abi/constants.tsv` holds ONE value per name (`DELETE 0`, the `<arpa/nameser_compat.h>` meaning,
   `ns_uop_delete`); `spec/abi/enum-macros.tsv` already excludes talkd.h from the nameser alias (`not_in`), but the
   value then falls back to the global constant. Effect: inetutils (blocking, step 7) fails on the system flavour,
   `process.c:133: error: duplicate case value` (`case DELETE:` after `case LEAVE_INVITE:`, both 0); stock builds
   24 debs. Probes: cxxp-1 `_Static_assert (DELETE == 2)` stock PASS, libniji FAIL (C and C++); yb10-2 talkd rows
   (gcc and g++, 3 modes, both flavours) differ only in that value.
2. **Same class, `<scsi/scsi.h>`: `ABORT` stock 6 / libniji 238, `NOP` stock 8 / libniji 241** (telnet's values).
   Found by dupval-1, a compiler-only audit of every macro in `headers/dup-listed.tsv` (6583 name/header pairs; the
   other 6580 equal, no libniji-only failure). Not a regression: dupval-ref263-1 shows the same 3 on 26.3. It is
   known-wrong, so fix it with item 1 (per-header values for a name defined in several headers).
3. **`<protocols/rwhod.h>` in C++: libniji declares `struct whoent` at namespace scope; stock nests it in `struct
   whod`** (GAPS-I row). cxxp-1: `sizeof (whod::whoent)` compiles on stock, fails on libniji ("'whoent' is not a
   member of 'whod'"); `sizeof (struct whoent)` the reverse; C is equal both ways. C++ names that mention it also
   mangle differently. Fix: emit `whoent` inside `whod` (C scoping is unchanged by that).
4. **`<protocols/timed.h>` `ANYADDR` compiles without `NULL` in scope on libniji only** (`((void *) 0)` / `__null`;
   stock's expansion needs NULL from another header). Wider than stock, unrecorded; low impact. RULING NEEDED below.

## Steps (results; jobs in brackets; "as 26.3" = cmp13/tails masked comparison with 1126C's job, 0 lines differing)
* 1-2 head, serial: above.
* 3 rerun: every 1126C job rerun on dbb8c346f (74 labels, plus the new ones below). cmp13a-1/cmp13b-1: 65 jobs, 0
  masked lines differ from 26.3. tails-1 (jobs whose verdicts cmp13's pattern does not catch): all equal except PIDs,
  wall seconds and kernel-side start counts (user side equal), and fp (below).
* 4 header rows: yb10-2 (GAPS-H/I rows, 1126C step 4 rows; recorded rows vs stock 2.41 x86-64 headers): same=708
  diff=36, both-fail 114. The 36: ftp-tables gcc 6 = probe artefact (the four `FTP_NAMES` arrays hold exactly
  stock's strings; only their definition order differs, which no program can observe through the ABI; g++ SAME);
  talkd gcc+g++ 12 = finding 1; rwhod g++ 6 = finding 3; timed-members gcc+g++ 12 = finding 4. Every `sched_attr` / `__USE_TIME_BITS64` row SAME against
  2.41. GAP-DIFF 24 = GAPS-I's reported, not-added TSPTYPES/RIPCMDS tables (counted apart, as its report). yb9-1:
  same=28 diff=20, all `sched_attr`/`sched_setattr` vs 2.39 (the recorded difference). yb6/yb7/yb8, hp/hp2/hp3 0 DIFF;
  hp4 14 DIFF = syslog names alone, as 26.3.
* 5 fixincludes: gi-1 `fixincl_check` 0 headers rewritten (installed 242, merged 331).
* 6 units: gi-1 ok=89 fail=3 (c_locale.cc on the host libc too: harness, as GAPS-I); uuid `uuid_time64@@UUID_2.40`
  yes; keke units-1 15/15 both rc 0 (GAPS-G/H set). units2-1: fakechroot `glob64.c` and nftables igmp unit compile
  (both failed on 26.3); llvm-19 `sanitizer_platform_limits_posix.cpp` still stops (non-blocking): `<netax25/ax25.h>`
  `struct ax25_parms_struct`, `<netrom/netrom.h>` `struct nr_parms_struct`, `<net/if_ppp.h>` `struct ppp_stats`
  incomplete and `PPPIOCGASYNCMAP` undeclared (both missing on 26.3 as well).
* 7 builds: ybuild-1 util-linux PASS (43 = stock), npth, gnupg2, libev, keyutils PASS; **inetutils FAIL** (finding 1;
  26.3 stopped earlier, on FTP_NAMES). gcc-14 (non-blocking): a full build does not fit, so g14-2 compiles the units
  the Aqours log reaches with that log's flags (libstdc++ `config/locale/gnu` both ABIs with the host's libstdc++ 13
  headers, libgomp's C units with a standalone configure): libstdc++ 16 SAME, libgomp 50 SAME, 0 libniji-only stops
  (4 units fail on stock too: harness defines).
* 8 perl, apt, packages, sweeps: ystage2-niji perl rc 0; yapt aptTests pass; ypk 7 of 7; as 26.3.
* 9 busybox, #21, start-up, lint, docs: as 26.3. Start-up equal on both CPU classes (Zen 1 pie0 156,365 instr/52
  faults, pie3 241,640/67, time 167,313/56; Skylake-SP 156,607/52, 241,897/67, 167,526/55). heapnode-1: #21
  `find_over_200000_entries_bounds_its_address_space` fails as 26.3. fp-1 bad 4 -> 6: the 2 new are the `FP use`
  rows for `__USE_TIME_BITS64` (libniji defines it, stock 2.39 does not: D-X.178, recorded); forms +1
  replaced-form row and vis +1 line with 74 new names (not read one by one: NOT VERIFIED).
* 10 stage: done (above). shiki gate-1 GATE bad=0, debs-1 PASS (symbols 4492 lines), plants-1 6 CAUGHT / 0 SURVIVED
  (one WARN-DEV-SHM line: plants dir on /dev/shm of the Haswell-EP host).

## INFORMATIONAL
* keke smoke-keke-1 failed once ("a negative control does not build"); its log dir was removed by gaimass.sh's
  re-extraction before it was read. smoke-keke-3 on re-extracted, sha-checked products: bad=0. Not reproduced.
* I ran `sed -i` once on a remote data list (mei `cmp13-b.list`, `-ref262` -> `-ref263`) before noticing the rule.
* yb10 compares emitted data in order; the next snapshot should compare each object's contents (ftp-tables false
  DIFF), and its rwhod/timed C++ bodies should use stock's forms (`whod::whoent`, NULL from `<stddef.h>`).

## New job scripts (mei `~/work/RELEASE-SNAPSHOT-1127A/`; copy them into the next snapshot)
`cxxp.sh` (C++ scoping and the three values), `dupval.py` (every dup-listed macro's value per header vs stock),
`tails.sh` (tail-line comparison for jobs cmp13 cannot read), `vdiff.py` (token diff of yb10 value rows), `cmp13.sh`,
fixed `yb10.sh` and `g14.sh`.

## Jobs
JOBS (final head, fresh build): shiki: smoke-shiki-1, gate-1, debs-1, plants-1, precheck-1. keke: smoke-keke-3,
startup-keke-1, kstart-keke-1, rstart-keke-1, startup215-keke-1, gaimass-1, units-1, pkgdiff-1, stage-1, install-1.
mei: the 1126C set (smoke-mei-1 ... lint-1), gi-1, g14-2, cxxp-1, dupval-1, cmp13a-1, cmp13b-1, tails-1.
Evidence, rc != 0 by finding: shiki heapnode-1; keke hdrdiff-1, units2-1, units2-ref263-1; mei yb10-2, yb9-1,
ybuild-1, fp-1, hp4-1, ystage2cmp-1, drp-1, drp-ref263-1, dlcd-1, y14-1, y14r-1, yhc-1, yb10-ref263-3 (rc 1 by
design or by the findings above; see the json). 26.3 before-pictures: dupval-ref263-1. SUPERSEDED: keke smoke-keke-1
(unexplained), smoke-keke-2 (26.3 products left by gaimass); mei yb10-1, yb10-ref263-1/-2 (probe defects), g14-1
(verdict bug).

## Exact next step
Coordinator: brief the fix (per-header macro values in headers-gen for talkd `DELETE`, scsi `ABORT`/`NOP`; nested
`whoent` in rwhod.h; the ANYADDR ruling), then a snapshot after it reruns these job scripts. Plan rows: D-X.178.
