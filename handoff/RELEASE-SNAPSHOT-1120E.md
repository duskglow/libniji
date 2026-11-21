COMPLETE: snapshot-ready at 305065b07 (#24 part fixed end to end: in a swapped trixie root, all 9 `$ORIGIN` start
cases name the library by the same full path as stock glibc in dl_iterate_phdr, dladdr, dlinfo and _r_debug;
20.4 fails 28 of 36 name checks, including the issue's own `./libA.so` shape; start-up -42.1k user instructions)

# RELEASE-SNAPSHOT-1120E: snapshot validation of 305065b07 (GitHub #24 part, start-up layout wins)

**State.** Validated head `305065b071d612f6624a9036b5e3ee412367c115` "Regenerate train-owned views" (origin/next at the
start, 23:04 UTC; commit time 22:01:52 UTC, epoch 1795212112); `git merge-base --is-ancestor 305065b07` holds
(DL-NAME-PATH-2). Branch `task/release-snapshot-1120e` = that head + handoff/ only (no product or packaging code
changed). Role keke `~/w/n1` (Zen 1); mei = Skylake-SP. One leg.

**Landed since snapshot-2026-11-20.4** (`git log --oneline 0e5a87e92..305065b07`: 39 lines; first-parent merges):
layout-ro-stable, dyn-start-link-shape, loader-folded-verify, release-snapshot-1120d (handoff only),
gate-bounded-parallel-2 (xtask only), **dl-name-path-2** (GitHub #24 part), two train view regenerations. Product
files changed: crates/boundary/src/ld/{entry,link}.rs, crates/boundary/src/{dlerr,thread}.rs, crates/ld/src/{elf,
reloc}.rs, xtask/src/so.rs (layout); no tools/deb change, so **DEB-COEXIST-SERIAL is not in this head**.

**Scripts.** mei and keke `~/work/RELEASE-SNAPSHOT-1120E/`, made on mei by `copy.sh` + `subst.sed` (both there) from
1120D's full set (every input directory: d/ m/ y/ y10/ y19/ y23/ y25/ y26/ y26t/ c19/ startup/; y25/ added to the
copy list); head, epoch, serial 20261120.5, labels, staging dir, product sets p204/p205 changed; role keke n1 as
before. `tree/` = `git archive 305065b07`. `run-keke.sh` = 1120D's through subst.sed; `run-keke2.sh` (new, sequencing
only: kstart-keke-1, which 1120D started by hand, then plants-1). New check: `y24.sh` + `y24/` (step 3; y25.sh's
recipe with gcc and libc6-dev in the root instead of g++). Known cosmetic carry-overs: cmp.sh's summary says "vs 1120b"
(it reads the 1120d logs) and does not cover y23/y25 (read directly below); the history comment lines of the copied
scripts were rewritten by the hash substitution (comments only).

## Results against snapshot-2026-11-20.4 (all on 305065b07's products)

| check | job | result | vs 20.4 |
|---|---|---|---|
| smoke lua_reload, cxx_dlclose | keke -smoke-keke-1 (builds the products in n1), mei -smoke-mei-1 (keke's products, sha checked) | `SMOKE: RESULT bad=0` both | same |
| gate | keke -gate-1 rc 0 on 305065b07 (gate.sh: clippy --all-features, xtask clippy, static, records check, affected --base origin/next) | green | same |
| system debs, check-system | keke -debs-1 | `check: PASS (7 packages, version 2.39+niji20261120.5-1)` | same |
| check-symbols | debs-1 | PASS, 4440 symbol lines, 13 libraries | same (4440) |
| coexist debs + check | debs-1 | `check: PASS (2 packages)`; version order: finding 2 | same |
| debs-1 rc 1 | debs-1 | DBG only: 17 of 19 ELF objects (no libc6-dbg file for `/usr/bin/iconv`, `/usr/bin/locale`; ruling #6321) | same |
| package comparison | keke -pkgdiff-1 rc 0 | 9 packages, file counts unchanged (libc6 75, libc6-dbg 44, libc6-dev 38, libc-bin 22, niji6 29, niji6-dev 48, others 7); libc6-dbg: 5 build-ids replaced; control fields: versions only; sizes within 6 KB | same |
| empty root | mei -roots-1 | `root: RESULT pass=19 fail=0` | same |
| swapped trixie root | roots-1 | `verdict: commands=65 same=63 known=2 bad=0` (ldd-6, ldd-7) | same |
| libc6-dbg + gdb | roots-1 (its only rc 1) | `dbg: RESULT pass=8 fail=1` trixie and excalibur (`Breakpoint 1.1`) | same |
| fresh install + ldconfig | roots-1 | `fresh: RESULT fail=0`; 119 masked lines, only the work-dir path differs (4 lines) | same |
| lintian | mei -lint-1 rc 0 | errors=10 warnings=60, trixie and excalibur; 46 masked lines identical to 20.4's | same |
| coexist proofs | lint-1 | `proof: RESULT pass=21 fail=0` trixie and excalibur | same |
| #16 rerun | mei -yohane-1 rc 1 | `pass=16 fail=2` (libc6-dev ships no C headers); 31 masked lines identical | same |
| many objects (#9) | mei -many-1 rc 0 | `pass=9 fail=0`; 18 masked lines identical | same |
| #22 | mei -dyn22-1 rc 0 | `pass=3 fail=0`; 57 masked lines identical | same |
| #10 + #18 | mei -y10-1 rc 1 | 19 PASS / 1 FAIL (bare-name dlopen through `$ORIGIN` DT_RUNPATH: DL-RUNPATH, not landed); 34 masked lines identical | same |
| #19 | mei -y19-1 rc 0 | `pass=14 fail=0`; 16 masked lines identical | same |
| #26 checks | mei -y26-1 rc 0 | `YOHANE-26 pass=20 fail=0`; 22 masked lines identical | same |
| #26 gcc ASan sample | mei -y26t-1 rc 0 | judge 0; 1 masked line identical | same |
| #23 col | mei -y23-1 rc 1 | 11 PASS / 2 FAIL (invalid-plain, invalid-b: cut-short sequences; STDIO-WIDE-BYTE-READS-3 not landed) | same |
| COMPAT19 | mei -c19-1 rc 0 | `COMPAT19 pass=16 fail=0`; 18 masked lines identical | same |
| #25 | mei -y25-1 rc 0 | `YOHANE-25 pass=21 fail=0` | same |
| **#24** | mei -y24-1 rc 0; 20.4: -y24-ref204-4 rc 1 | `YOHANE-24 pass=46 fail=0` (table below) | 20.4: 18 PASS / 28 FAIL |
| plants | keke -plants-1 rc 0 | 6 plants: 6 CAUGHT, 0 SURVIVED, 0 WARN-RC-ONLY, 0 WARN-NO-MSG (handoff/RELEASE-SNAPSHOT-1120E-plants.txt, 1120D's set) | same |
| stage | keke -stage-1 rc 0 | 9 assets + SHA256SUMS + MANIFEST; `sha256sum -c` 9 OK | - |
| install instructions | keke -install-1 rc 0 | `inst: RESULT fails=0` | same |
| docs | local doccheck.py (1120C's) on the head | 23 problems, the same 23 as 20.4 (line numbers unchanged) | same |

## #24 end to end (swapped trixie root, gcc 4:14.2.0-1, stock libc6 2.41-12+deb13u4)

Probes built once on the stock root with the root's gcc (`y24/pre.sh`): `libA.so` in `/tmp/y/bin/lib` needed by
p-runpath (DT_RUNPATH `$ORIGIN/lib`), p-rpath (DT_RPATH `$ORIGIN/lib`), p-plain (none; run with
`LD_LIBRARY_PATH='$ORIGIN/lib'`); `/tmp/y/link -> bin`; and the issue's shape, `/tmp/y/here/p-origin` with
`libA.so` beside it (DT_RUNPATH `$ORIGIN`), started as `./p-origin` from that directory. Run there (stock), then
libniji's libc6, libc-bin, libc6-dev, libc-dev-bin swapped in (held; libc.so.6, libm.so.6, the loader and ldd checked
to be the package's) and the same binaries rerun (`y24/post.sh`). Judged: exit status and libA.so's name per
interface; shown: every object's name per interface and the /proc/self/maps file names.

| case (started from /tmp/y) | stock glibc (all four interfaces) | snapshot-2026-11-20.4 | 305065b07 |
|---|---|---|---|
| origin-here (`cd here; ./p-origin`, the issue's shape) | `/tmp/y/here/libA.so` | `./libA.so` (4 of 4 differ) | identical to stock |
| runpath-rel `./bin/p-runpath` | `/tmp/y/bin/lib/libA.so` | `./bin/lib/libA.so` | identical |
| rpath-rel `./bin/p-rpath` | same | `./bin/lib/libA.so` | identical |
| ldlp-rel `./bin/p-plain` | same | `./bin/lib/libA.so` | identical |
| runpath-link `./link/p-runpath` | same (link resolved) | `./link/lib/libA.so` | identical |
| rpath-link `./link/p-rpath` | same | `./link/lib/libA.so` | identical |
| ldlp-link `./link/p-plain` | same | `./link/lib/libA.so` | identical |
| runpath-abs, rpath-abs (absolute start) | same | identical | identical |

On 305065b07 every output line of every case matches stock, including each interface's full object list (main
program `""`, `linux-vdso.so.1`, libA, libc.so.6, the loader), except the `maps all` line. **What remains of #24
(/proc/self/maps):** libA.so's mapping carries the same file name as on stock in all 9 cases, on 20.4 and on
305065b07 (the kernel names the file the loader opened). The only maps difference is placement: stock maps libA
between libc.so.6 and the loader, libniji above the loader (`[prog][libc.so.6][ld-linux][libA.so]` in address order).
No probe here shows a maps file-name difference; if #24's remaining part is a specific maps case, it needs the
issue's program (INFORMATIONAL, finding 3). Exit status 0 and stderr empty in every case on both sides.

## Start-up (execcount: exec to exit, ASLR off, median of 101; product paths of equal length in every tree)

User instructions / page faults (jobs -startup-mei-1, -startup-keke-1):

| program | class | glibc 2.39 | snapshot-2026-11-20.4 | 305065b07 | delta |
|---|---|---|---|---|---|
| empty main | Skylake-SP | 107,752 / 49 | 788,679 / 56 | 746,501 / 56 | -42,178 / 0 |
| 3 libraries | Skylake-SP | 143,094 / 65 | 1,044,190 / 72 | 1,002,062 / 72 | -42,128 / 0 |
| clock_gettime | Skylake-SP | 109,862 / 50 | 822,682 / 60 | 780,501 / 60 | -42,181 / 0 |
| empty main | Zen 1 | 101,533 / 48 | 788,448 / 56 | 746,269 / 56 | -42,179 / 0 |
| 3 libraries | Zen 1 | 136,734 / 64 | 1,043,944 / 72 | 1,001,815 / 72 | -42,129 / 0 |
| clock_gettime | Zen 1 | 103,672 / 50 | 822,480 / 61 | 780,298 / 61 | -42,182 / 0 |

20.4's column reproduces 1120D's own numbers (Skylake-SP exact, Zen 1 within 1). The -42.1k is the combined effect of
LAYOUT-RO-STABLE, DYN-START-LINK-SHAPE and LOADER-FOLDED-VERIFY (not split per story here; the brief's -3,780 is
LAYOUT-RO-STABLE's own figure). libc.so.6 and ld.so hashes identical on both hosts.

Kernel side (jobs -kstart-mei-1, -kstart-keke-1; kernel instructions median of the run, exclude_user; syscalls =
strace -f lines):

| program | class | glibc kernel / syscalls | 20.4 | 305065b07 |
|---|---|---|---|---|
| empty main | Skylake-SP | 689,157 / 46 | 838,790 / 45 | 834,378 / 44 |
| 3 libraries | Skylake-SP | 981,481 / 72 | 1,273,781 / 90 | 1,257,550 / 89 |
| clock_gettime | Skylake-SP | 691,188 / 46 | 874,945 / 45 | 866,416 / 44 |
| empty main | Zen 1 | 704,896 / 42 | 866,791 / 44 | 863,324 / 43 |
| 3 libraries | Zen 1 | 1,035,798 / 68 | 1,383,508 / 89 | 1,357,419 / 88 |
| clock_gettime | Zen 1 | 742,323 / 42 | 926,200 / 44 | 950,230 / 43 |

One syscall fewer in every row: one `mmap` fewer (libc.so.6's read-only data is one segment; by-name lists
otherwise identical). Kernel medians move -3k to -26k except Zen 1 clock_gettime (+24k); kernel instruction counts
spread up to ~70k between min and max within one run (Skylake-SP log) and the Zen 1 host ran at load average ~20, so
that row is within noise; not chased (non-blocking).

## Findings

1. **No regression; #24 (part) fixed.** Every check matches 20.4's masked lines except #24, 28 FAIL -> 0.
2. **Coexist version order: right this time, by the hash, not the serial (RULING NEEDED stays from 1120D).**
   DEB-COEXIST-SERIAL is not in this head, so niji6/niji6-dev are `0.0~snapshot20261120+305065b071d6`, built with
   NIJI_DEB_SERIAL=5 (system packages only use it). `dpkg --compare-versions 0.0~snapshot20261120+305065b071d6 gt
   0.0~snapshot20261120+0e5a87e92723` is true (the leading digit run 305065 > 0), so 20.4 -> 20.5 upgrades; a
   later same-day head starting with a lower digit run would invert again. Recommendation unchanged: land
   DEB-COEXIST-SERIAL before the next same-day snapshot.
3. **#24's /proc/self/maps part: no file-name difference found** (table above); only mapping placement differs.
   INFORMATIONAL: whoever owns #24 confirms with the reporter which maps case remains.
4. **Start-up -42.1k user instructions per start (-5.3%), faults unchanged, one syscall fewer**, both classes, every
   program.
5. **Docs (route to a docs story; install-packages.md passes against the staged assets, so not blocking).** No user
   doc changed since 20.4 (README.md, RELEASE-NOTES.md, docs/ outside the codemap untouched; spec/security/
   threat-model.md rule 2 corrected). **No release-notes entry or docs sentence for any of:** (a) "a library found at
   start through the program's `$ORIGIN` (DT_RUNPATH, DT_RPATH or LD_LIBRARY_PATH) is named by its full path, links
   resolved, in dl_iterate_phdr, dladdr, dlinfo(RTLD_DI_LINKMAP) and `_r_debug` (DL-NAME-PATH-2, GitHub #24 part)";
   (b) "a thread's dlerror record is unmapped when the thread exits (LOADER-FOLDED-VERIFY, D-27.67)"; (c) "libc.so.6's
   read-only data after its text is one PT_LOAD: one mapping fewer per process (LAYOUT-RO-STABLE, D-X.143)";
   (d) "dynamic start-up about 42,000 instructions cheaper (LAYOUT-RO-STABLE, DYN-START-LINK-SHAPE, D-27.68)".
6. Unchanged from 20.4: dbg `Breakpoint 1.1`; bare-name dlopen through `$ORIGIN` DT_RUNPATH (y10; DL-RUNPATH);
   libc6-dev ships no C headers (yohane fail=2); DBG 17 of 19 (ruling #6321); #23 cut-short sequences.

## Staged assets (keke `~/release-staging/snapshot-20261120.5/`, private)

libc6, libc6-dbg, libc6-dev, libc-bin, libc-dev-bin (`_2.39+niji20261120.5-1_amd64.deb`), libc-l10n, locales
(`_2.39+niji20261120.5-1_all.deb`), niji6, niji6-dev (`_0.0.snapshot20261120+305065b071d6_amd64.deb`), SHA256SUMS,
MANIFEST. Nothing to GitHub, no tag.

## Jobs

Named (prove the final head): keke gate-1, plants-1, smoke-keke-1, startup-keke-1, kstart-keke-1, pkgdiff-1, stage-1,
install-1, precheck-1; mei smoke-mei-1, startup-mei-1, kstart-mei-1, lint-1, many-1, dyn22-1, y19-1, y26-1, y26t-1,
c19-1, y25-1, y24-1. Evidence only, not named: debs-1, roots-1, yohane-1, y10-1, y23-1 (rc 1 on their known,
unchanged failures, rows above); y24-ref204-4 (20.4's packages, the "before" side, rc 1 by design). Superseded:
y24-ref204-1 (root lacked libc6-dev: gcc only recommends it; fixed), -2 (before the maps list was added), -3 (before
the issue's own shape was added).

## Exact next step

Coordinator: tag snapshot-2026-11-20.5 from the staged directory at 305065b07 when the owner says so and tell Yohane
#24's names are fixed (ask which maps case remains, finding 3); land DEB-COEXIST-SERIAL before the next same-day
snapshot (finding 2); route finding 5 to a docs story. Not to redo: anything above. Plants:
handoff/RELEASE-SNAPSHOT-1120E-plants.txt. Report: handoff/reports/RELEASE-SNAPSHOT-1120E.json.
