COMPLETE: snapshot-ready at 71597d2a4 (serial 2: 2.39+niji20261121.2-1, staged as snapshot-20261121.2; tag
snapshot-2026-11-21.2): no regression against 21.1; #21: malloc under `ulimit -v` and findutils' `find` under the
issue's formula now pass (find 5 of 5; the probe's absolute 35000 KiB still fails); libc6-dbg 19 of 19; plants 6/6 CAUGHT.

# RELEASE-SNAPSHOT-1121A: snapshot validation of 71597d2a4 (GitHub #21 partial fix, libresolv, dbg)

**State.** Validated head `71597d2a40ac281b2d0cd2bf4e03d17ba160bf88` (origin/next at the start, 05:09 UTC; "Regenerate
train-owned views"; commit time 2026-11-21T04:06:43Z, epoch 1795234003); `git merge-base --is-ancestor 1509aeb3` holds.
Branch `task/release-snapshot-1121a` = that head + handoff/ only (no product or packaging code changed). Role **mei
`~/w/s1` (Skylake-SP)**; keke = Zen 1. 21.1 = 15a2d6482 (RELEASE-SNAPSHOT-1120F). Two legs (refresh after leg 1).

**Landed since snapshot-2026-11-21.1** (`git log --oneline --first-parent 15a2d6482..71597d2a4`): libresolv-own-code
(142011daa), dyn-start-quick-2 (429cdf93f), release-snapshot-1120f (df83365cd, handoff only), sl-headers-s08-6
(a98d5c496), window-gate-stable (743cc2907), heap-address-space-10 (1509aeb35, #21), sys-dbg-dwarf (e6dfe4332), five
train view regenerations (10f7ac236, 0122a0d8a, e6d3dc76c, 7ac731794, 71597d2a4).

**Version / serial.** Commit date 2026-11-21 = the day's second snapshot: `NIJI_DEB_SERIAL=2`, debs-1 built
`2.39+niji20261121.2-1` (7 system packages) and `niji6{,-dev}_0.0~snapshot20261121.2+71597d2a40ac`. `dpkg
--compare-versions` (mei): `2.39+niji20261121.2-1` gt `2.39+niji20261121.1-1` and lt `2.40`;
`0.0~snapshot20261121.2+71597d2a40ac` gt `0.0~snapshot20261121.1+15a2d6482744`: all yes.

**Scripts** (mei and keke `~/work/RELEASE-SNAPSHOT-1121A/`; local scratch `~/w/.scratch/RELEASE-SNAPSHOT-1121A/`):
1120F's set copied by copy.sh + subst.sed (names, head, epoch, serial, role, labels only); tree/ = `git archive
71597d2a4` (sha256 checked on both hosts); run-mei.sh = 1120F's run-mei.sh + run-mei2.sh in one chain (serial known
from the start, so one debs round); cmp3.sh = cmp2.sh with per-job old labels (1120F's final ones). Plants file
handoff/RELEASE-SNAPSHOT-1121A-plants.txt (1120F's; ids, paths, role, version only; results appended). New: y21.sh
(#21), yres.sh (libresolv), rstart.sh (libresolv start-up), from y28.sh's / kstart.sh's recipes (mknew.sh,
mkrstart.sh). rstart-mei-1 was void: its `mr.c` (`return ns_initparse == 0;`) was folded by the compiler, so
`--as-needed` dropped libresolv.so.2 (res0 = pie0 on all three libraries, no NEEDED libresolv.so.2); mr.c now takes the
address through a volatile (NEEDED libresolv.so.2 checked on both hosts), rerun as rstart-mei-2 / rstart-keke-1.

## Results against snapshot-2026-11-21.1 (all on 71597d2a4's products / serial-2 packages)

cmp3.sh: each check's masked PASS/FAIL/RESULT lines against 1120F's final log of the same check.

| check | job | result | vs 21.1 |
|---|---|---|---|
| smoke lua_reload, cxx_dlclose | mei smoke-mei-1 (builds the products in s1), keke smoke-keke-1 (mei's products, sha checked) | `SMOKE: RESULT bad=0` both | same |
| gate | mei gate-1 rc 0 (clippy --all-features, xtask clippy, static, records check, affected --base origin/next) | `GATE bad=0` | same |
| system debs + check-system + symbols | mei debs-1 **rc 0** | `check: PASS (7 packages, version 2.39+niji20261121.2-1)`; check-symbols PASS 4440 lines, 13 libraries | same (4440) |
| coexist debs + check | debs-1 | `check: PASS (2 packages)`; version order (above) | same |
| **libc6-dbg (DBG)** | debs-1 | **19 of 19 ELF objects resolve; 19 .debug files** | **21.1: 17 of 19** (iconv, locale; ruling #6321): fixed by SYS-DBG-DWARF |
| package comparison | keke pkgdiff-1 rc 0 | 9 packages; file counts unchanged except libc6-dbg 44 -> 46; control fields: versions only; libc6-dbg 8,824,720 -> 11,030,512 B (+2,205,792; Installed-Size 9551 -> 11538); libc6 +3,588 B; libc-bin +968 B; libc6-dev -3,448 B; niji6 +4,672 B; niji6-dev -2,156 B | expected (dbg now carries DWARF for every object) |
| #16 rerun | mei yohane-1 rc 1 | `pass=16 fail=2` (libc6-dev ships no C headers) | same, 0 lines differ |
| many objects (#9) | mei many-1 | `pass=9 fail=0` | same |
| #22 | mei dyn22-1 | `pass=3 fail=0` | same |
| #10 + #18 | mei y10-1 rc 1 | 19 PASS / 1 FAIL (DL-RUNPATH, not landed) | same |
| #19 | mei y19-1 | `pass=14 fail=0` | same |
| #26 / gcc ASan | mei y26-1, y26t-1 | `pass=20 fail=0`; judge 0 | same |
| #23 | mei y23-1 | `pass=13 fail=0` | same |
| COMPAT19 | mei c19-1 | `pass=16 fail=0` | same |
| #25 (dynamic) | mei y25-1 | `pass=21 fail=0` | same |
| #25 static | mei y25s-1 | `pass=20 fail=0` | same, 0 lines differ |
| #24 | mei y24-1 | `pass=46 fail=0` | same |
| #28 | mei y28-1 | `pass=10 fail=0` | same |
| #6 | mei y6-1 rc 1 | `pass=79 fail=2` (WINDOWS-1252, windows-1251) | same, 0 lines differ |
| **#21** | mei y21-1; 21.1: y21-ref211-1 | `pass=7 fail=0`; tables below | changed (below) |
| **libresolv** | mei yres-1; 21.1: yres-ref211-1 | `pass=36 fail=0`, every row "same as stock" | same as 21.1 and stock |
| empty root | mei roots-1 | `root: RESULT pass=21 fail=0` | same |
| swapped trixie root | roots-1 | `verdict: commands=65 same=63 known=2 bad=0` | same |
| libc6-dbg + gdb | roots-1 (its only rc 1) | `dbg: RESULT pass=8 fail=1` trixie and excalibur (`#5 ... in ?? ()` in a running /usr/bin/true, known) | same; the setlocale source line moved (export.rs line 102 -> 116) |
| fresh install + ldconfig | roots-1 | `fresh: RESULT fail=0`; 121 -> 121 masked lines, 8 differ: the work-dir path (4) and the setlocale line number (4) | same |
| **lintian** | mei lint-1 rc 0 | errors=10 **warnings=46**, trixie and excalibur; no new tag | **21.1: 10 / 61**: gone per suite: 12 libc6-dbg `debug-file-with-no-debug-symbols`, `extended-description-line-too-long` on libc6 and libc-bin, libc6 `undeclared-elf-prerequisites` |
| coexist proofs | lint-1 | `proof: RESULT pass=21 fail=0` trixie and excalibur | same |
| plants | mei plants-1 rc 0 | 6 plants: 6 CAUGHT, 0 SURVIVED, 0 WARN | same |
| stage | keke stage-1 rc 0 | 9 assets + SHA256SUMS + MANIFEST (list below); `sha256sum -c` 9 OK | - |
| install instructions | keke install-1 rc 0 | `inst: RESULT fails=0` | same |
| docs | doccheck.py (1120F's) on the head | 23 problems, the same 23 as 21.1 (three line numbers in differences-from-glibc.md moved by +3) | same |

**Staged** (keke `~/release-staging/snapshot-20261121.2/`): libc6, libc6-dbg, libc6-dev, libc-bin, libc-dev-bin
(`_2.39+niji20261121.2-1_amd64.deb`), libc-l10n, locales (`_2.39+niji20261121.2-1_all.deb`), niji6, niji6-dev
(`_0.0.snapshot20261121.2+71597d2a40ac_amd64.deb`, `~` written as `.` for GitHub), SHA256SUMS, MANIFEST.

## #21 in the swapped trixie root (y21; probe21.c = the issue's probe; stock = trixie glibc 2.41)

The probe's steps, VmSize / VmRSS KiB (no limit):

| step | stock | 21.1 | 71597d2a4 | issue (stock 2.41) |
|---|---|---|---|---|
| start of main | 3,988 / 1,432 | 7,892 / 1,228 | 7,816 / 1,156 | 4,112 |
| 200000 x malloc(96..159) | 31,972 / 31,104 | 40,728 / 3,092 | 36,812 / 3,120 | 31,964 |
| free every other | 31,972 / 31,104 | 40,728 / 3,156 | 36,812 / 3,184 | 31,964 |
| refill with malloc(120) | 33,556 / 32,668 | 57,112 / 3,236 | 47,052 / 3,264 | 33,944 |
| free everything | 33,556 / 32,668 | 57,112 / 3,236 | 47,052 / 3,264 | 33,944 |
| realloc-doubled array to 200000 | 32,040 / 31,224 | 75,060 / 5,236 | 48,620 / 5,188 | 32,036 |
| bisected `ulimit -v` need | 29,568 | 57,648 | **43,120** | - |
| under `ulimit -v 35000` / 41296 | rc 0 / rc 0 | rc 10 / rc 10 (the 200000 mallocs fail) | rc 11 / rc 11 (the refill fails) | - |

(Observed only: the probe does not write its blocks; libniji's RSS stays near 3-5 MiB through every step, stock's
follows its VmSize.)

malloc until NULL under `ulimit -v` = start + 65536 KiB (loop21.c; obtained KiB / room left KiB):

| size | stock | 21.1 | 71597d2a4 |
|---|---|---|---|
| 100 | 58,456 / 64 | 43,575 / 0 | **56,021 / 0** |
| 4096 | 65,232 / 48 | 64,188 / 0 | 64,168 / 0 |
| 131072 | 63,488 / 64 | 48,512 / 60 | **61,056 / 48** |
| 1048576 | 64,512 / 640 | 48,128 / 444 | **60,416 / 696** |
| 16777216 | 49,152 / 16,240 | 32,768 / 15,804 | **49,152 / 11,960** |

findutils' `find` (bisected `ulimit -v` need, KiB; the issue's formula = need over 3 files + 35000, 5 randomised runs):

| | stock | 21.1 | 71597d2a4 |
|---|---|---|---|
| need, 3 files | 4,640 | 20,640 | 20,464 |
| need, 200000 entries | 31,616 | 65,536 | 47,248 |
| 200000 over 3 files (allowance 35,000) | 26,976 | 44,896 | **26,784** |
| the issue's formula | 5 of 5 succeed | 0 of 5 (`Cannot allocate memory`) | **5 of 5 succeed** |

The brief expected `find` to still fail on the head (HEAP-SPILL-COLD's acceptance): it passes, with 8,216 KiB to spare
(stock 8,024). The fixed cost is still higher (3 files: 20,464 vs 4,640 KiB; probe start 7,816 vs 3,988), so an
absolute `ulimit -v` near stock's need still fails (the probe under 35000).

## libresolv (yres, swapped trixie root; forked local responder; `-lresolv`)

res_init, res_query (A, AAAA, NXDOMAIN, silent server), res_search (short, NXDOMAIN, absolute), res_querydomain,
res_mkquery, dn_comp (5 names + bytes), dn_expand (5 + loop + short), inet_net_pton/ntop, ns_samename, p_class,
b64_ntop/pton, dn_count_labels: return values, `h_errno` and output same as stock on the head and on 21.1 (36 PASS,
both); NEEDED `libresolv.so.2 libc.so.6` as stock.

## Start-up (execcount, ASLR off, median of 101; startup-*-1, kstart-*-1, rstart-mei-2, rstart-keke-1)

User instructions / page faults:

| program | class | glibc 2.39 | snapshot-2026-11-21.1 | 71597d2a4 | delta |
|---|---|---|---|---|---|
| empty main | Skylake-SP | 107,752 / 49 | 746,622 / 57 | 741,092 / 52 | -5,530 / -5 |
| 3 libraries | Skylake-SP | 143,094 / 65 | 1,002,177 / 73 | 996,656 / 67 | -5,521 / -6 |
| clock_gettime | Skylake-SP | 109,862 / 50 | 780,622 / 61 | 775,102 / 56 | -5,520 / -5 |
| empty main | Zen 1 | 101,533 / 48 | 746,389 / 57 | 740,859 / 52 | -5,530 / -5 |
| 3 libraries | Zen 1 | 136,734 / 64 | 1,001,930 / 73 | 996,409 / 67 | -5,521 / -6 |
| clock_gettime | Zen 1 | 103,672 / 50 | 780,418 / 62 | 774,898 / 57 | -5,520 / -5 |

21.1's column reproduces 1120F's 15a2d6482 numbers exactly (Zen 1 empty main: 746,389 vs 746,390, one instruction). Kernel side (kernel instructions / syscalls):

| program | class | glibc | 21.1 | 71597d2a4 |
|---|---|---|---|---|
| empty main | Skylake-SP | 688,441 / 46 | 849,797 / 44 | 782,237 / 38 |
| 3 libraries | Skylake-SP | 984,832 / 72 | 1,271,726 / 88 | 1,190,588 / 80 |
| clock_gettime | Skylake-SP | 691,707 / 46 | 879,550 / 44 | 816,901 / 38 |
| empty main | Zen 1 | 697,236 / 42 | 867,824 / 43 | 800,333 / 37 |
| 3 libraries | Zen 1 | 994,332 / 68 | 1,294,693 / 87 | 1,211,766 / 79 |
| clock_gettime | Zen 1 | 705,499 / 42 | 903,340 / 43 | 837,023 / 37 |

Empty main's syscalls, 21.1 -> head: mmap 14 -> 12, munmap 7 -> 5, arch_prctl 3 -> 2, lseek 1 -> 0 (DYN-START-QUICK-2).

A program that needs libresolv.so.2 (res0: empty main taking `ns_initparse`'s address) beside an empty main (pie0),
user instructions / faults, kernel instructions / syscalls:

| class | program | glibc 2.39 | 21.1 | 71597d2a4 |
|---|---|---|---|---|
| Skylake-SP | pie0 | 107,645 / 49, 690,017 / 46 | 746,576 / 57, 849,008 / 44 | 741,046 / 52, 780,279 / 38 |
| Skylake-SP | res0 | 169,020 / 55, 820,295 / 58 | 993,047 / 61, 997,924 / 62 | 920,057 / 57, 974,187 / 55 |
| Skylake-SP | res0 - pie0 (user) | +61,375 | +246,471 | **+179,011** |
| Zen 1 | pie0 | 101,481 / 48, 697,755 / 42 | 746,293 / 57, 867,531 / 43 | 740,764 / 52, 801,424 / 37 |
| Zen 1 | res0 | 162,716 / 54, 840,598 / 54 | 992,508 / 61, 1,018,829 / 60 | 919,518 / 57, 990,051 / 53 |
| Zen 1 | res0 - pie0 (user) | +61,235 | +246,215 | **+178,754** |

## Findings

1. No regression in any check against 21.1 (table above); every compared check's lines are the same except the ones
   named.
2. **#21:** malloc under `ulimit -v` now obtains more of the limit before NULL at every size but 4096 (unchanged): 100 B
   56,021 KiB vs 21.1's 43,575 (stock 58,456), 1 MiB 60,416 vs 48,128 (stock 64,512), 16 MiB 49,152 vs 32,768 (as
   stock); findutils' `find` over 200000 entries passes the issue's formula 5 of 5 (21.1: 0 of 5).
   Still higher than stock: the start-up address space (3 files: 20,464 vs 4,640 KiB) and the probe's absolute need
   (43,120 vs 29,568 KiB). The brief's premise ("find expected to still fail") does not hold on this head:
   INFORMATIONAL for HEAP-SPILL-COLD's acceptance.
3. **libc6-dbg 19 of 19** (SYS-DBG-DWARF; ruling #6321's 17 of 19 exception no longer needed); debs-1 rc 0; lintian 15
   warnings fewer per suite.
4. **Start-up:** DYN-START-QUICK-2's gain shows on both classes (user -5.5k, -5 faults, -6 syscalls, kernel -60..80k per
   start); LIBRESOLV-OWN-CODE: loading libresolv.so.2 costs +179k user instructions (21.1: +246k; glibc +61k).
5. Docs (non-blocking; route to a docs story): docs/RELEASE-NOTES.md "Unreleased" still has only COMPAT-19: no entry
   for this snapshot's #21 (heap reservations, `ulimit -v`), libresolv's own code, libc6-dbg DWARF for every object,
   nor 21.1's (#28, #6 names, #25 static, #23); differences-from-glibc.md "Character sets" still gives
   snapshot-2026-11-17's state; no differences row for the -static-libgcc thread exit (ruling #7118). doccheck: the
   same 23 problems as 21.1.
6. The run's own slips: rstart-mei-1's libresolv program did not need libresolv (compiler-folded reference;
   superseded by rstart-mei-2); in leg 2 one local Bash call with a heredoc (a no-op python3 stub) was refused by the
   no-heredoc hook, nothing ran, the edits were made with the Edit tool instead. Keke's slots were full for most of leg
   2: each chained keke job queued 20-30 min behind other stories' long jobs (the main wait of this run).

## Jobs

JOBS (the final head 71597d2a4; this branch adds handoff/ only; rc 0 each): mei smoke-mei-1, gate-1, startup-mei-1,
kstart-mei-1, rstart-mei-2, debs-1, plants-1, lint-1, many-1, dyn22-1, y19-1, y26-1, y26t-1, y23-1, c19-1, y25-1,
y24-1, y28-1, y25s-1, y21-1, yres-1, precheck-1; keke smoke-keke-1, startup-keke-1, kstart-keke-1, rstart-keke-1,
pkgdiff-1, stage-1, install-1 (all labels prefixed release-snapshot-1121a-). Evidence, not named (rc 1 on known lines
only, as 1120F): roots-1 (dbg `#5 ... in ?? ()`), yohane-1 (libc6-dev ships no C headers), y10-1 (DL-RUNPATH, not
landed), y6-1 (WINDOWS-1252, windows-1251); 21.1 side: y21-ref211-1, yres-ref211-1. SUPERSEDED: rstart-mei-1 (its
libresolv program did not need libresolv.so.2).

## Next step

Coordinator: tag snapshot-2026-11-21.2 from keke `~/release-staging/snapshot-20261121.2/` at 71597d2a4 when the owner
says so and tell Yohane (#21: malloc under `ulimit -v` and findutils' `find` formula pass; libc6-dbg complete). Route
findings 2 (HEAP-SPILL-COLD's acceptance already met by `find`; the fixed start-up address space remains) and 5
(release notes, differences rows).
