COMPLETE: snapshot-ready at 2c64fde8a (serial 7 of 2026-11-21: 2.39+niji20261121.7-1, staged as
snapshot-20261121.7): #13 rest fixed (memcheck with the whole libc6-dbg: ls /, ls -l /etc, find /etc -name passwd,
sort /etc/passwd and an all-string-functions program give 0 errors as stock, 21.6 gave 12, 14, 71, 0, 12; the strlen
overread is still reported); getaddrinfo_a answers all 25,000 requests and delivers all 5,000 signals under load
(21.6 too: its "~2% lost" was the test's deadline); DL-RUNPATH $ORIGIN bare-name dlopen as stock (21.6 not); ICONV-REST
in (CP949, UHC, WINDOWS-1252, windows-1251, KOI8R, ISO885915 open and round-trip as stock; 21.6 failed 6 of 13 rows);
COMPAT19-LDSO-COST-2's start-up gain shows (-16.5k user instructions per empty main); every other check as 21.6;
plants 6/6 CAUGHT. Finding 1: +1 page fault per program and ~0.7k of COMPAT19's gain absorbed by ICONV-REST's
relocations (fix-forward ICONV-CJK-STARTUP, queued; not in the head).

# RELEASE-SNAPSHOT-1121F: snapshot validation of 2c64fde8a (#13 rest, getaddrinfo_a, ICONV-REST, DL-RUNPATH-4)

**State.** Validated head `2c64fde8a98df721ad7ee082d8ba8d7d073172d9` (origin/next at the start, 00:38 UTC;
"Regenerate train-owned views"; commit time 2026-11-21T23:47:05Z, epoch 1795304825). Branch
`task/release-snapshot-1121f` = that head + handoff/ only (no product or packaging code changed). Role **keke
`~/w/n3` (Zen 1)**; mei = Skylake-SP (test jobs only, on keke's products and .debs, sha256 checked on arrival).
21.6 = 97bf3542f (RELEASE-SNAPSHOT-1121E). Two legs (refresh after the start-up counts).

**Landed since snapshot-2026-11-21.6** (`git log --first-parent 97bf3542f..2c64fde8a`): release-snapshot-1121e
(83f91cb05), compat19-ldso-cost-2 (bbcc073c5), view regen (4f32aa946), wl-count-honest-cheap (228dcf3b6), view regen
(0299fd6e1), dl-runpath-4 (69e37399f), startup-structure-review (b1cd6f3e1), valgrind-string-redirect (e64dbbc30),
affected-coverage (ec977d861), view regen (e72318002), gai-mass-loss (6094e53e9), iconv-rest (ed6d48230), view regen
(2c64fde8a). All four required merges (VALGRIND-STRING-REDIRECT, GAI-MASS-LOSS, COMPAT19-LDSO-COST-2, DL-RUNPATH-4)
are in.

**Not in the head:** ICONV-CJK-STARTUP (no merge of it since 21.6): start-up is not back to 21.5's fault level.

**Version / serial.** Commit date 2026-11-21 = the day's seventh snapshot: `NIJI_DEB_SERIAL=7`; debs-1 built
`2.39+niji20261121.7-1` (7 system packages) and `niji6{,-dev}_0.0~snapshot20261121.7+2c64fde8a98d`. `dpkg
--compare-versions`: `2.39+niji20261121.7-1` gt `2.39+niji20261121.6-1`; `0.0~snapshot20261121.7+2c64fde8a98d` gt
`0.0~snapshot20261121.6+97bf3542f927`: both yes.

**Scripts** (mei and keke `~/work/RELEASE-SNAPSHOT-1121F/`, identical; local copies in this run's scratch): 1121E's
set by copy.sh + subst.sed (head, epoch, serial 7, labels, p216 = 21.6 / p217 = head, role `w/s1` -> `w/n3`), tree.tgz
= `git archive 2c64fde8a` (8924 files = ls-tree; sha checked on both hosts). Plants file
handoff/RELEASE-SNAPSHOT-1121F-plants.txt (1121E's six, each pattern present once in the head; results appended).
New this run: run-keke.sh / run-mei.sh (per-host sequencing; the .debs relayed keke -> mei by tar, debs.sha last and
checked), run-gai.sh + gaimass.sh, rootrun.sh (VALGRIND-STRING-REDIRECT's root recipe, probe dir as a parameter),
y13s.sh (#13 rest; probe = that story's vs/ unchanged), drp.sh (probe drp/: DL-RUNPATH-4, plain and setgid), y6r.sh
(ICONV-REST six sets, made by mky6r.py from y6c; every row judged; y6r/post.sh line 7 keeps y6c's stale "not judged"
comment, the code judges every row), cmp5.sh (cmp4.sh with the old log directory as a parameter, for 1121E's gate,
plants and debs logs written on mei). Not rerun: percall (no stdio story landed; NOT VERIFIED by a count).

## Results against snapshot-2026-11-21.6 (all on 2c64fde8a's products / serial-7 packages)

cmp4.sh / cmp5.sh (masked PASS/FAIL/RESULT/verdict lines against 1121E's log of the same check): 0 lines differ for
roots, lint, yohane, many, dyn22, y19, y26, y26t, c19, y23, y25, y24, y28, y25s, y21, yres, y23p, dlc, y27, y13, y14r,
y6c, eetwet, startup/kstart/rstart (both), install, pkgdiff, gate, debs. Differ, all explained: y10 (the DL-RUNPATH
row now PASS: `bare-name dlopen ... ok`, `fail=1` -> `fail=0`), y6 (WINDOWS-1252 and windows-1251 now open: `fail=2` ->
`fail=0`), y14 (one line: America/Inuvik `(v1,v2)` -> `(v1)`; trixie's tzdata moved 2026c -> 2026f between the runs,
input skew, no tz code landed), sig32 and dpo600 (set labels p216/p217 only), smoke-mei / smoke-keke (the layout
verdict cache lines move with the host that built the products), plants (log paths and which copy ran a plant; 6
CAUGHT both).

| check | job | result | vs 21.6 |
|---|---|---|---|
| smoke lua_reload, cxx_dlclose | keke smoke-keke-1 (build), mei smoke-mei-1 | `SMOKE: RESULT bad=0` both | same |
| gate | keke gate-1 rc 0 | clippy all-features, xtask clippy, static, records check, affected: `GATE bad=0` | same |
| system debs + check + symbols | keke debs-1 rc 0 | `check: PASS (7 packages)`; check-symbols PASS 4440 lines, 13 libraries; `DEBS bad=0` | same |
| libc6-dbg | debs-1 | 21 of 21 ELF objects resolve; 21 .debug files | same |
| package comparison | keke pkgdiff-1 rc 0 | file lists equal (libc6-dbg 50 -> 50: build-id names only); sizes: finding 3 | expected |
| #16 / #9 / #22 | mei yohane-1 rc 1, many-1, dyn22-1 | `pass=16 fail=2` (known), `pass=9`, `pass=3` | same |
| **#10 + #18** / #19 | mei y10-1 rc 0, y19-1 | **20 PASS / 0 FAIL** (DL-RUNPATH row now passes); `pass=14 fail=0` | **21.6: 1 FAIL** |
| #26, #23, #25, #24, #28, #21, #27, libresolv | y26-1, y26t-1, y23-1, y23p-1, y25-1, y25s-1, y24-1, y28-1, y21-1, y27-1, yres-1 | all fail=0 | same |
| loader corpus / COMPAT19 / 600 libraries | mei dlc-1, c19-1, dpo600-1 | `pass=5`, `pass=16`; 600 libraries load (604 objects) | same |
| #13 (1121E's probe) | mei y13-1 rc 0 | `pass=9 fail=0` | same |
| **#13 rest** | mei y13s-1 rc 0; 21.6 side y13s-ref216-1 rc 1 | `fail=0` (below) | **21.6: fail=4** |
| **getaddrinfo_a** | keke gaimass-1 rc 0 | dns_products 3 passed both sets; all requests answered (below) | 21.6 also all answered |
| **DL-RUNPATH** | mei drp-1 rc 1; 21.6 side drp-ref216-1 rc 1 | `pass=16 fail=2` (the 2: finding 2, by ruling) | **21.6: fail=4** |
| #6 CJK / #6 (1120F's probe) | mei y6c-1 rc 0, y6-1 rc 0 | `pass=32 fail=0`; **`pass=81 fail=0`** | y6: **21.6 fail=2** |
| **ICONV-REST** | mei y6r-1 rc 0; 21.6 side y6r-ref216-1 rc 1 | `pass=13 fail=0 ref_differ=0` (below) | **21.6: fail=6** |
| #20 / signal 32 | mei y20-1, sig32-1 | `RESULT fail=0`; `fail=0` | same |
| #14 | mei y14-1 rc 1, y14r-1 rc 1, eetwet-1 | as 1121E (trixie skew, now tzdata 2026f; zdump stderr) | same |
| roots / dbg+gdb | mei roots-1 rc 1 | `fresh: RESULT fail=0`; `VALIDATE roots bad=1` (the known dbg row) | same |
| lintian | mei lint-1 rc 0 | as 21.6 (0 masked lines differ) | same |
| plants | keke plants-1 rc 0 | 6 plants: 6 CAUGHT, 0 SURVIVED, 0 WARN | same |
| stage / install | keke stage-1, install-1 rc 0 | 9 assets + SHA256SUMS + MANIFEST, `sha256sum -c` 9 OK; `inst: RESULT fails=0` | - |
| docs | 1121D's doccheck.py on the head | 23 problems, the same 23 (two line numbers moved) | same |

**Staged** (keke `~/release-staging/snapshot-20261121.7/`): libc6, libc6-dbg, libc6-dev, libc-bin, libc-dev-bin
(`_2.39+niji20261121.7-1_amd64.deb`), libc-l10n, locales (`_2.39+niji20261121.7-1_all.deb`), niji6, niji6-dev
(`_0.0.snapshot20261121.7+2c64fde8a98d_amd64.deb`, `~` written as `.` for GitHub), SHA256SUMS, MANIFEST. Nothing
published, no tag.

## #13 rest: memcheck on everyday programs (y13s-1 on the head, y13s-ref216-1 on 21.6)

Swapped trixie root, the whole libc6-dbg of the set under test, trixie's valgrind, `--leak-check=full`.

| program | stock | head | 21.6 |
|---|---|---|---|
| ls / | 0 errors | 0 errors | 12 (first invalid read in niji_machine_strcmp) |
| ls -l /etc | 0 | 0 | 14 (niji_machine_strcmp) |
| find /etc -name passwd | 0 | 0 | 71 (niji_machine_strlen) |
| sort /etc/passwd | 0 | 0 | 0 |
| every replaced string function (strs) | 0 | 0 | 12 (niji_machine_strncmp) |
| deliberate strlen overread (over) | 1 (invalid read in strlen) | 1 (strlen) | 1 (strlen) |

All rc 0. (The premise's 21.6 counts were 12, 14, 72, 0, 12; this run measured 71 for find.) **Leak record**
(y13-1, cxx.cc): `operator new[]` (vg_replace_malloc.c:729) <- `keep(int)` (cxx.cc:11) <- `main` (cxx.cc:27), as
stock; the stack does not stop inside libc. Still reachable 4,104 B in 1 block at exit under libniji (stock 0), as
21.6 (1121E's finding 2, routed to VALGRIND-LOADER-VIEW).

## getaddrinfo_a (gaimass-1, keke)

The head's dns_products resolver test (3 passed, both sets) and its gaimass load case (one build; probe linked
against each set's libc.a; load average 21-33 on 60 vCPUs, the job in a 4-CPU slot): head and 21.6 both `done:
ok=25000 other=0 answer 192.0.2.10 only=25000; signals: 5000`, cache on and off, as the reference.

Line for the notes: "21.6's 'loses up to ~2% under heavy load' was the test's fixed 65 s deadline running out on a
loaded host, not lost requests: run with the load-independent wait, 21.6 and this snapshot answer all 25,000 requests
and deliver all 5,000 signals."

## DL-RUNPATH (drp-1 on the head, drp-ref216-1 on 21.6)

Corpus built in the stock root: prog (DT_RUNPATH `$ORIGIN/lib`) -> libcall.so (DT_RUNPATH `$ORIGIN/cdep`) whose
`call_open("libD.so")` / `call_open("$ORIGIN/cdep/libD.so")` dlopen a bare name; abs (no lists) dlopens libcall.so by
absolute path (untrusted probe dir, and a subdirectory of the system library directory) then the bare libD.so; each
plain and as a setgid program (AT_SECURE 1 from the kernel, checked).

| rows | head | 21.6 |
|---|---|---|
| prog-plain: bare libD.so and `$ORIGIN/cdep/libD.so` through libcall's RUNPATH | found, as stock | NULL (2 FAIL) |
| abs-plain (both dirs) | found, as stock | as stock |
| prog-sgid (setgid program with `$ORIGIN` RUNPATH in an untrusted dir) | does not start (rc 127), as stock | as stock |
| abs-sgid (both dirs): libcall.so's `$ORIGIN` in a secure process | NULL (2 rows differ) | NULL (same 2) |

Secure mode: finding 2.

## ICONV-REST (y6r-1 on the head, y6r-ref216-1 on 21.6)

iconv_open both ways and a sample text UTF-8 -> set -> UTF-8 (cjkrt), then the iconv program round trip, for CP949,
UHC, WINDOWS-1252, windows-1251, KOI8R, ISO885915; judged against trixie's stock, reference = the host's glibc 2.39.

| | head | 21.6 |
|---|---|---|
| cjkrt rows as stock | 6 of 6 (bytes in hex equal) | 3 of 6: UHC, WINDOWS-1252, windows-1251 not opened |
| iconv program round trip | 6 of 6 as stock (bytes and md5) | 3 of 6 (same three: rc 1, 0 bytes) |
| rows differing from glibc 2.39 | 0 | 3 |
| `iconv -l` names | 984 (stock 1,180; 958 in both); all six listed | 310 (284 in both) |

`iconv -l`: ICONV-REST's hand-back says 982 of the reference's 1,180; y6r counts 984 output names (counting method
differs, not examined). Known limitations for the notes (ICONV-REST's report; known and owed, not findings of this
run): 35 names that converted differently before ICONV-REST, unchanged by it; 222 names the reference lists without
a libniji converter (EBCDIC/DBCS IBM sets, ISO-2022-*, UTF-7).

## Start-up (execcount, ASLR off, median of 101; startup-*-1, kstart-*-1, rstart-*-1)

21.6 = 1121E's products rerun here (p216): they reproduce 1121E's user numbers (within 1 instruction). User
instructions / page faults:

| program | class | glibc 2.39 | snapshot-2026-11-21.6 | 2c64fde8a | delta |
|---|---|---|---|---|---|
| empty main | Skylake-SP | 107,752 / 49 | 742,940 / 56 | 726,461 / 57 | -16,479 / +1 |
| 3 libraries | Skylake-SP | 143,094 / 65 | 985,927 / 71 | 966,077 / 72 | -19,850 / +1 |
| clock_gettime | Skylake-SP | 109,862 / 50 | 765,032 / 59 | 748,574 / 60 | -16,458 / +1 |
| empty main | Zen 1 | 101,533 / 48 | 742,708 / 56 | 726,222 / 57 | -16,486 / +1 |
| 3 libraries | Zen 1 | 136,734 / 64 | 985,680 / 71 | 965,824 / 72 | -19,856 / +1 |
| clock_gettime | Zen 1 | 103,672 / 50 | 764,828 / 60 | 748,365 / 61 | -16,463 / +1 |

Kernel instructions / syscalls (kstart; kernel counts move by up to ~50k between runs; syscalls exact, unchanged):

| program | class | glibc | 21.6 | 2c64fde8a |
|---|---|---|---|---|
| empty main | Skylake-SP | 690,382 / 46 | 742,353 / 29 | 749,611 / 29 |
| 3 libraries | Skylake-SP | 981,782 / 72 | 1,142,407 / 59 | 1,150,405 / 59 |
| clock_gettime | Skylake-SP | 693,862 / 46 | 772,876 / 29 | 777,629 / 29 |
| empty main | Zen 1 | 732,965 / 42 | 816,602 / 28 | 823,334 / 28 |
| 3 libraries | Zen 1 | 1,038,525 / 68 | 1,236,674 / 58 | 1,244,035 / 58 |
| clock_gettime | Zen 1 | 740,137 / 42 | 850,769 / 28 | 857,374 / 28 |

libresolv pair (rstart; res0 needs libresolv.so.2; pie0 empty), user instructions, then res0 faults; kernel / syscalls:

| class | | glibc 2.39 | 21.6 | 2c64fde8a |
|---|---|---|---|---|
| Skylake-SP | pie0 / res0 user | 107,645 / 169,020 | 742,908 / 921,834 | 726,429 / 889,150 |
| Skylake-SP | res0 faults; kernel / syscalls | 55; 823,115 / 58 | 61; 938,070 / 42 | 62; 944,007 / 42 |
| Zen 1 | pie0 / res0 user | 101,481 / 162,716 | 742,625 / 921,294 | 726,141 / 888,599 |
| Zen 1 | res0 faults; kernel / syscalls | 54; 841,740 / 54 | 61; 956,015 / 40 | 62; 962,548 / 40 |

## Findings

1. **Start-up (INFORMATIONAL, not blocking in my judgement; route to ICONV-REST's / ICONV-CJK-STARTUP's owner):**
   COMPAT19-LDSO-COST-2's gain shows on both classes (-16.5k empty main, -19.9k three libraries, -32.7k for res0
   on both) but ~0.7k short of its stated -17.2k / -20.5k, and every program has **+1 page fault**. libc.so.6's RELATIVE
   relocations 1,573 -> 1,606 (+33; relsites.py: 1,861 -> 1,894 sites, relocated pages 13 -> 14): 8 in
   `niji::iconv::cjk_tables::CP949` (ICONV-REST's new descriptor), 25 unnamed; RW segment file size +4,096 B; third
   read-only segment +123,264 B. ~20 instructions per relocation x 33 = ~660 (estimate) and the one more relocated
   page = the fault. Same mechanism as 1121E's finding 1; ICONV-CJK-STARTUP (queued) is the fix-forward. Syscalls
   unchanged.
2. **DL-RUNPATH secure rows (INFORMATIONAL; the brief's "secure mode as stock" does not hold for two rows, by
   ruling):** in a secure process libniji expands a dlopened library's `$ORIGIN` only when its directory is a
   trusted directory (owner ruling #7190, D-27.16, threat-model rule 5: the compiled-in default directories). Both
   abs-sgid rows put libcall.so outside them (the probe dir, and a subdirectory of the system library directory), so
   both give NULL, as 21.6; not a regression. Whether a subdirectory of a default directory should count as trusted
   is for the loader's owner; no `docs/differences-from-glibc.md` row names this rule (grep `ORIGIN`: none).
3. **Sizes (INFORMATIONAL):** Installed-Size libc6 9503 -> 9631 KiB, libc6-dev 18183 -> 18309 KiB, libc-bin 2547 ->
   2663 KiB, niji6 9328 -> 9452 KiB, libc6-dbg 11672 -> 11684 KiB; file lists equal. Consistent with ICONV-REST's
   tables (libc.so.6 read-only segment +123,264 B); per-file growth in libc-bin not examined (1121E's finding 3:
   getconf carries the iconv tables).
4. **Docs (INFORMATIONAL, non-blocking):** doccheck: the same 23 problems as 21.6. RELEASE-NOTES.md mentions none of
   #13, #6, the getaddrinfo_a correction or DL-RUNPATH.
5. **Tooling (INFORMATIONAL):** mei's job queue (position ~33, 48 cores) held the .deb validations ~30 min after the
   relay; the .debs build (debs-1) took 42 min on keke beside the gate and plants.

## Jobs

JOBS (the final head; this branch adds handoff/ only; labels prefixed release-snapshot-1121f-): keke smoke-keke-1,
gate-1, startup-keke-1, kstart-keke-1, rstart-keke-1, gaimass-1, debs-1, plants-1, pkgdiff-1, stage-1, install-1,
precheck-1; mei smoke-mei-1, startup-mei-1, kstart-mei-1, rstart-mei-1, sig32-1, y20-1, dpo600-1, lint-1, many-1,
dyn22-1, y10-1, y19-1, y26-1, y26t-1, y23-1, c19-1, y25-1, y24-1, y28-1, y6-1, y25s-1, y21-1, yres-1, y23p-1, dlc-1,
y27-1, y13-1, y6c-1, eetwet-1, y13s-1, y6r-1. Evidence, not named (rc 1 on known or ruled lines only): roots-1,
yohane-1, y14-1, y14r-1, drp-1 (finding 2); 21.6 side (rc 1 by design): y13s-ref216-1, drp-ref216-1, y6r-ref216-1.

## Next step

Coordinator: tag snapshot-2026-11-21.7 from keke `~/release-staging/snapshot-20261121.7/` at 2c64fde8a when the
owner says so and tell Yohane: #13 rest fixed (memcheck clean on ls, find, sort and the string functions), #6 rest
in (CP949/UHC, WINDOWS-1252/1251, KOI8R, ISO885915; `iconv -l` 984 names), getaddrinfo_a: no requests were lost (the
note line above), DL-RUNPATH $ORIGIN fixed. Route finding 1 (and 3) to ICONV-CJK-STARTUP, 2 to the loader's owner,
4 to the next docs catch-up.
