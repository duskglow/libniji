COMPLETE: snapshot-ready at cd1bbabdb (serial 5: 2.39+niji20261121.5-1, staged as snapshot-20261121.5; tag
snapshot-2026-11-21.5): no regression against 21.4; #14 in the package (zic and zdump in libc-bin: against glibc
2.39's zic 591 of 598 zones decode the same, the rest read the same; zdump -v 29,787 lines identical; localedef
still missing, #14 stays partial); #20 as FCLOSE-COOKIE-SEEK states (37 rows, 3 ruled differences); wide reads
640 -> 120 instructions per 2-byte fgetwc_unlocked; start-up unchanged; plants 6/6 CAUGHT.

# RELEASE-SNAPSHOT-1121D: snapshot validation of cd1bbabdb (#14 zic/zdump, #20 stdio, WIDE-READ-FAST)

**State.** Validated head `cd1bbabdba59474b6c9f8605ed08c057452ccbd1` (origin/next at the start, 16:01 UTC;
"Regenerate train-owned views"; commit time 2026-11-21T15:08:59Z, epoch 1795273739); contains a42e446a (TZ-TOOLS-2,
#14) and 11381084 (FCLOSE-COOKIE-SEEK, #20). Branch `task/release-snapshot-1121d` = that head + handoff/ only (no
product or packaging code changed). Role **mei `~/w/s1` (Skylake-SP)**; keke = Zen 1. 21.4 = e4d7011e6
(RELEASE-SNAPSHOT-1121C). Two legs. origin/next has moved since (4aff6705c); not validated here.

**Landed since snapshot-2026-11-21.4** (`git log --oneline --first-parent e4d7011e6..cd1bbabdb`):
release-snapshot-1121c (5e1903cce, handoff), wide-read-fast (9f4eccfa9), view regen (7ca864312), sl-headers-s11-7
(4dbdcc68b), view regen (43058b7df), heap-gap-test-keke (8e34f9c72), tz-tools-2 (a42e446af, #14),
fclose-cookie-seek (113810845, #20), dl-rdebug-version (8ca99d206), view regen (cd1bbabdb).

**Not in the head** (in the CI queue at the start): COMPAT19-LDSO-COST (start-up tables below are without it);
DL-PROCESS-OBJECTS-2 (dlc runs the corpus without it; no 600-library run); VALGRIND-DBG-RODATA (#13: y13 still
aborts with the whole libc6-dbg, as 21.4); ICONV-CJK (#6: y6 as 21.4, pass=79 fail=2; CJK sets not checked).

**Version / serial.** Commit date 2026-11-21 = the day's fifth snapshot: `NIJI_DEB_SERIAL=5`; debs-1 built
`2.39+niji20261121.5-1` (7 system packages) and `niji6{,-dev}_0.0~snapshot20261121.5+cd1bbabdba59`. `dpkg
--compare-versions`: `2.39+niji20261121.5-1` gt `2.39+niji20261121.4-1` and lt `2.40`;
`0.0~snapshot20261121.5+cd1bbabdba59` gt `0.0~snapshot20261121.4+e4d7011e669f`: all yes.

**Scripts** (mei and keke `~/work/RELEASE-SNAPSHOT-1121D/`; local edits in the run's scratch `mirror/`): 1121C's set
by copy.sh + subst.sed (every input dir incl. y13/ y27/ sig32/); tree.tgz = `git archive cd1bbabdb` (8807 files =
ls-tree; sha checked on both hosts). Plants file handoff/RELEASE-SNAPSHOT-1121D-plants.txt (1121C's; each pattern
present once). New: **y14.sh** (#14 in the swapped trixie root, stock = trixie's libc-bin 2.41), **y14r.sh** (#14
against the reference: the packaged zic and zdump vs glibc 2.39's on one source; y14/interp.py points a copy's
PT_INTERP at the packaged loader), **y20.sh** (#20, 37 rows), y14/eetwet.sh (probe). y19/post.sh: the loader's
`_r_debug` expectation 40 -> 48 (DL-RDEBUG-VERSION; finding 3).

## Results against snapshot-2026-11-21.4 (all on cd1bbabdb's products / serial-5 packages)

cmp4.sh (masked PASS/FAIL/RESULT/verdict lines against 1121C's log of the same check): 0 lines differ for 17 checks;
smoke: the role path only; debs: DBG 19 -> 21 (zic, zdump); y19: finding 3. y27-1, y13-1, roots-1 diffed against
1121C's y27-2, y13-1, roots-2: 0 lines differ.

| check | job | result | vs 21.4 |
|---|---|---|---|
| smoke lua_reload, cxx_dlclose | mei smoke-mei-1, keke smoke-keke-1 | `SMOKE: RESULT bad=0` both | same |
| gate | mei gate-1 rc 0 | clippy, static, records rc 0; `GATE bad=0` (handoff-only branch) | same |
| system debs + check-system + symbols | mei debs-1 rc 0 | `check: PASS` (7 packages); check-symbols PASS 4440 lines, 13 libraries | same |
| coexist debs + check | debs-1 | `check: PASS (2 packages)`; version order above | same |
| libc6-dbg | debs-1 | **21 of 21** ELF objects resolve; 21 .debug files | 19 -> 21 (zic, zdump) |
| package comparison | keke pkgdiff-1 rc 0 | libc-bin + `usr/bin/zdump`, `usr/sbin/zic` (+20,812 B; Installed-Size 1967 -> 2035 KiB); libc6-dbg 47 -> 51 files; control: versions only | expected |
| #16 rerun | mei yohane-1 rc 1 | `pass=16 fail=2` (libc6-dev ships no C headers) | same |
| many objects (#9) | mei many-1 | `pass=9 fail=0` | same |
| #22 | mei dyn22-1 | `pass=3 fail=0` | same |
| #10 + #18 | mei y10-1 rc 1 | 19 PASS / 1 FAIL (DL-RUNPATH, not landed) | same |
| #19 | mei y19-2 (y19-1: finding 3) | `pass=14 fail=0` | same with the recorded 48-byte `_r_debug` |
| #26 / gcc ASan | mei y26-1, y26t-1 | `pass=20 fail=0`; judge 20/20 x3 | same |
| #23 / #23 probe | mei y23-1, y23p-1 | `pass=13` / `pass=9`, fail 0 | same |
| loader corpus | mei dlc-1 | `pass=5 fail=0` | same |
| COMPAT19 | mei c19-1 | `pass=16 fail=0` | same |
| #25 dynamic / static | mei y25-1, y25s-1 | `pass=21` / `pass=20`, fail 0 | same |
| #24, #28 | mei y24-1, y28-1 | `pass=46`, `pass=10`, fail 0 | same |
| #6 | mei y6-1 rc 1 | `pass=79 fail=2` (WINDOWS-1252, windows-1251) | same |
| #21 | mei y21-1 | `pass=7 fail=0` | same |
| libresolv | mei yres-1 | `pass=36 fail=0` | same |
| #27 | mei y27-1 | `pass=17 fail=0` | same |
| #13 | mei y13-1 rc 1 | `pass=6 fail=3` (the whole-libc6-dbg rows; VALGRIND-DBG-RODATA not in the head) | same |
| signal 32 (#7075) | mei sig32-1 | `fail=0`: default before the first thread, handler after | same |
| empty root / swapped trixie root | mei roots-1 | `root: RESULT pass=21 fail=0`; `verdict: commands=65 same=63 known=2 bad=0` | same |
| libc6-dbg + gdb | roots-1 (its only rc 1) | `dbg: RESULT pass=8 fail=1` (known) | same |
| lintian | mei lint-1 rc 0 | errors=10 warnings=48; proofs pass=21 fail=0, trixie and excalibur | +2 warnings: `no-manual-page` zic, zdump |
| **#14** | mei y14-1 rc 1, y14r-2 rc 1, eetwet-1; 21.4: y14-ref214-1 | below | **21.4: no zic, no zdump** |
| **#20** | mei y20-1 | `RESULT fail=0`, 37 rows; below | **21.4 differs on 15 rows** |
| plants | mei plants-1 rc 0 | 6 plants: 6 CAUGHT, 0 SURVIVED, 0 WARN | same |
| stage | keke stage-1 rc 0 | 9 assets + SHA256SUMS + MANIFEST; `sha256sum -c` 9 OK | - |
| install instructions | keke install-1 rc 0 | `inst: RESULT fails=0` | same |
| docs | doccheck.py (1120F's) on the head | 23 problems, the same 23 lines as 21.4 (two line numbers moved) | same |

**Staged** (keke `~/release-staging/snapshot-20261121.5/`): libc6, libc6-dbg, libc6-dev, libc-bin, libc-dev-bin
(`_2.39+niji20261121.5-1_amd64.deb`), libc-l10n, locales (`_2.39+niji20261121.5-1_all.deb`), niji6, niji6-dev
(`_0.0.snapshot20261121.5+cd1bbabdba59_amd64.deb`, `~` written as `.` for GitHub), SHA256SUMS, MANIFEST.

## #14: zic and zdump (libc-bin)

In the swapped trixie root (y14-1; tzdata 2026c, `/usr/share/zoneinfo/tzdata.zi` and `leapseconds`): libniji's
`/usr/sbin/zic` and `/usr/bin/zdump` come from libc-bin 2.39+niji20261121.5-1; zic compiles the whole source in
each mode (default, `-b fat`, `-b slim`, `-b fat -L leapseconds`): rc 0, 598 files, stderr lines as stock (0, 0, 0,
1). The 22 invalid-name cases for zic (TZ-TOOLS-2's: as zone and as link) give the stock status, messages and
leftovers. `localedef`: absent (libniji ships none): **#14 stays partial**. 21.4 (y14-ref214-1): no zic, no zdump,
no localedef after the swap.

trixie's libc-bin is glibc 2.41, a later zic and zdump than the reference, so y14's decoded comparison with it
(differ: default 598, fat 209, slim 35, leap 595) and its zdump diff (360 lines: out-of-range times printed as
`(gmtime failed)` by 2.41, `= NULL` by libniji) are version skew, not a judgement. y14r-2 compares with the
reference itself: glibc 2.39's zic and zdump (Ubuntu 24.04 libc-bin 2.39-0ubuntu8.9) against the packaged ones (run
under the packaged loader and libc.so.6) on one source (tzdata 2026e):

| judged | result |
|---|---|
| zic default / `-b fat` / `-b fat -L`: rc, files, stderr lines | as the reference (0, 598, 0 / 0 / 1) |
| zic default, fat, `-L` decoded (tzdec.py: transitions with indicators, type before the first, leaps, footer) | 591 of 598 the same (198 byte-identical) |
| zic `-b slim` decoded | 590 of 598 the same (274 byte-identical) |
| the differing zones | TZ-TOOLS' known list (Asia/Tbilisi, Europe/Lisbon, Portugal, Europe/Warsaw, Poland; slim also Ciudad_Juarez, Ojinaga, Indiana/Winamac, Gaza, Hebron) **plus EET and WET** (fat, default, `-L`): finding 4 |
| EET, WET, Warsaw read through the reference's zdump (`-v -c 1800,2100`) | the same (496, 496, 582 lines) |
| zdump `-v`, `-v -c 1800,2100`, `-i -c 1800,2100` on 14 zones (UTC, London, Dublin, New_York, St_Johns, Sao_Paulo, Lord_Howe, Kolkata, Tehran, Chatham, Casablanca, Troll, Lisbon, Gaza) | **29,787 lines, identical** |
| zdump on 4 names that are no zone (`No/Such_Zone`, `../../etc/passwd`, `/etc/passwd`, empty) | stdout and status as the reference; stderr differs: finding 5 |

## #20: stdio (y20; glibc 2.39 vs 21.4 vs head; FCLOSE-COOKIE-SEEK's cases)

37 rows, `RESULT fail=0`: every row as glibc 2.39 except the three FCLOSE-COOKIE-SEEK lists as ruled (its handoff,
RULING NEEDED there): the descriptor offset after fclose of a partly read `r`/`r+` file (head 10, the bytes
consumed; glibc 100, the read-ahead kept); fflush with a failing cookie write hook (head EINVAL, ferror 1; glibc
ferror 0); a cookie stream with no seek hook (head fflush 0, ftell / fseek -1 ESPIPE; glibc -1 with errno
unchanged). 21.4 differs from the head on 15 rows: fclose of a cookie stream whose hook fails (-1 EINVAL after one
seek, 8 rows), a seek at fclose with a working hook (4), fflush with the failing hook (-1 EINVAL, ferror 1), ftell /
fseek without a seek hook (EINVAL instead of ESPIPE).

## Wide reads (percall-1, Skylake-SP; instructions per call, sd 0)

| call, character width | glibc 2.39 | 21.4 | head |
|---|---|---|---|
| fgetwc_unlocked 1 / 2 / 3 / 4 bytes | 35.6 / 58.1 / 84.9 / 98.1 | 363 / 640 / 908 / 1173 | 89.1 / 120.2 / 138.4 / 152.6 |
| fgetwc 1 / 2 / 3 / 4 bytes | 67.6 / 90.2 / 116.9 / 130.1 | 373 / 650 / 918 / 1183 | 95.1 / 126.2 / 144.4 / 158.6 |

## Start-up (execcount, ASLR off, median of 101; startup-*-1, kstart-*-1, rstart-*-1)

21.4 = 1121C's products rerun here (p214); they reproduce 1121C's numbers. Nothing on the start-up path landed: the
head equals 21.4 to within 1 instruction on every row, faults identical.

User instructions / page faults:

| program | class | glibc 2.39 | snapshot-2026-11-21.4 | cd1bbabdb | delta |
|---|---|---|---|---|---|
| empty main | Skylake-SP | 107,752 / 49 | 741,532 / 51 | 741,532 / 51 | 0 / 0 |
| 3 libraries | Skylake-SP | 143,094 / 65 | 984,519 / 65 | 984,518 / 65 | -1 / 0 |
| clock_gettime | Skylake-SP | 109,862 / 50 | 763,624 / 54 | 763,624 / 54 | 0 / 0 |
| empty main | Zen 1 | 101,533 / 48 | 741,300 / 51 | 741,300 / 51 | 0 / 0 |
| 3 libraries | Zen 1 | 136,734 / 64 | 984,271 / 65 | 984,271 / 65 | 0 / 0 |
| clock_gettime | Zen 1 | 103,672 / 50 | 763,421 / 55 | 763,420 / 55 | -1 / 0 |

Kernel instructions / syscalls (kernel counts move by up to ~50k between runs of one tree; syscalls are exact):

| program | class | glibc | 21.4 | cd1bbabdb |
|---|---|---|---|---|
| empty main | Skylake-SP | 684,672 / 46 | 720,784 / 29 | 723,928 / 29 |
| 3 libraries | Skylake-SP | 990,648 / 72 | 1,118,407 / 59 | 1,124,129 / 59 |
| clock_gettime | Skylake-SP | 688,480 / 46 | 752,949 / 29 | 753,471 / 29 |
| empty main | Zen 1 | 736,743 / 42 | 786,987 / 28 | 787,269 / 28 |
| 3 libraries | Zen 1 | 997,035 / 68 | 1,142,895 / 58 | 1,140,629 / 58 |
| clock_gettime | Zen 1 | 707,382 / 42 | 770,917 / 28 | 771,675 / 28 |

Per-call syscall lists (kstart's "by name") are the same for 21.4 and the head on every row; clock_gettime,
Skylake-SP: `mmap 11, openat 5, mprotect 2, arch_prctl 2, uname, set_tid_address, set_robust_list, rseq, munmap,
fstat, exit_group, execve, close 1` (29).

The libresolv pair (res0 needs libresolv.so.2; pie0 empty), user instructions, then res0's kernel instructions /
syscalls:

| class | | glibc 2.39 | 21.4 | cd1bbabdb |
|---|---|---|---|---|
| Skylake-SP | pie0 / res0 user | 107,645 / 169,020 | 741,500 / 920,426 | 741,500 / 920,426 |
| Skylake-SP | res0 - pie0 | +61,375 | +178,926 | +178,926 |
| Skylake-SP | res0 kernel / syscalls | 816,903 / 58 | 919,815 / 42 | 920,425 / 42 |
| Zen 1 | pie0 / res0 user | 101,481 / 162,716 | 741,218 / 919,887 | 741,217 / 919,887 |
| Zen 1 | res0 - pie0 | +61,235 | +178,669 | +178,670 |
| Zen 1 | res0 kernel / syscalls | 849,096 / 54 | 988,769 / 40 | 991,456 / 40 |

## Findings

1. No regression in any check against 21.4: every compared check's verdict lines are the same, start-up equal.
2. **#14 in the package (partial):** zic and zdump ship in libc-bin and work in the swapped trixie root; against the
   reference (glibc 2.39) zdump's readings are identical and zic's output decodes the same for 591 of 598 zones
   (590 slim), the rest reading the same. localedef is still missing: tell Yohane #14 is partial.
3. **y19's `_r_debug` row changed by design:** DL-RDEBUG-VERSION makes the loader's `_r_debug` the 48-byte extended
   structure, `r_version` 2 (differences row "`_r_debug`'s `r_version` and size", D-27.62, COMPAT-36); y19-1 failed
   only on its hard-coded 40. With the expectation changed, y19-2: `pass=14 fail=0`; gnat1's 40-byte COPY, the
   version-1 reads of r-nopie / r-pie and the Ada build pass as before.
4. **INFORMATIONAL (TZ-TOOLS follow-up):** zic's EET and WET (Zones in this source, not links) differ from glibc
   2.39's only in the standard-time and UT indicators of the type before the first transition (reference 1/1,
   libniji 0/0); transitions (122) and footer equal, the readings equal. TZ-TOOLS' KNOWN_FAT list does not name
   them (its keke corpus had 593 of 598; here 591).
5. **INFORMATIONAL (TZ-TOOLS follow-up):** zdump on a name that is no zone prints the reference's line and status
   but adds `warning: zone "NAME" abbreviation "" has fewer than 3 characters`, which the reference does not print;
   with an empty name the reference warns `abbreviation "Universal" has more than 6 characters` and libniji does
   not. Stderr only; no differences row records it.
6. **INFORMATIONAL (loader):** the packaged `ld-linux-x86-64.so.2`, run as a program, accepts only `--help` and
   `--version` ("This build runs position-independent programs that need no shared objects"); `--library-path` is
   refused (exit 127). The same in 21.4 (checked on its staged libc6). spec/behaviour/dynlink.md names
   `--library-path` as an input of `ld.so PROGRAM`; no differences row or known limitation states the gap.
7. **#20 records (INFORMATIONAL):** FCLOSE-COOKIE-SEEK's three ruled differences have no row in
   docs/differences-from-glibc.md yet (its handoff lists them as RULING NEEDED).
8. **Docs (INFORMATIONAL, non-blocking):** doccheck: the same 23 lines as 21.4. RELEASE-NOTES.md's "Unreleased"
   mentions none of zic/zdump (#14), #20 or the wide-read speed-up; lintian's 2 new warnings are the missing manual
   pages of zic and zdump.
9. The run's own slips, fixed and rerun: y14r-1 ran the packaged programs through `ld.so --library-path` (finding
   6), so its libniji side never ran; y14r-2 copies them with PT_INTERP rewritten instead.

## Jobs

JOBS (the final head; this branch adds handoff/ only): mei smoke-mei-1, gate-1, startup-mei-1, kstart-mei-1,
rstart-mei-1, percall-1, sig32-1, y20-1, debs-1, plants-1, lint-1, many-1, dyn22-1, y19-2, y26-1, y26t-1, y23-1,
c19-1, y25-1, y24-1, y28-1, y25s-1, y21-1, yres-1, y23p-1, dlc-1, y27-1, eetwet-1, precheck-1; keke smoke-keke-1,
startup-keke-1, kstart-keke-1, rstart-keke-1, pkgdiff-1, stage-1, install-1 (all labels prefixed
release-snapshot-1121d-). Evidence, not named (rc 1 on known lines only): roots-1, yohane-1, y10-1, y6-1, y13-1,
y14-1 (trixie skew, findings 4-5), y14r-2 (finding 5's stderr); 21.4 side (rc 1 by design): y14-ref214-1.
SUPERSEDED: y19-1 (finding 3), y14r-1 (finding 9).

## Next step

Coordinator: tag snapshot-2026-11-21.5 from keke `~/release-staging/snapshot-20261121.5/` at cd1bbabdb when the owner
says so and tell Yohane: #14 partly fixed (zic and zdump; localedef not yet), #20 fixed as FCLOSE-COOKIE-SEEK
states. Route findings 4-5 to a TZ-TOOLS follow-up, 6 to the loader owners, 7 to FCLOSE-COOKIE-SEEK's ruling,
8 to the next docs catch-up.
