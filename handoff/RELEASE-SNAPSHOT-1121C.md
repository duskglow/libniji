COMPLETE: snapshot-ready at e4d7011e6 (serial 4: 2.39+niji20261121.4-1, staged as snapshot-20261121.4; tag
snapshot-2026-11-21.4): no regression against 21.3; #27 fixed in the package (start files carry IBT/SHSTK, one
executable LOAD, lsfd column-xmode 18/18 as stock; 21.3 fails 2); #13 partly: memcheck starts and summarises C and C++
with the loader's debug file, but with libniji's whole libc6-dbg every run still aborts in valgrind's debug reader (the
open item VALGRIND-LDSO-SYMBOLS recorded); signal 32 as ruling #7075; 8 syscalls fewer per start; plants 6/6 CAUGHT.

# RELEASE-SNAPSHOT-1121C: snapshot validation of e4d7011e6 (GitHub #27 start files, #13 valgrind, DYN-START-RING-2)

**State.** Validated head `e4d7011e669f79de280dc83207d08eb92dd3ce5f` (origin/next at the start, 10:12 UTC; "Merge
task/release-snapshot-1121b into next"; commit time 2026-11-21T10:07:39Z, epoch 1795255659); contains 8cbcc715 and
879c45b3. Branch `task/release-snapshot-1121c` = that head + handoff/ only (no product or packaging code changed).
Role **mei `~/w/s4` (Skylake-SP)**; keke = Zen 1. 21.3 = fe0899b79 (RELEASE-SNAPSHOT-1121B). Two legs.

**Landed since snapshot-2026-11-21.3** (`git log --oneline --first-parent fe0899b79..e4d7011e6`): wl-nightly-1121
(43987d4dc), crt-start-files-3 (8cbcc7151, #27), dyn-start-ring-2 (9c0e5d927), valgrind-ldso-symbols-2 (879c45b3d,
#13), train view regeneration (57970ef41), release-snapshot-1121b (e4d7011e6, handoff only).

**Version / serial.** Commit date 2026-11-21 = the day's fourth snapshot: `NIJI_DEB_SERIAL=4`; debs-1 built
`2.39+niji20261121.4-1` (7 system packages) and `niji6{,-dev}_0.0~snapshot20261121.4+e4d7011e669f`. `dpkg
--compare-versions`: `2.39+niji20261121.4-1` gt `2.39+niji20261121.3-1` and lt `2.40`;
`0.0~snapshot20261121.4+e4d7011e669f` gt `0.0~snapshot20261121.3+fe0899b79e08`: all yes.

**Scripts** (mei and keke `~/work/RELEASE-SNAPSHOT-1121C/`; local scratch `~/w/.scratch/RELEASE-SNAPSHOT-1121C/`,
`mirror/` = the edited copies): 1121B's set by copy.sh + subst.sed (names, head, epoch, serial, labels; every input dir
incl. y23p/ and dlc/); tree.tgz = `git archive e4d7011e6` (8751 files = ls-tree; sha checked on both hosts). Plants
file handoff/RELEASE-SNAPSHOT-1121C-plants.txt (1121B's; each pattern present once). New: **y27.sh** (#27), **y13.sh**
(#13), **sig32.sh** (ruling #7075), made by mknew3.sh from y21.sh's swapped-root recipe; run-new3.sh sequences them.

## Results against snapshot-2026-11-21.3 (all on e4d7011e6's products / serial-4 packages)

cmp4.sh (each check's masked PASS/FAIL/RESULT/verdict lines against 1121B's log of the same check): 0 lines differ for
19 checks; plants, debs: work-dir paths and line order only; y21: obtained-KiB amounts only (56,021 vs 56,027 and
64,168 vs 64,424 KiB; both runs PASS, room left 0). roots-2 and lint-2 diffed by hand against 1121B's -1: same.

| check | job | result | vs 21.3 |
|---|---|---|---|
| smoke lua_reload, cxx_dlclose | mei smoke-mei-1, keke smoke-keke-1 (mei's products, sha checked) | `SMOKE: RESULT bad=0` both | same |
| gate | mei gate-1 rc 0 | `GATE bad=0` (handoff-only branch) | same |
| system debs + check-system + symbols | mei debs-1 rc 0 | `check: PASS` (7 packages, 2.39+niji20261121.4-1); check-symbols PASS 4440 lines, 13 libraries | same |
| coexist debs + check | debs-1 | `check: PASS (2 packages)`; version order above | same |
| libc6-dbg | debs-1 | 19 of 19 ELF objects resolve; 19 .debug files | same |
| package comparison | keke pkgdiff-1 rc 0 | 9 packages; files unchanged but libc6-dbg 46 -> 47 (build-id directories); control: versions only; sizes: finding 5 | expected |
| #16 rerun | mei yohane-1 rc 1 | `pass=16 fail=2` (libc6-dev ships no C headers) | same |
| many objects (#9) | mei many-1 | `pass=9 fail=0` | same |
| #22 | mei dyn22-1 | `pass=3 fail=0` | same |
| #10 + #18 | mei y10-1 rc 1 | 19 PASS / 1 FAIL (DL-RUNPATH, not landed) | same |
| #19 | mei y19-1 | `pass=14 fail=0` | same |
| #26 / gcc ASan | mei y26-1, y26t-1 | `pass=20 fail=0`; judge 0 | same |
| #23 / #23 probe | mei y23-1, y23p-1 | `pass=13` / `pass=9`, fail 0 | same |
| loader corpus | mei dlc-1 | `pass=5 fail=0` | same |
| COMPAT19 | mei c19-1 | `pass=16 fail=0` | same |
| #25 dynamic / static | mei y25-1, y25s-1 | `pass=21` / `fail=0` | same |
| #24, #28 | mei y24-1, y28-1 | `pass=46`, `pass=10`, fail 0 | same |
| #6 | mei y6-1 rc 1 | `pass=79 fail=2` (WINDOWS-1252, windows-1251) | same |
| #21 | mei y21-1 | `pass=7 fail=0` | same |
| libresolv | mei yres-1 | `pass=36 fail=0` | same |
| empty root / swapped trixie root | mei roots-2 | `root: RESULT pass=21 fail=0`; `verdict: commands=65 same=63 known=2 bad=0` | same |
| libc6-dbg + gdb | roots-2 (its only rc 1) | `dbg: RESULT pass=8 fail=1` (known) | same |
| lintian | mei lint-2 rc 0 | errors=10 warnings=46; proofs pass=21 fail=0, trixie and excalibur | same |
| **#27** | mei y27-2; 21.3: y27-ref213-3 | `pass=17 fail=0`; table below | **21.3: pass=1 fail=16** |
| **#13** | mei y13-1 rc 1; 21.3: y13-ref213-2 | `pass=6 fail=3` (the 3 = whole libc6-dbg); table below | **21.3: pass=1 fail=8** |
| **signal 32 (#7075)** | mei sig32-2 | `fail=0`; table below | **changed by design** |
| plants | mei plants-1 rc 0 | 6 plants: 6 CAUGHT, 0 SURVIVED, 0 WARN | same |
| stage | keke stage-1 rc 0 | 9 assets + SHA256SUMS + MANIFEST; `sha256sum -c` 9 OK | - |
| install instructions | keke install-1 rc 0 | `inst: RESULT fails=0` | same |
| docs | doccheck.py (1120F's) on the head | 23 problems, the same 23 lines as 21.3 | same |

**Staged** (keke `~/release-staging/snapshot-20261121.4/`): libc6, libc6-dbg, libc6-dev, libc-bin, libc-dev-bin
(`_2.39+niji20261121.4-1_amd64.deb`), libc-l10n, locales (`_2.39+niji20261121.4-1_all.deb`), niji6, niji6-dev
(`_0.0.snapshot20261121.4+e4d7011e669f_amd64.deb`, `~` written as `.` for GitHub), SHA256SUMS, MANIFEST.

## #27: start files (y27; swapped trixie root; stock = trixie glibc 2.41's libc6-dev)

The facts CRT-START-FILES-3 states (D-X.144), read from the root's installed start files, and crt_maps.c (the
repository's probe) linked by the root's gcc in four modes. util-linux: trixie's upstream tarball
util-linux_2.41.5.orig.tar.xz (Debian's patches not applied: the closest equivalent to Yohane's build), lsfd and
test_mkfds built statically against util-linux's own libraries on the stock root, relinked after the swap (0 objects
recompiled), `tests/run.sh lsfd/column-xmode`.

| judged | head e4d7011e6 | 21.3 |
|---|---|---|
| crt1, Scrt1, rcrt1, gcrt1, grcrt1: `_start` (endbr64), `__data_start`, weak `data_start`, `_IO_stdin_used` = 0x20001, notes IBT SHSTK x86-64-baseline ABI 3.2.0, max alignment 64 | PASS x5 (1,816 / 1,816 / 1,784 / 2,744 / 2,712 B) | FAIL x5 (no data symbols, no endbr64, no notes, alignment 65,536; 67,104-68,128 B) |
| crti.o, crtn.o carry IBT and SHSTK | PASS x2 | FAIL x2 |
| pie, nopie, static, static-pie: no `-z cet-report` warning names a start file | PASS x4 | FAIL x4 (3 start files each) |
| pie, nopie: one executable LOAD, page-aligned, IBT and SHSTK, one r-xp mapping | PASS x2 | FAIL x2 (2 LOADs, 0x10000, no feature) |
| test_mkfds: one executable LOAD; one r-xp mapping at run time (stock 1, 1) | PASS x2 | FAIL (2, 2) |
| lsfd column-xmode (stock: 18 of 18) | **PASS: 18 of 18** | **FAIL: MODE-x-bit, XMODE-x-bit (the `r-x` row printed twice)** |

## #13: valgrind (y13; trixie's valgrind package, memcheck; swapped root; stock = trixie's glibc + its libc6-dbg)

| judged | head e4d7011e6 | 21.3 |
|---|---|---|
| the loader's debug file (by build id): LOCAL FUNC `strcmp` (23 B) and `strlen` (966 B) | PASS | FAIL (only `niji_machine_strlen`) |
| memcheck with only the loader's debug file: starts, the program runs, the usual summary (stock in brackets) | PASS: /bin/true 1 error (0); bad.c 2 errors, the invalid read at its line (1); C++ throw/catch 1 error, 32 bytes lost at the throw (1, 32) | FAIL: "Fatal error at startup: a function redirection" x3 |
| memcheck with libniji's whole libc6-dbg | **FAIL x3: `m_debuginfo/readelf.c:3013 (find_rodata): Assertion 'di->rodata_size == sh_size' failed`** | FAIL x3 (the redirection error) |
| shown: the installed loader's `.dynsym` strcmp/strlen rows; `.symtab` | 0; none | 0; none |

## DYN-START-RING-2: signal 32 (sig32; raw rt_sigaction / kill system calls; program linked as startup.sh links)

| process | before the first thread | after one thread created and joined |
|---|---|---|
| head e4d7011e6 | disposition default; ended by 32 (status 160): **PASS** | handler installed; alive (status 0): **PASS** |
| 21.3 | handler; alive (status 0) | handler; alive |
| glibc 2.39 (host; shown) | default; ended by 32 | default; ended by 32 |

As the differences row "Reserved real-time signals" and DYN-START-RING's handoff (771ed7154) record: the handler is
installed by the first thread creation; a raw 32 before it ends the process.

## Start-up (execcount, ASLR off, median of 101; startup-*-1, kstart-*-1, rstart-*-1)

User instructions / page faults (21.3 = 1121B's products, rerun here: they reproduce 1121B's numbers, Zen 1 within 1):

| program | class | glibc 2.39 | snapshot-2026-11-21.3 | e4d7011e6 | delta |
|---|---|---|---|---|---|
| empty main | Skylake-SP | 107,752 / 49 | 741,148 / 52 | 741,532 / 51 | +384 / -1 |
| 3 libraries | Skylake-SP | 143,094 / 65 | 996,715 / 67 | 984,518 / 65 | -12,197 / -2 |
| clock_gettime | Skylake-SP | 109,862 / 50 | 775,158 / 56 | 763,624 / 54 | -11,534 / -2 |
| empty main | Zen 1 | 101,533 / 48 | 740,915 / 52 | 741,300 / 51 | +385 / -1 |
| 3 libraries | Zen 1 | 136,734 / 64 | 996,468 / 67 | 984,271 / 65 | -12,197 / -2 |
| clock_gettime | Zen 1 | 103,672 / 50 | 774,955 / 57 | 763,421 / 55 | -11,534 / -2 |

Kernel instructions / syscalls (kernel counts move by up to ~50k between runs of one tree; syscalls are exact):

| program | class | glibc | 21.3 | e4d7011e6 |
|---|---|---|---|---|
| empty main | Skylake-SP | 688,349 / 46 | 780,066 / 37 | 722,312 / 29 |
| 3 libraries | Skylake-SP | 987,184 / 72 | 1,205,085 / 80 | 1,119,331 / 59 |
| clock_gettime | Skylake-SP | 694,936 / 46 | 814,635 / 37 | 754,298 / 29 |
| empty main | Zen 1 | 698,554 / 42 | 796,090 / 36 | 737,143 / 28 |
| 3 libraries | Zen 1 | 999,834 / 68 | 1,216,337 / 79 | 1,143,743 / 58 |
| clock_gettime | Zen 1 | 709,881 / 42 | 832,937 / 36 | 771,019 / 28 |

Per call (clock_gettime, Skylake-SP): 21.3 `mmap 12, openat 6, munmap 4, mprotect 2, close 2, arch_prctl 2, uname,
set_tid_address, set_robust_list, rt_sigaction, rseq, read, fstat, exit_group, execve 1` (37); head `mmap 11, openat 5,
mprotect 2, arch_prctl 2, uname, set_tid_address, set_robust_list, rseq, munmap, fstat, exit_group, execve, close 1`
(29): no `rt_sigaction` (the lazy handler), one file fewer opened and read, 3 `munmap` fewer. The libresolv pair (res0
needs libresolv.so.2; pie0 empty), user instructions, then res0's kernel instructions / syscalls:

| class | | glibc 2.39 | 21.3 | e4d7011e6 |
|---|---|---|---|---|
| Skylake-SP | pie0 / res0 user | 107,645 / 169,020 | 741,102 / 920,120 | 741,500 / 920,426 |
| Skylake-SP | res0 - pie0 | +61,375 | +179,018 | +178,926 |
| Skylake-SP | res0 kernel / syscalls | 820,902 / 58 | 976,133 / 55 | 926,284 / 42 |
| Zen 1 | pie0 / res0 user | 101,481 / 162,716 | 740,819 / 919,580 | 741,217 / 919,886 |
| Zen 1 | res0 - pie0 | +61,235 | +178,761 | +178,669 |
| Zen 1 | res0 kernel / syscalls | 842,600 / 54 | 992,963 / 53 | 938,886 / 40 |

## Findings

1. No regression in any check against 21.3: every compared check's verdict lines are the same.
2. **#27 fixed in the package:** every start-file fact of D-X.144 holds in the swapped root; Yohane's lsfd
   column-xmode case passes 18 of 18 as stock (21.3 prints the `r-x` row twice and fails 2).
3. **#13 partly fixed (INFORMATIONAL, for what Yohane is told):** memcheck starts and reports its usual summary for C
   and C++ programs when only the loader's debug file is installed; with libniji's whole libc6-dbg installed (trixie's
   valgrind package pulls libc6-dbg in, per VALGRIND-LDSO-SYMBOLS's handoff) every run aborts with valgrind's
   `find_rodata` assertion, as that handoff records under "Not fixed" item 1 (libc.so.6's `.rodata1`/`.rodata` pair).
   memcheck also reports one error more than stock on /bin/true and bad.c (its "Not fixed" item 3).
4. **DYN-START-RING-2 in the package:** 8 syscalls fewer per start (no `rt_sigaction`, one file not opened), kernel
   instructions -7% to -11%, faults -1/-2; user instructions -1.2% for 3 libraries and clock_gettime, **+384 / +385
   (+0.05%) for the empty main** on both classes (INFORMATIONAL; not chased, no product change in this run).
5. **Package sizes (INFORMATIONAL):** the start files lose their 64 KiB padding (crt1 variants -65 KB each; libc6-dev
   Installed-Size -189 KiB) and so do the dynamic programs in libc-bin (getent, iconv -175,880 B, locale -163,848 B;
   libc-bin Installed-Size -379 KiB). Grown, not explained here: the static-pie getconf (+94,448 B; `.text` +57,600)
   and ldd (+33,008 B; `.text` +8,384), and libc.a (+134,390 B).
6. **Docs (INFORMATIONAL, non-blocking):** doccheck: the same 23 lines as 21.3. New gap beyond 21.3's: the release
   notes' "Unreleased" has nothing for #27, #13 or the lazy signal-32 handler (the differences row is current);
   DOCS-CATCHUP-1121 is queued for the notes.
7. The run's own slips, all fixed and rerun: sig32.sh ran the programs without the products' library path (sig32-1,
   status 127); the y27 root lacked xz-utils for the .tar.xz (y27-ref213-1); y13's build-id parse took readelf's
   note type (y13-ref213-1; fixed before y13-1 ran); crtsum.sh read `_IO_stdin_used` from `.rodata` while both stock
   and libniji keep it in `.rodata.cst4` (y27-1, y27-ref213-2); leg 1 copied tree.tgz to mei without unpacking it
   (lint-1, roots-1: "cannot open .../tree/tools/deb/...").

## Jobs

JOBS (the final head; this branch adds handoff/ only; rc 0 each): mei smoke-mei-1, gate-1, startup-mei-1, kstart-mei-1,
rstart-mei-1, debs-1, plants-1, lint-2, many-1, dyn22-1, y19-1, y26-1, y26t-1, y23-1, c19-1, y25-1, y24-1, y28-1,
y25s-1, y21-1, yres-1, y23p-1, dlc-1, y27-2, sig32-2, precheck-1; keke smoke-keke-1, startup-keke-1, kstart-keke-1,
rstart-keke-1, pkgdiff-1, stage-1, install-1 (all labels prefixed release-snapshot-1121c-). Evidence, not named (rc 1
on known lines only): roots-2, yohane-1, y10-1, y6-1, y13-1 (the whole-libc6-dbg rows); 21.3 side (rc 1 by design):
y27-ref213-3, y13-ref213-2. SUPERSEDED: sig32-1, y27-ref213-1, y27-ref213-2, y13-ref213-1, y27-1, lint-1, roots-1
(finding 7).

## Next step

Coordinator: tag snapshot-2026-11-21.4 from keke `~/release-staging/snapshot-20261121.4/` at e4d7011e6 when the owner
says so and tell Yohane: #27 fixed (their lsfd case passes); #13: valgrind starts with the loader's debug file, but
not yet with libniji's whole libc6-dbg (finding 3; the `.rodata1` layout item needs a story). Route finding 5's
static-pie growth to the start-up / layout owners, finding 6 to DOCS-CATCHUP-1121.
