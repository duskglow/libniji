COMPLETE: snapshot-ready at fe0899b79 (serial 3: 2.39+niji20261121.3-1, staged as snapshot-20261121.3; tag
snapshot-2026-11-21.3): no regression against 21.2; #23 kind 1: Yohane's differential probe has every kind-1 row as
stock (21.2: 6 table + 4 ccs rows differ); the 300-library corpus opens, names and closes as stock (21.2: "out of room
for object names"); plants 6/6 CAUGHT.

# RELEASE-SNAPSHOT-1121B: snapshot validation of fe0899b79 (GitHub #23 kind 1, the loader's growing arenas)

**State.** Validated head `fe0899b79e087e42af80cfe1a683e92bce304f01` (origin/next at the start, 08:56 UTC; "Regenerate
train-owned views"; commit time 2026-11-21T08:04:01Z, epoch 1795248241); `git merge-base --is-ancestor a766bbb4` holds.
Branch `task/release-snapshot-1121b` = that head + handoff/ only (no product or packaging code changed). Role **mei
`~/w/s4` (Skylake-SP)**; keke = Zen 1. 21.2 = 71597d2a4 (RELEASE-SNAPSHOT-1121A). One leg.

**Landed since snapshot-2026-11-21.2** (`git log --oneline --first-parent 71597d2a4..fe0899b79`): headers-gen-s10-hooks
(390c13361), ldso-program-no-cwd (4fea0836f), static-layout-flip (6c3febe68), dl-process-objects part 1 (984cd01fa),
release-snapshot-1121a (78162595d, handoff only), compat19-startup-zero (c43016fc5), **heap-spill-cold (592faaec6; not
in the brief's list)**, stdio-wide-byte-reads-4 (a766bbb40, #23), sl-headers-s10-5 (6b6f6742b), five train view
regenerations (e4777a6ba, 32261597b, 7e5d8b3fa, ef7bc6fd3, fe0899b79). **Not in the head:** VALGRIND-LDSO-SYMBOLS-2
(#13) and CRT-START-FILES-3 (#27), so their checks (brief step 6) were not run.

**Version / serial.** Commit date 2026-11-21 = the day's third snapshot: `NIJI_DEB_SERIAL=3`, debs-1 built
`2.39+niji20261121.3-1` (7 system packages) and `niji6{,-dev}_0.0~snapshot20261121.3+fe0899b79e08`. `dpkg
--compare-versions` (mei): `2.39+niji20261121.3-1` gt `2.39+niji20261121.2-1` and lt `2.40`;
`0.0~snapshot20261121.3+fe0899b79e08` gt `0.0~snapshot20261121.2+71597d2a40ac`: all yes.

**Scripts** (mei and keke `~/work/RELEASE-SNAPSHOT-1121B/`; local scratch `~/w/.scratch/RELEASE-SNAPSHOT-1121B/`):
1121A's set copied by copy.sh + subst.sed (names, head, epoch, serial, role, labels only; every input directory
including 1121A's y21/ and yres/); tree/ = `git archive fe0899b79` (sha256 checked on both hosts, 8722 files = `git
ls-tree`). Plants file handoff/RELEASE-SNAPSHOT-1121B-plants.txt (1121A's; ids, paths, role, version only; each pattern
checked present once). New: **y23p.sh** (#23: Yohane's differential probe as STDIO-WIDE-BYTE-READS-4 rebuilt it:
wbprobe.c, cases.bash, wbccs.c and its recorded outputs) and **dlc.sh** (DL-PROCESS-OBJECTS' 300-library corpus,
its refprobe recipe, prog.c plus a count after each dlclose), both made by mknew.sh from y21.sh's swapped-root recipe;
run-new2.sh sequences them. cmp4.sh = cmp3.sh with 1121A's real labels (cmp3's substituted per-job numbers were
1120F's, so it found no 21.2 log for most checks). kx.sh extracts the start-up columns.

## Results against snapshot-2026-11-21.2 (all on fe0899b79's products / serial-3 packages)

cmp4.sh: each check's masked PASS/FAIL/RESULT/verdict lines against 1121A's log of the same check. Every check: 0
lines differ except work-dir paths (s1 -> s4, RELEASE-SNAPSHOT-1121A -> 1121B).

| check | job | result | vs 21.2 |
|---|---|---|---|
| smoke lua_reload, cxx_dlclose | mei smoke-mei-1 (builds the products in s4), keke smoke-keke-1 (mei's products, sha checked) | `SMOKE: RESULT bad=0` both | same |
| gate | mei gate-1 rc 0 | `GATE bad=0` (0 suites: handoff-only branch) | same |
| system debs + check-system + symbols | mei debs-1 rc 0 | `check: PASS` (7 packages, 2.39+niji20261121.3-1); check-symbols PASS 4440 lines, 13 libraries | same |
| coexist debs + check | debs-1 | `check: PASS (2 packages)`; version order above | same |
| libc6-dbg | debs-1 | 19 of 19 ELF objects resolve; 19 .debug files | same |
| package comparison | keke pkgdiff-1 rc 0 | 9 packages, file counts unchanged; control fields: versions only; libc6 +48 B, libc6-dbg +17,940 B, libc6-dev +2,680 B, niji6 +1,604 B, niji6-dev +5,592 B, libc-bin -36 B | expected |
| #16 rerun | mei yohane-1 rc 1 | `pass=16 fail=2` (libc6-dev ships no C headers) | same |
| many objects (#9) | mei many-1 | `pass=9 fail=0` | same |
| #22 | mei dyn22-1 | `pass=3 fail=0` | same |
| #10 + #18 | mei y10-1 rc 1 | 19 PASS / 1 FAIL (DL-RUNPATH, not landed) | same |
| #19 | mei y19-1 | `pass=14 fail=0` | same |
| #26 / gcc ASan | mei y26-1, y26t-1 | `pass=20 fail=0`; judge 0 | same |
| #23 (y23) | mei y23-1 | `pass=13 fail=0` | same |
| **#23 differential probe** | mei y23p-1; 21.2: y23p-ref212-1 | `pass=9 fail=0`; table below | **21.2: pass=3 fail=6** (the kind-1 rows) |
| **loader corpus** | mei dlc-1; 21.2: dlc-ref212-2 | `pass=5 fail=0`; table below | **21.2: fail** ("out of room for object names") |
| COMPAT19 | mei c19-1 | `pass=16 fail=0` | same |
| #25 dynamic / static | mei y25-1, y25s-1 | `pass=21` / `pass=20`, fail 0 | same |
| #24 | mei y24-1 | `pass=46 fail=0` | same |
| #28 | mei y28-1 | `pass=10 fail=0` | same |
| #6 | mei y6-1 rc 1 | `pass=79 fail=2` (WINDOWS-1252, windows-1251) | same |
| #21 | mei y21-1 | `pass=7 fail=0`; numbers as 21.2 except find's 3-file need 20,464 -> 20,480 KiB (one 16 KiB bisection step); the issue's formula 5 of 5 | same |
| libresolv | mei yres-1 | `pass=36 fail=0`, every row same as stock | same |
| empty root / swapped trixie root | mei roots-1 | `root: RESULT pass=21 fail=0`; `verdict: commands=65 same=63 known=2 bad=0` | same |
| libc6-dbg + gdb | roots-1 (its only rc 1) | `dbg: RESULT pass=8 fail=1` (`#5 ... in ?? ()`, known) | same |
| lintian | mei lint-1 rc 0 | errors=10 warnings=46, trixie and excalibur; proofs pass=21 fail=0 both | same |
| plants | mei plants-1 rc 0 | 6 plants: 6 CAUGHT, 0 SURVIVED, 0 WARN | same |
| stage | keke stage-1 rc 0 | 9 assets + SHA256SUMS + MANIFEST; `sha256sum -c` 9 OK | - |
| install instructions | keke install-1 rc 0 | `inst: RESULT fails=0` | same |
| docs | doccheck.py (1120F's) on the head | 23 problems, the same 23 lines as 21.2 (no line moved) | same |

**Staged** (keke `~/release-staging/snapshot-20261121.3/`): libc6, libc6-dbg, libc6-dev, libc-bin, libc-dev-bin
(`_2.39+niji20261121.3-1_amd64.deb`), libc-l10n, locales (`_2.39+niji20261121.3-1_all.deb`), niji6, niji6-dev
(`_0.0.snapshot20261121.3+fe0899b79e08_amd64.deb`, `~` written as `.` for GitHub), SHA256SUMS, MANIFEST.

## #23: Yohane's differential probe (y23p; swapped trixie root, C.UTF-8; stock = trixie glibc 2.41)

Yohane's format = result + EILSEQ, without the `[eof= err=]` indicators.

| judged | head fe0899b79 | 21.2 |
|---|---|---|
| stock table (39 rows) and boundary rows (23) = STDIO-WIDE-BYTE-READS-4's recorded reference | PASS | PASS |
| kind 1, table: `c3`, `e381` x `wbwbwb` `wbbbbb` `wwwwww` (6 rows) as stock | **PASS** | FAIL (6 rows differ) |
| kind 1, `ccs=UTF-8`: `c3`, `e381` (4 rows) as stock | **PASS** | FAIL (4 rows) |
| rows differing from stock = the recorded kind 2 / kind 3 rows (15: kind 2 `61fc6364` `616263`x2 `c3a978`x2 `e381826f6b`x2 `c3287a` `e282ac7478`x2 `f09f988021`x2 in `wbwbwb`/`wbbbbb`; kind 3 `f490808073` x3) | PASS | FAIL (21 rows: those + the 6 kind-1) |
| libniji's table, boundary and ccs outputs = STDIO-WIDE-BYTE-READS-4's recorded head outputs | PASS x3 | FAIL x3 |

Shown, not judged (head): with the indicators, 23 of 39 table rows, 14 of 23 boundary rows and 6 of 8 ccs rows
differ from stock (21.2: 27, 17, 8). Two kind-1 rows differ only in an indicator after the byte reads (`c3`/`e381`
`wbbbbb`: stock `eof=0`, libniji `eof=1`), a kind-2 effect (stock's byte reads after its wide layer return `EOF` with
no indicator; the differences row "Byte operations on a wide-oriented stream" covers it).

## The loader's arenas (dlc: DL-PROCESS-OBJECTS' corpus through dlopen, swapped trixie root; many-1 above)

Corpus built in the stock root: 300 libraries in a directory of 1056 bytes, 320,292 path bytes, 44,850 dependency
entries (library k needs every later one); prog opens libroot.so with RTLD_NOW twice and dlcloses it.

| per round (both rounds) | stock | head: prog (stock-built) | head: prog relinked with libniji's dev | 21.2 (both programs) |
|---|---|---|---|---|
| status | 0 | 0 | 0 | 1: `open: libp122.so: out of room for object names` |
| sum / dladdr / phdr / lmap | 45150 / 300 / 301 / 301 | same | same | - |
| after dlclose: corpus objects in dl_iterate_phdr / link map | 0 / 0 | 0 / 0 | 0 / 0 | - |

## Start-up (execcount, ASLR off, median of 101; startup-*-1, kstart-*-1, rstart-*-1)

User instructions / page faults (21.2 = 1121A's products, rerun here: they reproduce 1121A's numbers, Zen 1 within 1):

| program | class | glibc 2.39 | snapshot-2026-11-21.2 | fe0899b79 | delta |
|---|---|---|---|---|---|
| empty main | Skylake-SP | 107,752 / 49 | 741,092 / 52 | 741,148 / 52 | +56 / 0 |
| 3 libraries | Skylake-SP | 143,094 / 65 | 996,656 / 67 | 996,715 / 67 | +59 / 0 |
| clock_gettime | Skylake-SP | 109,862 / 50 | 775,102 / 56 | 775,158 / 56 | +56 / 0 |
| empty main | Zen 1 | 101,533 / 48 | 740,860 / 52 | 740,915 / 52 | +55 / 0 |
| 3 libraries | Zen 1 | 136,734 / 64 | 996,408 / 67 | 996,468 / 67 | +60 / 0 |
| clock_gettime | Zen 1 | 103,672 / 50 | 774,898 / 57 | 774,955 / 57 | +57 / 0 |

Kernel instructions / syscalls (kernel counts move by up to ~50k between runs of one tree; syscalls are exact):

| program | class | glibc | 21.2 | fe0899b79 |
|---|---|---|---|---|
| empty main | Skylake-SP | 690,321 / 46 | 785,691 / 38 | 781,571 / 37 |
| 3 libraries | Skylake-SP | 986,138 / 72 | 1,189,058 / 80 | 1,195,324 / 80 |
| clock_gettime | Skylake-SP | 693,766 / 46 | 821,436 / 38 | 810,387 / 37 |
| empty main | Zen 1 | 708,476 / 42 | 850,334 / 37 | 800,133 / 36 |
| 3 libraries | Zen 1 | 1,002,516 / 68 | 1,216,725 / 79 | 1,218,444 / 79 |
| clock_gettime | Zen 1 | 710,853 / 42 | 883,125 / 37 | 882,130 / 36 |

Empty main and clock_gettime: one `munmap` fewer (5 -> 4). The libresolv pair (res0 needs libresolv.so.2; pie0 empty),
user instructions, then res0's kernel instructions / syscalls:

| class | | glibc 2.39 | 21.2 | fe0899b79 |
|---|---|---|---|---|
| Skylake-SP | pie0 / res0 user | 107,645 / 169,020 | 741,046 / 920,057 | 741,102 / 920,120 |
| Skylake-SP | res0 - pie0 | +61,375 | +179,011 | +179,018 |
| Skylake-SP | res0 kernel / syscalls | 826,632 / 58 | 978,326 / 55 | 979,023 / 55 |
| Zen 1 | pie0 / res0 user | 101,481 / 162,716 | 740,764 / 919,517 | 740,820 / 919,580 |
| Zen 1 | res0 - pie0 | +61,235 | +178,753 | +178,760 |
| Zen 1 | res0 kernel / syscalls | 846,126 / 54 | 994,401 / 53 | 993,792 / 53 |

## Findings

1. No regression in any check against 21.2: every compared check's verdict lines are the same.
2. **#23 kind 1 fixed in the package:** every kind-1 row of Yohane's probe is stock's (table and ccs=UTF-8); the rows
   still different are exactly STDIO-WIDE-BYTE-READS-4's recorded kind 2 and kind 3 (recorded differences).
3. **DL-PROCESS-OBJECTS part 1 in the package:** the corpus 21.2 refused opens, is named and closes as stock.
4. **Start-up (INFORMATIONAL):** +55..60 user instructions per start on both classes (0.008%), faults unchanged,
   one syscall fewer on the empty and clock_gettime programs; not chased (no product change in this run).
5. **Docs (INFORMATIONAL, non-blocking; same as 21.2's finding 5):** docs/RELEASE-NOTES.md "Unreleased" still lists
   only COMPAT-19 (nothing for #23 kind 1, the loader arenas, #21, libresolv, libc6-dbg DWARF, nor 21.1's items);
   differences-from-glibc.md "Character sets" still gives snapshot-2026-11-17's state; no row for the
   -static-libgcc thread exit (ruling #7118). The #23 row ("Byte operations on a wide-oriented stream") is current.
6. **Record (INFORMATIONAL):** STDIO-WIDE-BYTE-READS-4's handoff says `ccs-head.txt` = `ccs239.txt`; its own files
   differ in 4 of 8 rows (indicators, `ftell`, and `6162e381 wtbbt` byte values: kind 2). The kind-1 ccs rows match
   in Yohane's format, so the fix holds; the sentence overstates it.
7. The run's own slips: dlc-ref212-1 compiled prog.c after the swap, but libniji's libc6-dev ships no C headers (#16),
   so its relink step failed; fixed (prog.o compiled on the stock root, linked after the swap) and rerun as
   dlc-ref212-2 before the head's dlc-1. y23p's cases.sh renamed cases.bash (the maker checks *.sh with `sh -n`).

## Jobs

JOBS (the final head; this branch adds handoff/ only; rc 0 each): mei smoke-mei-1, gate-1, startup-mei-1, kstart-mei-1,
rstart-mei-1, debs-1, plants-1, lint-1, many-1, dyn22-1, y19-1, y26-1, y26t-1, y23-1, c19-1, y25-1, y24-1, y28-1,
y25s-1, y21-1, yres-1, y23p-1, dlc-1, precheck-1; keke smoke-keke-1, startup-keke-1, kstart-keke-1, rstart-keke-1,
pkgdiff-1, stage-1, install-1 (all labels prefixed release-snapshot-1121b-). Evidence, not named (rc 1 on known lines
only, as 21.2): roots-1, yohane-1, y10-1, y6-1; 21.2 side (rc 1 by design): y23p-ref212-1, dlc-ref212-2.
SUPERSEDED: dlc-ref212-1 (its relink step needed C headers libniji's libc6-dev does not ship).

## Next step

Coordinator: tag snapshot-2026-11-21.3 from keke `~/release-staging/snapshot-20261121.3/` at fe0899b79 when the owner
says so and tell Yohane (#23 kind 1: every kind-1 row of their probe as stock; kinds 2 and 3 are the recorded
differences). Route finding 5 (release notes, differences rows) to a docs story.
