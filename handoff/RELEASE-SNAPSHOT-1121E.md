COMPLETE: snapshot-ready at 97bf3542f (serial 6 of 2026-11-21: 2.39+niji20261121.6-1, staged as
snapshot-20261121.6; tag snapshot-2026-11-21.6): #13 fixed (memcheck with the whole libc6-dbg gives stock's error
summaries for /bin/true, a C, a C++ and a leaking program; 21.5 aborted); #6 CJK fixed (BIG5, EUC-KR, CP932, CP936,
GB18030 and 21 aliases open and round-trip byte for byte as glibc 2.39 and trixie; 21.5 failed 13 of 31 rows);
DL-PROCESS-OBJECTS-2 in (600 libraries load; 21.5 stopped at 510); every other check as 21.5; plants 6/6 CAUGHT.
Finding 1: start-up +1,408 user instructions and +5 page faults (+6 with 3 libraries) on every program, both classes.

# RELEASE-SNAPSHOT-1121E: snapshot validation of 97bf3542f (#6 CJK, #13 valgrind with libc6-dbg)

**State.** Validated head `97bf3542f9276ec1b622c3c793a8bb3cd63735cd` (origin/next at the start, 18:41 UTC; "Merge
task/release-snapshot-1121d into next"; commit time 2026-11-21T18:13:38Z, epoch 1795284818). Contains 62c6f2409
(ICONV-CJK, #6) and 123ad2548 (VALGRIND-DBG-RODATA, #13). Branch `task/release-snapshot-1121e` = that head +
handoff/ only (no product or packaging code changed). Role **mei `~/w/s1` (Skylake-SP)**; keke = Zen 1. 21.5 =
cd1bbabdb (RELEASE-SNAPSHOT-1121D). One leg.

**Landed since snapshot-2026-11-21.5** (`git log --oneline --first-parent cd1bbabdb..97bf3542f`): headers-winsize-twice
(ed11ecc05), iconv-cjk (62c6f2409, #6), valgrind-dbg-rodata (123ad2548, #13), cutover-residue-audit (b655dfeec),
cutover-residue-audit-b (59294ffd9), view regen (4aff6705c), dl-process-objects-2 (806500ce7), forced-unwind-ref-race
(af5d17754), nscache-bootid-tmpfs-2 (0c7f985ad), limits-runtime-small (f8e85a948), view regen (d5764940d),
release-snapshot-1121d (97bf3542f, handoff).

**Not in the head:** COMPAT19-LDSO-COST-2 (start-up tables below are without it). WINDOWS-1252 / WINDOWS-1251: not
built in (queued; y6 and y6c show them unopened, as 21.5).

**Version / serial.** Commit date 2026-11-21 = the day's sixth snapshot: `NIJI_DEB_SERIAL=6`; debs-1 built
`2.39+niji20261121.6-1` (7 system packages) and `niji6{,-dev}_0.0~snapshot20261121.6+97bf3542f927`. `dpkg
--compare-versions`: `2.39+niji20261121.6-1` gt `2.39+niji20261121.5-1` and lt `2.40`;
`0.0~snapshot20261121.6+97bf3542f927` gt `0.0~snapshot20261121.5+cd1bbabdba59`: all yes.

**Scripts** (mei and keke `~/work/RELEASE-SNAPSHOT-1121E/`; local copies in the run's scratch `mirror/`): 1121D's set
by copy.sh + subst.sed (every input dir incl. y14/ y20/; head, epoch, serial, labels; comparison side 21.5 = p215,
head = p216); tree.tgz = `git archive 97bf3542f` (8864 files = ls-tree; sha checked on both hosts); run-keke.sh
from keke's 1121D copy by the same subst.sed. Plants file handoff/RELEASE-SNAPSHOT-1121E-plants.txt (1121D's; each
pattern present once). New: **y6c.sh** (#6 CJK, made by mknew5.sh from y6.sh's swapped-trixie-root recipe; probe
y6c/cjkrt.c; judged against trixie's stock and the host's glibc 2.39), **dpo600.sh** (DL-PROCESS-OBJECTS-2's
count.sh recipe; count.c and inert_soname.py copied unchanged into dpo/), **run-new5.sh** (replaces run-new.sh and
run-new4.sh), relsites.py (finding 1 probe). Not rerun: percall (1121D's per-call rows were WIDE-READ-FAST's
evidence; no stdio story in the landed list; NOT VERIFIED by a count).

## Results against snapshot-2026-11-21.5 (all on 97bf3542f's products / serial-6 packages)

cmp4.sh (masked PASS/FAIL/RESULT/verdict lines against 1121D's log of the same check): 0 lines differ for roots,
yohane, many, dyn22, y10, y26, y26t, c19, y23, y25, y24, y28, y6, y25s, y23p, y27, y20, y21, yres, dlc, y14, lint,
smoke-mei, gate, debs (mei) and smoke-keke, install (keke); y19-1 against 1121D's y19-2: 0; y14r-1 against y14r-2:
the same verdict lines (md5); sig32: only the set labels (p215/p216). Differ: y13 (by design, below); plants (one
CAUGHT line of rs1121e-cxx-needs-libstdcxx differs past the compared 200 columns, not examined; 6 CAUGHT both).

| check | job | result | vs 21.5 |
|---|---|---|---|
| smoke lua_reload, cxx_dlclose | mei smoke-mei-1, keke smoke-keke-1 | `SMOKE: RESULT bad=0` both | same |
| gate | mei gate-1 rc 0 | `GATE bad=0` (handoff-only branch), peak 2.5G | same |
| system debs + check-system + symbols | mei debs-1 rc 0 | `check: PASS`; check-symbols PASS 4440 lines, 13 libraries | same |
| libc6-dbg | debs-1 | 21 of 21 ELF objects resolve; 21 .debug files | same |
| package comparison | keke pkgdiff-1 rc 0 | file lists equal (libc6-dbg 51 -> 50 entries: build-id directories only); sizes: finding 3 | expected |
| #16 / #9 / #22 | mei yohane-1 rc 1, many-1, dyn22-1 | `pass=16 fail=2` (known), `pass=9`, `pass=3` | same |
| #10 + #18 / #19 | mei y10-1 rc 1, y19-1 | 19 PASS / 1 FAIL (DL-RUNPATH); `pass=14 fail=0` | same |
| #26, #23, #25, #24, #28, #21, #27, libresolv | y26-1, y26t-1, y23-1, y23p-1, y25-1, y25s-1, y24-1, y28-1, y21-1, y27-1, yres-1 | all fail=0 | same |
| loader corpus / COMPAT19 | mei dlc-1, c19-1 | `pass=5`, `pass=16`, fail 0 | same |
| **600 libraries** (DL-PROCESS-OBJECTS-2) | mei dpo600-1 | head and glibc load 600 (604 objects); 21.5: `libc510.so: cannot load more objects` | **new** |
| **#13** | mei y13-1 rc 0 | `pass=9 fail=0` | **21.5: pass=6 fail=3** |
| **#6 CJK** | mei y6c-1 rc 0; 21.5 side y6c-ref215-1 rc 1 | `pass=32 fail=0`, glibc 2.39 same=26 differ=0 | **21.5: fail=13** |
| #6 (1120F's probe) | mei y6-1 rc 1 | `pass=79 fail=2` (WINDOWS-1252, windows-1251) | same |
| #20 / signal 32 | mei y20-1, sig32-1 | `RESULT fail=0` (37 rows; 0 rows differ from 21.5); `fail=0` | same |
| #14 | mei y14-1 rc 1, y14r-1 rc 1, eetwet-1 | as 1121D (trixie skew; zdump stderr, its finding 5) | same |
| roots / dbg+gdb | mei roots-1 rc 1 | `pass=21`; `verdict: same=63 known=2 bad=0`; `dbg: pass=8 fail=1` (known) | same |
| lintian | mei lint-1 rc 0 | errors=10 warnings=48; proofs pass=21 fail=0 | same |
| plants | mei plants-1 rc 0 | 6 plants: 6 CAUGHT, 0 SURVIVED, 0 WARN | same |
| stage / install | keke stage-1, install-1 rc 0 | 9 assets + SHA256SUMS + MANIFEST, `sha256sum -c` 9 OK; `inst: RESULT fails=0` | - |
| docs | 1121D's doccheck.py on the head | 23 problems, the same 23 lines | same |

**Staged** (keke `~/release-staging/snapshot-20261121.6/`): libc6, libc6-dbg, libc6-dev, libc-bin, libc-dev-bin
(`_2.39+niji20261121.6-1_amd64.deb`), libc-l10n, locales (`_2.39+niji20261121.6-1_all.deb`), niji6, niji6-dev
(`_0.0.snapshot20261121.6+97bf3542f927_amd64.deb`, `~` written as `.` for GitHub), SHA256SUMS, MANIFEST.

## #13: valgrind with the whole libc6-dbg (y13-1, swapped trixie root, trixie's valgrind)

| run (memcheck --leak-check=full) | stock | head, whole libc6-dbg | 21.5, whole libc6-dbg |
|---|---|---|---|
| /bin/true | 0 errors from 0 contexts | 0 from 0, rc 0 | aborted (assertion), no summary |
| bad.c (C: read past a 16-byte block) | 1 from 1; invalid read at bad.c:11 | 1 from 1; the same record | aborted |
| cxx.cc (C++: throw/catch, leaks 32 B) | 1 from 1; 32 B definitely lost | 1 from 1; 32 B definitely lost | aborted |

The leak record (the leaking program is cxx.cc) is the same as stock: `operator new[]` (vg_replace_malloc.c:729) <-
`keep(int)` (cxx.cc:11) <- `main` (cxx.cc:27); stock's stack also ends at main, so it does not stop inside libc.
With only the loader's debug file (ldonly) bad.c now gives stock's `1 errors from 1 contexts` (21.5: 2 from 2).
Finding 2 (still reachable).

## #6: CJK (y6c-1 on the head, y6c-ref215-1 on 21.5)

cjkrt (iconv_open both ways, a sample text per set UTF-8 -> set -> UTF-8, encoded bytes in hex) on 26 names: BIG5
BIG-5 BIG-FIVE BIGFIVE CN-BIG5 CP950 big5 / EUC-KR EUCKR CSEUCKR OSF0004000A euc-kr / CP932 MS932 WINDOWS-31J
SJIS-OPEN SJIS-WIN cp932 / CP936 GBK MS936 WINDOWS-936 GB13000 gbk / GB18030 gb18030. Samples: traditional Han and
fullwidth forms (BIG5); Hangul and hanja (EUC-KR); kanji, katakana, half-width kana, NEC row 13 (①, Ⅲ), ～ ￢ ∵
(CP932); simplified Han and € (CP936, 0x80); Han, Mongolian, U+1F600 and € (GB18030, four-byte sequences).

| | head | 21.5 |
|---|---|---|
| cjkrt rows as trixie stock (glibc 2.41) | 26 of 26 | 14 of 26: BIG-FIVE BIGFIVE CN-BIG5 CP950 CSEUCKR OSF0004000A MS932 SJIS-OPEN SJIS-WIN GB13000 not opened (EINVAL); GB18030 and gb18030 EILSEQ on the four-byte text |
| cjkrt rows as glibc 2.39 (host, same source) | 26 of 26 (stock 2.41 and 2.39 identical) | 14 of 26 |
| iconv program round trip, 5 sets | 5 of 5 as stock (bytes and md5) | GB18030 fails (rc 1, 34 of 48 bytes) |
| `iconv -l` | all 26 names listed, as stock; 310 names in all (stock 1180, 284 in both) | - |
| WINDOWS-1252 / WINDOWS-1251 (queued) | not opened (EINVAL), not listed; stock and 2.39 round-trip | same |

## Start-up (execcount, ASLR off, median of 101; startup-*-1, kstart-*-1, rstart-*-1)

21.5 = 1121D's products rerun here (p215): they reproduce 1121D's numbers exactly. User instructions / page faults:

| program | class | glibc 2.39 | snapshot-2026-11-21.5 | 97bf3542f | delta |
|---|---|---|---|---|---|
| empty main | Skylake-SP | 107,752 / 49 | 741,532 / 51 | 742,940 / 56 | +1,408 / +5 |
| 3 libraries | Skylake-SP | 143,094 / 65 | 984,518 / 65 | 985,927 / 71 | +1,409 / +6 |
| clock_gettime | Skylake-SP | 109,862 / 50 | 763,624 / 54 | 765,032 / 59 | +1,408 / +5 |
| empty main | Zen 1 | 101,533 / 48 | 741,300 / 51 | 742,708 / 56 | +1,408 / +5 |
| 3 libraries | Zen 1 | 136,734 / 64 | 984,271 / 65 | 985,680 / 71 | +1,409 / +6 |
| clock_gettime | Zen 1 | 103,672 / 50 | 763,421 / 55 | 764,829 / 60 | +1,408 / +5 |

Kernel instructions / syscalls (kernel counts move by up to ~50k between runs; syscalls exact and unchanged):

| program | class | glibc | 21.5 | 97bf3542f |
|---|---|---|---|---|
| empty main | Skylake-SP | 684,577 / 46 | 718,743 / 29 | 738,195 / 29 |
| 3 libraries | Skylake-SP | 979,079 / 72 | 1,113,359 / 59 | 1,140,280 / 59 |
| clock_gettime | Skylake-SP | 687,331 / 46 | 748,690 / 29 | 770,183 / 29 |
| empty main | Zen 1 | 734,784 / 42 | 788,253 / 28 | 760,299 / 28 |
| 3 libraries | Zen 1 | 1,033,141 / 68 | 1,200,598 / 58 | 1,237,062 / 58 |
| clock_gettime | Zen 1 | 726,963 / 42 | 773,496 / 28 | 849,046 / 28 |

libresolv pair (res0 needs libresolv.so.2; pie0 empty), user instructions / faults, then res0 kernel / syscalls:

| class | | glibc 2.39 | 21.5 | 97bf3542f |
|---|---|---|---|---|
| Skylake-SP | pie0 / res0 user | 107,645 / 169,020 | 741,500 / 920,426 | 742,908 / 921,834 |
| Skylake-SP | res0 faults; kernel / syscalls | 55; 824,739 / 58 | 56; 919,448 / 42 | 61; 939,511 / 42 |
| Zen 1 | pie0 / res0 user | 101,481 / 162,716 | 741,217 / 919,887 | 742,626 / 921,295 |
| Zen 1 | res0 faults; kernel / syscalls | 54; 871,374 / 54 | 56; 990,925 / 40 | 61; 959,260 / 40 |

## Findings

1. **Start-up cost rose (INFORMATIONAL, route to the start-up owners; not blocking in my judgement):** +1,408
   user instructions (+0.19%) and +5 page faults (+6 with 3 libraries) per process on both classes; syscalls
   unchanged. libc.so.6 has 71 more dynamic relocations (1,790 -> 1,861, all RELATIVE): 40 sit in the five
   ICONV-CJK table descriptors (`niji::iconv::cjk_tables::{BIG5,EUC_KR,CP932,CP936,GB18030}`, 8 each), 31 in
   unnamed sites (relsites.py); at ~20 instructions per relocation that is about the +1,408 (estimate). The
   relocated pages stay at 13, so the faults come from elsewhere; not attributed (layout: libc.so.6's read-only
   segment +448,656 B, ld.so text +14,976 B, libc RW memsz -45,056 B). Possible fix-forward: keep the descriptors'
   pointers out of relocated data (offsets instead of pointers) - for ICONV-CJK's owner to judge.
2. **#13 (INFORMATIONAL):** besides equal error summaries, memcheck under libniji reports `still reachable: 4,104
   bytes in 1 blocks` at exit (bad.c and cxx.cc) where stock reports 0; 21.5's ldonly run had the same block, so
   it is not new. Route to VALGRIND-LOADER-VIEW or a follow-up.
3. **Sizes (INFORMATIONAL):** libc6 +174,920 B (Installed-Size 9047 -> 9503 KiB), libc6-dev +171,456 B, libc-bin
   +165,952 B (2035 -> 2547 KiB): in libc-bin only `usr/bin/getconf` (+459,136 B, about the CJK tables' 447,668 B)
   and `usr/bin/ldd` (+65,920 B) grew. Whether getconf needs the iconv tables is for ICONV-CJK's owner.
4. **Docs (INFORMATIONAL, non-blocking):** doccheck: the same 23 lines as 21.5. RELEASE-NOTES.md mentions none of
   #6 CJK, #13, #14 or #20.
5. The run's own slip: run-new5.sh chained eetwet-1 after y14r-1 with `&&`; y14r's known rc 1 skipped it, so
   eetwet-1 was started by hand after y14r-1 (same input).

## Jobs

JOBS (the final head; this branch adds handoff/ only): mei smoke-mei-1, gate-1, startup-mei-1, kstart-mei-1,
rstart-mei-1, sig32-1, y20-1, dpo600-1, debs-1, plants-1, lint-1, many-1, dyn22-1, y19-1, y26-1, y26t-1, y23-1, c19-1,
y25-1, y24-1, y28-1, y25s-1, y21-1, yres-1, y23p-1, dlc-1, y27-1, y13-1, y6c-1, eetwet-1, precheck-1; keke
smoke-keke-1, startup-keke-1, kstart-keke-1, rstart-keke-1, pkgdiff-1, stage-1, install-1 (all labels prefixed
release-snapshot-1121e-). Evidence, not named (rc 1 on known lines only): roots-1, yohane-1, y10-1, y6-1, y14-1,
y14r-1; 21.5 side (rc 1 by design): y6c-ref215-1.

## Next step

Coordinator: tag snapshot-2026-11-21.6 from keke `~/release-staging/snapshot-20261121.6/` at 97bf3542f when the
owner says so and tell Yohane: #13 fixed (valgrind with the whole libc6-dbg), #6 CJK sets fixed (BIG5, EUC-KR,
CP932, CP936, GB18030 and aliases; WINDOWS-1252/1251 still queued). Route finding 1 (and 3) to ICONV-CJK's owner /
the start-up owners, 2 to VALGRIND-LOADER-VIEW, 4 to the next docs catch-up.
