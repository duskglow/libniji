COMPLETE: snapshot-ready at 0e5a87e92 (#25 fixed end to end: the issue's reproducer and all 10 probe cases under g++ 14
cmp-identical to stock glibc in the swapped trixie root; 20.3 fails 10 of 10)

# RELEASE-SNAPSHOT-1120D: snapshot validation of 0e5a87e92 (GitHub #25 forced unwind)

**State.** Validated head `0e5a87e927235a019970cb2f8920562d331ae9a4` "Regenerate train-owned views" (origin/next at the
start, 20:11 UTC; commit time 19:33:03 UTC, epoch 1795203183); `git merge-base --is-ancestor 02fd586b8` holds
(THREAD-FORCED-UNWIND-3). Branch `task/release-snapshot-1120d` = that head + handoff/ only (no product or packaging
code changed). Role keke `~/w/n1` (Zen 1); mei = Skylake-SP. Two legs (one refresh).

**Landed since snapshot-2026-11-20.3** (`git log --oneline b732d9b46..0e5a87e92`: 119 lines; first-parent merges):
sl-headers-s15-3, **thread-forced-unwind-3** (GitHub #25), locale-slots-growth-7, release-snapshot-1120c (handoff
only), phase14-workarounds, wl-dynamic-gate, sl-headers-s07-6, train view regenerations. LAYOUT-RO-STABLE not landed.

**Scripts.** mei and keke `~/work/RELEASE-SNAPSHOT-1120D/`, made on mei by `copy.sh` + `subst.sed` (both there) from
1120C's full set (every input directory: d/ m/ y/ y10/ y19/ y23/ y26/ y26t/ c19/ startup/); head, epoch, serial
20261120.4, role keke n1, labels, staging dir changed. `tree/` = `git archive 0e5a87e92`. New (sequencing only):
`run-keke.sh` (keke: smoke-keke-1, products.tgz, gate-1, startup-keke-1, debs-1, debs.sha, pkgdiff-1, stage-1,
install-1), `chain-mei-pre.sh` (products.sha check, smoke-mei-1, startup-mei-1, kstart-mei-1), `chain-mei-debs.sh`
(debs.sha check, then the mei validations). New checks: `kstart.sh` (step 5 kernel side, `~/work/kprobe.sh`'s
method: execcount user and kernel instructions, strace -f syscall lines; glibc, p203, p204) and `y25.sh` + `y25/`
(#25; y23.sh's recipe). Note: cmp.sh's summary line still says "vs 1120b" (label only; it reads the 1120c logs), and
y23.sh/c19.sh usage comments name 20.2 (comments only).

## Results against snapshot-2026-11-20.3 (all on 0e5a87e92's products)

| check | job | result | vs 20.3 |
|---|---|---|---|
| smoke lua_reload, cxx_dlclose | keke -smoke-keke-1 (builds the products in n1), mei -smoke-mei-1 (keke's products, sha checked) | `SMOKE: RESULT bad=0` both | same |
| gate | keke -gate-1 rc 0 on 0e5a87e92; -gate-3 on the final head (same gate.sh) | clippy --all-features, xtask clippy, static, records check, affected --base origin/next | same |
| system debs, check-system | keke -debs-1 | `check: PASS (7 packages, version 2.39+niji20261120.4-1)` | same |
| check-symbols | debs-1 | PASS, 4440 symbol lines, 13 libraries | same (4440) |
| coexist debs + check | debs-1 | `check: PASS (2 packages)` | same; version order: finding 2 |
| debs-1 rc 1 | debs-1 | DBG only: 17 of 19 ELF objects (no libc6-dbg file for `/usr/bin/iconv`, `/usr/bin/locale`; ruling #6321) | same |
| package comparison | keke -pkgdiff-1 rc 0 | 9 packages, file counts unchanged (libc6 75, libc6-dbg 44, libc6-dev 38, libc-bin 22, niji6 29, niji6-dev 48, others 7); libc6-dbg: 6 build-ids replaced; control fields: versions only; sizes within 20 KB | same |
| empty root | mei -roots-1 | `root: RESULT pass=19 fail=0` | same |
| swapped trixie root | roots-1 | `verdict: commands=65 same=63 known=2 bad=0` (ldd-6, ldd-7) | same |
| libc6-dbg + gdb | roots-1 (its only rc 1) | `dbg: RESULT pass=8 fail=1` trixie and excalibur (`Breakpoint 1.1`) | same |
| fresh install + ldconfig | roots-1 | `fresh: RESULT fail=0`; 119 masked lines, only the work-dir path differs (4 lines) | same |
| lintian | mei -lint-1 rc 0 | errors=10 warnings=60, trixie and excalibur; 46 masked lines identical to 20.3's | same |
| coexist proofs | lint-1 | `proof: RESULT pass=21 fail=0` trixie and excalibur | same |
| #16 rerun | mei -yohane-1 rc 1 | `fail=2` (libc6-dev ships no C headers); 31 masked lines identical | same |
| many objects (#9) | mei -many-1 rc 0 | `pass=9 fail=0`; 18 masked lines identical | same |
| #22 | mei -dyn22-1 rc 0 | `dyn22: RESULT fail=0`; 57 masked lines identical | same |
| #10 + #18 | mei -y10-1 rc 1 | 19 PASS / 1 FAIL (bare-name dlopen through `$ORIGIN` DT_RUNPATH); 34 masked lines identical | same |
| #19 | mei -y19-1 rc 0 | `pass=14 fail=0`; 16 masked lines identical | same |
| #26 checks | mei -y26-1 rc 0 | `YOHANE-26 pass=20 fail=0`; 22 masked lines identical | same |
| #26 gcc ASan sample | mei -y26t-1 rc 0 | judge: stock 20/20, niji 20/20, nijibuilt 20/20 | same |
| #23 col | mei -y23-1 rc 1 | 11 PASS / 2 FAIL (1120C finding 1, cut-short sequences; STDIO-WIDE-BYTE-READS-3 not landed); 14 masked lines identical | same |
| COMPAT19 | mei -c19-1 rc 0 | `COMPAT19 pass=16 fail=0`; 18 masked lines identical | same |
| **#25** | mei -y25-1 rc 0; 20.3: -y25-ref203-1 rc 1 | `YOHANE-25 pass=21 fail=0` (table below) | 20.3: 9 PASS / 12 FAIL |
| plants | keke -plants-1 rc 0 | 6 plants: 6 CAUGHT, 0 SURVIVED, 0 WARN-RC-ONLY, 0 WARN-NO-MSG; one WARN-DEV-SHM (the brief's /dev/shm dir) (handoff/RELEASE-SNAPSHOT-1120D-plants.txt, 1120C's set) | same |
| stage | keke -stage-1 rc 0 | 9 assets + SHA256SUMS + MANIFEST; `sha256sum -c` 9 OK | - |
| install instructions | keke -install-1 rc 0 | `inst: RESULT fails=0` | same |
| docs | local doccheck.py (1120C's) on the head | 23 problems, the same 23 as 20.3 (three line numbers in differences-from-glibc.md moved by 6) | same |

## #25 end to end (swapped trixie root, g++ 4:14.2.0-1 / g++-14 14.2.0-19, libstdc++6 and libgcc-s1 14.2.0-19)

Probes built once on the stock root with the root's g++ (`y25/pre.sh`), run there (stock glibc), then libniji's
libc6, libc-bin, libc6-dev, libc-dev-bin swapped in (`dpkg -i`, held; libc.so.6, libm.so.6, the loader and ldd checked
to be the package's) and the same binaries rerun (`y25/post.sh`): output bytes by `cmp`, exit status, stderr. `repro`
= the issue's reproducer; `fu` cases one per run. Outputs below as `|`-joined lines.

| case | what it checks | stock (trixie libc6 2.41-12+deb13u4) | snapshot-2026-11-20.3 | 0e5a87e92 |
|---|---|---|---|---|
| repro | the issue's program | `destructor ran=1`, rc 0 | `destructor ran=0` | **cmp-identical to stock** |
| exit-dtor | pthread_exit: locals' destructors in the leaving frame and its caller | `dtor inner\|dtor outer\|join: 7` | `join: 7` (no destructor) | identical |
| cancel-dtor | the same on acting on a cancellation request | `dtor inner\|dtor outer\|join: PTHREAD_CANCELED` | `join: PTHREAD_CANCELED` | identical |
| exit-fu | `catch (abi::__forced_unwind &)` entered, rethrown | `dtor try\|caught abi::__forced_unwind, rethrow\|dtor inner\|dtor outer\|join: 7` | `join: 7` | identical |
| cancel-fu | the same, cancellation | as exit-fu, `join: PTHREAD_CANCELED` | `join: PTHREAD_CANCELED` | identical |
| exit-all | `catch (...)` entered, rethrown | `dtor try\|caught (...), rethrow\|dtor inner\|dtor outer\|join: 7` | `join: 7` | identical |
| cancel-all | the same, cancellation | as exit-all, `join: PTHREAD_CANCELED` | `join: PTHREAD_CANCELED` | identical |
| exit-lock | `std::lock_guard` held across pthread_exit | `dtor inner\|dtor outer\|join: 7\|mutex free after the thread ended` | `join: 7\|mutex still held` | identical |
| ptask | `std::packaged_task` whose function calls pthread_exit | `future_error: broken_promise`, rc 0 | no output, rc 124 (hangs: the future is never made ready; timeout) | identical |
| async | `std::async` whose function calls pthread_exit | `future_error: broken_promise`, rc 0 | no output, rc 124 (hangs) | identical |

Exit status 0 and stderr empty in every case on stock and on 0e5a87e92. 20.3's column is job -y25-ref203-1 (20.3's
staged-equivalent debs from 1120C's work dir, the same y25.sh): 9 PASS (package files, exit status of 8 cases) and
12 FAIL (10 outputs, 2 exit statuses). The stock root's own C library is recorded only, as in y23.

## Start-up (execcount: exec to exit, ASLR off, median of 101; product paths of equal length in every tree)

User instructions / page faults (jobs -startup-mei-1, -startup-keke-1):

| program | class | glibc 2.39 | snapshot-2026-11-20.3 | 0e5a87e92 | delta |
|---|---|---|---|---|---|
| empty main | Skylake-SP | 107,752 / 49 | 788,635 / 57 | 788,679 / 56 | +44 / -1 |
| 3 libraries | Skylake-SP | 143,094 / 65 | 1,044,146 / 73 | 1,044,190 / 72 | +44 / -1 |
| clock_gettime | Skylake-SP | 109,862 / 50 | 822,638 / 61 | 822,682 / 60 | +44 / -1 |
| empty main | Zen 1 | 101,533 / 48 | 788,404 / 57 | 788,447 / 56 | +43 / -1 |
| 3 libraries | Zen 1 | 136,734 / 64 | 1,043,900 / 73 | 1,043,943 / 72 | +43 / -1 |
| clock_gettime | Zen 1 | 103,672 / 50 | 822,435 / 62 | 822,479 / 61 | +44 / -1 |

20.3's column reproduces 1120C's own numbers within 5 instructions. Kernel side (jobs -kstart-mei-1, -kstart-keke-1;
kernel instructions median of the run, exclude_user; syscalls = strace -f lines):

| program | class | glibc kernel / syscalls | 20.3 | 0e5a87e92 |
|---|---|---|---|---|
| empty main | Skylake-SP | 687,602 / 46 | 849,756 / 45 | 841,251 / 45 |
| 3 libraries | Skylake-SP | 985,609 / 72 | 1,298,704 / 90 | 1,277,239 / 90 |
| clock_gettime | Skylake-SP | 691,443 / 46 | 886,475 / 45 | 873,681 / 45 |
| empty main | Zen 1 | 700,299 / 42 | 871,896 / 44 | 858,777 / 44 |
| 3 libraries | Zen 1 | 998,612 / 68 | 1,325,269 / 89 | 1,307,390 / 89 |
| clock_gettime | Zen 1 | 708,283 / 42 | 913,986 / 44 | 899,803 / 44 |

Kernel medians spread ~5-10k between min and max within a run; the 8-21k drop matches one page fault fewer. Syscall
counts unchanged. vs glibc: +22-31% kernel instructions, +18-21 syscalls with 3 libraries, as in the 11-20 kernel probe.

## Findings

1. **No regression; #25 fixed.** Every check matches 20.3's masked lines except #25, which goes from 12 FAIL to 0.
2. **Coexist version order now inverts (RULING NEEDED; 1120B's finding 1, now hit).** `niji6`/`niji6-dev`
   `0.0~snapshot20261120+0e5a87e92723` sorts BELOW 20.3's `0.0~snapshot20261120+b732d9b46a41` (`dpkg
   --compare-versions ... lt` true): apt sees 20.3 -> 20.4 of the coexist packages as a downgrade. The system packages
   (`2.39+niji20261120.4-1`, the ones Yohane swaps) order right. Not changed here (no packaging code in a snapshot).
   Recommendation: not blocking for Yohane (system packages); a packaging story puts the snapshot serial into the
   coexist version (e.g. `0.0~snapshot20261120.4+<hex>`), and the release notes say to install 20.4's niji6 with
   `dpkg -i` or an explicit version over 20.3's.
3. **Start-up +44 user instructions per start, one page fault fewer, both classes, every program**; syscalls
   unchanged. INFORMATIONAL, within the snapshot's non-blocking bound.
4. **Docs (route to a docs story; install-packages.md passes against the staged assets, so not blocking).** User docs
   changed since 20.3: docs/compat-flags.md (COMPAT-32, -33, -34) and docs/differences-from-glibc.md (rows below);
   RELEASE-NOTES.md, README.md, tunables.md unchanged: **no release-notes entry and no docs sentence for #25**.
   **User-visible changes since snapshot-2026-11-20.3:** (a) "pthread_exit and acting on a cancellation request
   unwind the thread's stack through the program's unwinder: C++ destructors run, `catch (abi::__forced_unwind &)`
   and `catch (...)` are entered and must rethrow; `std::packaged_task`/`std::async` report `broken_promise` instead
   of hanging (THREAD-FORCED-UNWIND-3, D-22.19, GitHub #25)" (no doc carries it); (b) "`<sys/stat.h>` declares no
   `getumask` (COMPAT-32)"; (c) "no libniji header declares `bsd_signal`; the symbol stays exported (COMPAT-33)";
   (d) "the `MS_*` names of `<sys/mount.h>` are macros only, so `<sys/mount.h>` and `<linux/mount.h>` compile in either
   order (COMPAT-34)"; (e) "`<sys/vm86.h>` includes and compiles on x86-64"; (f) "`struct _IO_FILE` is complete after
   `<mntent.h>` alone" (headers slices 7, 8, 15).
5. Unchanged from 20.3: dbg `Breakpoint 1.1`; bare-name dlopen through `$ORIGIN` DT_RUNPATH (y10); libc6-dev ships no
   C headers (yohane fail=2); DBG 17 of 19 (ruling #6321); #23 cut-short sequences (1120C finding 1).

## Staged assets (keke `~/release-staging/snapshot-20261120.4/`, private)

libc6, libc6-dbg, libc6-dev, libc-bin, libc-dev-bin (`_2.39+niji20261120.4-1_amd64.deb`), libc-l10n, locales
(`_2.39+niji20261120.4-1_all.deb`), niji6, niji6-dev (`_0.0.snapshot20261120+0e5a87e92723_amd64.deb`), SHA256SUMS,
MANIFEST. `sha256sum -c SHA256SUMS`: 9 OK. Nothing to GitHub, no tag.

## Jobs

Named (prove the final head): keke gate-3, plants-1, smoke-keke-1, startup-keke-1, kstart-keke-1, pkgdiff-1, stage-1,
install-1; mei smoke-mei-1, startup-mei-1, kstart-mei-1, lint-1, many-1, dyn22-1, y19-1, y26-1, y26t-1, c19-1, y25-1.
Evidence only, not named: debs-1, roots-1, yohane-1, y10-1, y23-1 (rc 1 on their known, unchanged failures, rows
above); y25-ref203-1 (20.3's packages, the "before" side, rc 1 by design). Superseded: gate-1 (on 0e5a87e92, green;
same gate.sh), gate-2 (red: `cargo xtask static` private-paths, two home paths in leg 1's handoff note; fixed by this
rewrite).

## Exact next step

Coordinator: tag snapshot-2026-11-20.4 from the staged directory at 0e5a87e92 when the owner says so and tell Yohane
#25 is fixed; rule on finding 2 (coexist version order); route finding 4 to a docs story. Not to redo: anything above.
Plants: handoff/RELEASE-SNAPSHOT-1120D-plants.txt. Report: handoff/reports/RELEASE-SNAPSHOT-1120D.json.
