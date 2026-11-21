COMPLETE: snapshot-ready at 15a2d6482 (serial 1: 2.39+niji20261121.1-1, staged as snapshot-20261121.1; tag
snapshot-2026-11-21.1): no regression against 20.5; #28, #6 names, #25 static, #23 fixed end to end; plants 6/6 CAUGHT.

# RELEASE-SNAPSHOT-1120F: snapshot validation of 15a2d6482 (GitHub #28, #6 names, #25 static, #23)

**State.** Validated head `15a2d64827445b4f39b3c5dd5d5bbcc9d2a852f5` (origin/next at the start, 01:04 UTC; "Merge
task/release-snapshot-1120e into next"; commit time 2026-11-21 00:49:33 UTC, epoch 1795222173); `git merge-base
--is-ancestor 1e377e145` holds. Branch `task/release-snapshot-1120f` = that head + handoff/ only (no product or
packaging code changed). Role **mei `~/w/s3` (Skylake-SP)**; keke = Zen 1. 20.5 = 305065b07 (1120E).

**Landed since snapshot-2026-11-20.5** (`git log --oneline 305065b07..15a2d6482`: 64 lines; first-parent merges):
thread-forced-unwind-static-2 (#25 static), libm-so-mvec (#28), iconv-name-match-4 (#6 names), deb-coexist-serial,
headers-gen-s08-hooks-2, stdio-wide-byte-reads-3 (#23), release-snapshot-1120e (handoff only), two train view
regenerations (1e377e145, b77a096b3).

**Version / serial (coordinator #7150, my challenge).** The head was committed on 11-21, and build.sh /
docs/install-packages.md take YYYYMMDD from the commit date, so the brief's `20261120.6` was impossible. debs-1 (serial
6: `2.39+niji20261121.6-1`) was superseded; the coordinator ruled serial 1: **debs-2** built `2.39+niji20261121.1-1`
(7 system packages) and `niji6{,-dev}_0.0~snapshot20261121.1+15a2d6482744`; staged at keke
`~/release-staging/snapshot-20261121.1/`; the tag will be snapshot-2026-11-21.1. `dpkg --compare-versions` (mei): system
21.1 gt 20.5 and gt 20.4 and lt 2.40; niji6 `0.0~snapshot20261121.1+15a2d6482744` gt 20.5's
`0.0~snapshot20261120+305065b071d6` and gt 20.4's `0.0~snapshot20261120+0e5a87e92723`: all yes.

**Scripts** (mei and keke `~/work/RELEASE-SNAPSHOT-1120F/`; local scratch `~/w/.scratch/RELEASE-SNAPSHOT-1120F/`, s1/
holds the leg-2 edits): leg 1's copy.sh set; leg 2 changed debs.sh (SERIAL=1), stage.sh + install.sh (staging dir
snapshot-20261121.1), inside.sh (coexist version had the pre-DEB-COEXIST-SERIAL form `0.0~snapshot20261120+...`; now
`0.0~snapshot20261121.1+15a2d6482744`), new run-mei2.sh (debs-2 then every deb check), chain-mei-debs2.sh (-2
labels), cmp2.sh (this run's labels vs 1120E's -1 logs; mask covers the new version forms). tree/ (= git archive
15a2d6482, sha256 of the tar checked against `git archive`) was missing on mei (host swap): roots-2 and lint-2 failed
at once on `tree/tools/deb/*.sh: No such file`, superseded by roots-3 and lint-3. Plants: plants-1 used 1120E's plants
file unchanged (baseline needed 1120E's products, absent on this host: exit 2 before any plant), superseded by
plants-2 with **handoff/RELEASE-SNAPSHOT-1120F-plants.txt** (paths, ids, role, version only).

## Results against snapshot-2026-11-20.5 (all on 15a2d6482's products / serial-1 packages)

| check | job | result | vs 20.5 |
|---|---|---|---|
| smoke lua_reload, cxx_dlclose | mei smoke-mei-1 (builds products in s3), keke smoke-keke-1 (mei's products, sha checked) | `SMOKE: RESULT bad=0` both | same |
| gate | mei gate-1 rc 0 (clippy --all-features, xtask clippy, static, records check, affected --base origin/next) | green | same |
| system debs + check-system + symbols | mei debs-2 | `check: PASS (7 packages, version 2.39+niji20261121.1-1)`; check-symbols PASS 4440 lines, 13 libraries | same (4440) |
| coexist debs + check | debs-2 | `check: PASS (2 packages)`; version order right by serial (above) | fixed (20.5: by hash) |
| debs-2 rc 1 | debs-2 | DBG only: 17 of 19 (iconv, locale; ruling #6321) | same |
| package comparison | keke pkgdiff-1 rc 0 | 9 packages, file counts unchanged (libc6 75, libc6-dbg 44, libc6-dev 38, libc-bin 22, niji6 29, niji6-dev 48, others 7); libc6-dbg: 5 build-ids replaced; control fields: versions only; sizes within 21 KB (libc6-dbg +20,824 B) | same |
| #16 rerun | mei yohane-2 rc 1 | `pass=16 fail=2` (libc6-dev ships no C headers) | same |
| many objects (#9) | mei many-2 | `pass=9 fail=0` | same |
| #22 | mei dyn22-2 | `pass=3 fail=0` | same |
| #10 + #18 | mei y10-2 rc 1 | 19 PASS / 1 FAIL (DL-RUNPATH, not landed) | same |
| #19 | mei y19-2 | `pass=14 fail=0` | same |
| #26 / gcc ASan | mei y26-2, y26t-2 | `pass=20 fail=0`; judge 0 | same |
| **#23** | mei y23-2 | **13 of 13** (`pass=13 fail=0`) | **20.5: 11 / 2** (invalid-plain, invalid-b) |
| COMPAT19 | mei c19-2 | `pass=16 fail=0` | same |
| #25 (dynamic) | mei y25-2 | `pass=21 fail=0` | same |
| #24 | mei y24-2 | `pass=46 fail=0` | same |
| **#28** | mei y28-1 rc 0; 20.5: y28-ref205-1 | `pass=10 fail=0` | 20.5: 5 / 5 |
| **#6** | mei y6-1 rc 1; 20.5: y6-ref205-2 | `pass=79 fail=2` | 20.5: 67 / 14 |
| **#25 static** | mei y25s-1 rc 0; 20.5: y25s-ref205-1 | `fail=0` (16 judged PASS) | 20.5: 4 FAIL |
| empty root | mei roots-3 | `root: RESULT pass=21 fail=0` (two new PASS rows: `-lm` mvec program runs, NEEDED libmvec.so.1 libc.so.6; nomvec NEEDED libm.so.6 libc.so.6) | 20.5: 19 / 0 (rows new with #28) |
| swapped trixie root | roots-3 | `verdict: commands=65 same=63 known=2 bad=0` | same |
| libc6-dbg + gdb | roots-3 (its only rc 1) | `dbg: RESULT pass=8 fail=1` trixie and excalibur (`Breakpoint 1.1`, known) | same |
| fresh install + ldconfig | roots-3 | `fresh: RESULT fail=0`; cmp2.sh: 119 -> 121 masked lines, 8 differ: the two new mvec rows, the pass count, and the work-dir path (4 lines) | same |
| lintian | mei lint-3 rc 0 | errors=10 warnings=61, trixie and excalibur; the one new warning `libc6: lacks-unversioned-link-to-shared-library example: usr/lib/x86_64-linux-gnu/libm.so [libm.so.6]` (libm.so is now #28's linker script, not a symlink) | 20.5: 10 / 60 |
| coexist proofs | lint-3 | `proof: RESULT pass=21 fail=0` trixie and excalibur | same |
| plants | mei plants-2 rc 0 | 6 plants: 6 CAUGHT, 0 SURVIVED, 0 WARN (handoff/RELEASE-SNAPSHOT-1120F-plants.txt) | same |
| stage | keke stage-1 rc 0 | 9 assets + SHA256SUMS + MANIFEST (list below); `sha256sum -c` 9 OK | - |
| install instructions | keke install-1 rc 0 | `inst: RESULT fails=0` | same |

**Staged** (keke `~/release-staging/snapshot-20261121.1/`): libc6, libc6-dbg, libc6-dev, libc-bin, libc-dev-bin
(`_2.39+niji20261121.1-1_amd64.deb`), libc-l10n, locales (`_2.39+niji20261121.1-1_all.deb`), niji6, niji6-dev
(`_0.0.snapshot20261121.1+15a2d6482744_amd64.deb`, `~` written as `.` for GitHub), SHA256SUMS, MANIFEST.
| docs | doccheck.py on the head (leg 1) | 23 problems, same 23 as 20.5, lines identical | same |

**#28 (y28, trixie gcc 14, plain `gcc -O3 -ffast-math ... -lm`):** head: mvec.c links, prints `0.8415`, NEEDED
`libmvec.so.1 libc.so.6` (as stock); nomvec.c NEEDED `libm.so.6 libc.so.6`, no libmvec (as stock); libm.so is the
linker script (`GROUP ( libm.so.6 AS_NEEDED ( libmvec.so.1 ) )`). 20.5: libm.so a symlink to libm.so.6; mvec link
fails `undefined reference to _ZGVbN4v_sinf`; nomvec same as stock. The issue's one-command form fails on both 20.5
and head with `math.h: No such file` (libc6-dev ships no C headers: known, yohane fail=2), stock links and prints 0.8415.

**#6 (y6, root with Debian `locales` for the charmaps, D-6.8):** iconv_open over 69 spellings + 11 iconv-program
spellings judged against stock: head 79 PASS / 2 FAIL, 20.5 67 / 14. Fixed since 20.5: `-UTF-8`, `UTF-8.`, `UT-F-8`,
`UTF_8`, `ISO-10646,/UTF8`, `-utf8`, `utf8.`, `latin-1`, `cp-1252` now refuse/open as stock; `ISO88591/ /TRANSLIT`
opens as stock; the iconv program for `-utf8`, `utf8.` as stock. **Remaining (non-CJK): `WINDOWS-1252` and
`windows-1251` do not open (stock opens; `cp1252`/`CP1251` open on both).** `iconv -l`: stock 1180 names, head 294 (267
in both), 20.5 293: the head adds `ISO88591`; stock lists `KOI8R`, `ISO885915`, `WINDOWS-1252`, the head does not.
CJK (shown, ICONV-CJK's): EUC-CN, EUCCN, UHC, ISO-2022-JP, ISO-2022-KR, ISO-2022-CN do not open; BIG5, BIG5-HKSCS,
EUC-JP(-MS), SHIFT_JIS/SJIS/CP932, GB2312, GBK, GB18030, EUC-KR same as stock.

**#25 static (y25s; forced_unwind_products.rs flags):** -static s-cxx, s-plain, s-fexceptions, s-buffers and dynamic
-static-libgcc C++ (d-slg-cxx): exit status and stdout as stock (head); 20.5: s-cxx timed out (rc 124, no dtors, trylock
16), s-fexceptions `order=K` vs `3a1K`, s-buffers `31K` vs `3a1K`. Shown only (ruling #7118, recorded difference):
-static-libgcc without libgcc_s: d-slg-fexceptions `order=K`, d-slg-buffers `31K` rc 0; d-slg-slstdcxx rc 124 (hang,
no dtors) on head and 20.5 (THREAD-EXIT-NO-UNWINDER, ruling #7119, not landed).

## Start-up (execcount, ASLR off, median of 101; startup-mei-1, startup-keke-1, kstart-mei-1, kstart-keke-1)

User instructions / page faults:

| program | class | glibc 2.39 | snapshot-2026-11-20.5 | 15a2d6482 | delta |
|---|---|---|---|---|---|
| empty main | Skylake-SP | 107,752 / 49 | 746,501 / 56 | 746,622 / 57 | +121 / +1 |
| 3 libraries | Skylake-SP | 143,094 / 65 | 1,002,062 / 72 | 1,002,177 / 73 | +115 / +1 |
| clock_gettime | Skylake-SP | 109,862 / 50 | 780,501 / 60 | 780,622 / 61 | +121 / +1 |
| empty main | Zen 1 | 101,533 / 48 | 746,269 / 56 | 746,390 / 57 | +121 / +1 |
| 3 libraries | Zen 1 | 136,734 / 64 | 1,001,815 / 72 | 1,001,930 / 73 | +115 / +1 |
| clock_gettime | Zen 1 | 103,672 / 50 | 780,298 / 61 | 780,418 / 62 | +120 / +1 |

20.5's column reproduces 1120E's 305065b07 numbers exactly. Kernel side (kernel instr / syscalls):

| program | class | glibc | 20.5 | 15a2d6482 |
|---|---|---|---|---|
| empty main | Skylake-SP | 691,968 / 46 | 841,002 / 44 | 853,051 / 44 |
| 3 libraries | Skylake-SP | 990,823 / 72 | 1,260,754 / 89 | 1,273,736 / 88 |
| clock_gettime | Skylake-SP | 692,973 / 46 | 869,261 / 44 | 882,844 / 44 |
| empty main | Zen 1 | 703,949 / 42 | 858,849 / 43 | 873,076 / 43 |
| 3 libraries | Zen 1 | 1,044,822 / 68 | 1,355,223 / 88 | 1,365,275 / 87 |
| clock_gettime | Zen 1 | 742,304 / 42 | 949,072 / 43 | 963,238 / 43 |

**Cause of the +1 fault (readelf -lW, products of 20.5 vs head):** libc.so.6's text grew 2,240 B (0x2e1389 ->
0x2e1c49) and its read-only data PT_LOAD 3,856 B (0xd92d4 -> 0xda1e4), which now ends past 0x3fa000, so the RW PT_LOAD
moves from 0x3fa000 to 0x3fb000; the loader's segments are unchanged. Layout (ruling #4347 jitter class; start-up cost
non-blocking for snapshots): INFORMATIONAL, routed to a start-up story; not chased per story.

## Findings

1. No regression in any check; #28, #6 (names), #25 static and #23 fixed end to end (tables above).
2. Start-up +1 fault, +115..121 user instructions, +10..14k kernel instructions per start; 3-library start one syscall
   fewer; both classes (cause above). Non-blocking.
3. #6 remaining: WINDOWS-1252 / windows-1251 (non-CJK) and the iconv -l spellings KOI8R, ISO885915, WINDOWS-1252; CJK
   EUC-CN, UHC, ISO-2022-JP/KR/CN (ICONV-CJK).
4. Docs (non-blocking; route to a docs story): docs/RELEASE-NOTES.md "Unreleased" has only COMPAT-19: no entry for #28
   (libm.so linker script, libmvec as needed), #6 names, #25 static, #23 cut-short sequences, the coexist serial, nor
   1120E's (a)-(d); differences-from-glibc.md "Character sets" intro still gives snapshot-2026-11-17's state (58 of 1180;
   ISO-8859-15 does not open; Debian's iconv program does not start): stale, dated; no differences row yet for the
   -static-libgcc-without-libgcc_s thread exit (ruling #7118 allows one).
5. Lintian: one new warning, `lacks-unversioned-link-to-shared-library` on libc6 for libm.so.6, because #28 made
   libc6-dev's libm.so a linker script (`GROUP ( libm.so.6 AS_NEEDED ( libmvec.so.1 ) )`) instead of a symlink.
   Expected and non-blocking; a packaging story may add a lintian override.
6. The run's own slips (fixed in-run): the plants file and tree/ were not adapted to the host swap (superseded jobs
   below); the brief's serial 20261120.6 could not match the head's commit date (challenge, ruling #7150: serial 1).

## Jobs

JOBS (the final head 15a2d6482; this branch adds handoff/ only; rc 0 each): mei smoke-mei-1, gate-1, startup-mei-1,
kstart-mei-1, plants-2, lint-3, many-2, dyn22-2, y19-2, y26-2, y26t-2, y23-2, c19-2, y25-2, y24-2, y28-1, y25s-1,
precheck-1 (GREEN, 33 checks, on 6a57821e8); keke smoke-keke-1, startup-keke-1, kstart-keke-1, pkgdiff-1, stage-1,
install-1 (all labels prefixed release-snapshot-1120f-). Evidence, not named (rc 1 on known lines only, as 1120E):
debs-2 (DBG 17 of 19, ruling #6321), roots-3 (dbg Breakpoint 1.1), yohane-2 (libc6-dev ships no C headers), y10-2
(DL-RUNPATH, not landed), y6-1 (WINDOWS-1252, windows-1251); 20.5 side: y28-ref205-1, y6-ref205-2, y25s-ref205-1
(rc 1: 20.5 fails the checks by design). SUPERSEDED: debs-1 (serial 6, ruling #7150), plants-1
(1120E's plants file, its products absent here), roots-2 and lint-2 (no tree/ on the host), y6-ref205-1 (root without
locales), every -1 deb check on mei (stopped or ran on the serial-6 packages: roots-1, lint-1, yohane-1, many-1,
dyn22-1, y10-1, y19-1, y26-1, y26t-1, y23-1, c19-1, y25-1, y24-1).

## Next step

Coordinator: tag snapshot-2026-11-21.1 from keke `~/release-staging/snapshot-20261121.1/` at 15a2d6482 when the owner
says so and tell Yohane (#28, #6 names, #25 static, #23). Route findings 2-5 (start-up layout, #6 rest / ICONV-CJK,
release notes and differences rows, lintian override).
