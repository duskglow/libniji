COMPLETE: snapshot-ready at 3f2a84520

# RELEASE-SNAPSHOT-1120B: snapshot validation of 3f2a84520 (GitHub #26 complete)

**State.** Validated head `3f2a8452011850988cb6d0a61ffaf283241aad2e` "Regenerate train-owned views" (origin/next at
the start, 2026-11-20 10:40 UTC; commit time 09:45:50 UTC, epoch 1795167950). `git merge-base --is-ancestor
3f2a84520` holds (it is the head; ASAN-MAIN-THREAD's merge is 458e9c223). Branch `task/release-snapshot-1120b` = that
head + handoff/ only (no product or packaging code changed). Role keke `~/w/n1` (Zen 1); mei = Skylake-SP. Two legs
(leg 1 built, staged and started everything; leg 2 collected the mei roots validations and the keke plants).

**Scripts.** keke and mei `~/work/RELEASE-SNAPSHOT-1120B/`, made on mei by `copy.sh` + `subst.sed` (both in that
dir) from mei `~/work/RELEASE-SNAPSHOT-1120/` (the full set: *.sh, d/ m/ y/ y10/ y19/ y26/ startup/, the
`binutils-src` link to 1119B's input), then copied unchanged to keke. Changed: head, epoch, serial 20261120.2, role
keke n1 (chain-role builds in keke and runs `smoke-keke-1`; mei's smoke runs on an unpacked tree `tree/` with keke's
products), labels, staging dir, pkgdiff against snapshot-20261120.1, start-up sets p201 (20.1's products.tgz) and
p202 (this head's), paths of equal length. `tree/` = `git archive 3f2a84520`. New this run: `y26t.sh` + `y26t/`
(y26.sh's recipe; 20 gcc 14.2.0 ASan tests and `judge.py`, below).

**Landed since snapshot-2026-11-20.1** (`git log --oneline 50abc44ed..3f2a84520`: 38 lines; first-parent merges):
pass2-native-poison, release-snapshot-1120 (handoff only), mp-isqrt-fast, layout-byte-budgets-3,
test-reloc-etxtbsy, **asan-main-thread** (GitHub #26 second half, D-22.20), train view regenerations.

## Results against snapshot-2026-11-20.1 (all on 3f2a84520's products)

| check | job | result | vs 20.1 |
|---|---|---|---|
| smoke lua_reload, cxx_dlclose | keke -smoke-keke-1 (builds products in n1), mei -smoke-mei-1 | `SMOKE: RESULT bad=0` both | same |
| gate | keke -gate-1 rc 0 (on 3f2a84520), -gate-2 rc 0 (on the final head 1430173a8, the same gate.sh) | clippy --all-features, xtask clippy, static, records check, affected --base origin/next (0 of 303 targets): all RC 0 | same |
| system debs, check-system | keke -debs-1 | `check: PASS (7 packages, version 2.39+niji20261120.2-1)` | same |
| check-symbols | debs-1 | PASS, 4440 symbol lines, 13 libraries | same (4440) |
| coexist debs + check | debs-1 | `check: PASS (2 packages)` | same |
| debs-1 rc 1 | debs-1 | DBG only: 17 of 19 ELF objects; no libc6-dbg file for `/usr/bin/iconv`, `/usr/bin/locale` (ruling #6321) | same |
| package comparison | keke -pkgdiff-1 rc 0 | 9 packages, file counts unchanged (libc6 75, libc6-dbg 44, libc6-dev 38, libc-bin 22, niji6 29, niji6-dev 48, others 7); libc6-dbg: 6 build-ids replaced; control fields: versions only; sizes within 11 KB | same |
| empty root | mei -roots-1 | `root: RESULT pass=19 fail=0` | same |
| swapped trixie root | roots-1 | `verdict: commands=65 same=63 known=2 bad=0` (ldd-6, ldd-7: the known version-information lines) | same |
| libc6-dbg + gdb | roots-1 (its only rc 1) | `dbg: RESULT pass=8 fail=1` trixie and excalibur (the `Breakpoint 1.1` line, 19.2's finding 1) | same |
| fresh install + ldconfig | roots-1 | `fresh: RESULT fail=0` | same |
| lintian | mei -lint-1 rc 0 | errors=10 warnings=60, trixie and excalibur; the 50 normalised LINTIAN lines identical to 20.1's | same |
| coexist proofs | lint-1 | `proof: RESULT pass=21 fail=0` trixie and excalibur | same |
| #16 rerun | mei -yohane-1 rc 1 | `fail=2` (libc6-dev ships no C headers: `cm.c` does not compile); 33 masked PASS/FAIL lines identical to 20.1's | same |
| many objects | mei -many-1 rc 0 | `pass=9 fail=0` | same |
| #22 | mei -dyn22-1 rc 0 | `dyn: RESULT niji fail=0`, `dyn22: RESULT fail=0`; 58 masked lines identical | same |
| #10 + #18 | mei -y10-1 rc 1 | 19 PASS / 1 FAIL (bare-name dlopen through `$ORIGIN` DT_RUNPATH); 41 masked lines identical | same |
| #19 | mei -y19-1 rc 0 | `pass=14 fail=0` | same |
| #26 checks | mei -y26-1 rc 0 | `YOHANE-26 pass=20 fail=0` (first-line checks; full reports below) | same count, report now complete |
| #26 gcc ASan sample | mei -y26t-1 rc 0 | judge: stock 20/20, niji 20/20, nijibuilt 20/20 | 20.1: 14/20 (y26t-ref201-1) |
| plants | keke -plants-1 rc 0 | 6 plants: 6 CAUGHT, 0 SURVIVED, 0 WARN (handoff/RELEASE-SNAPSHOT-1120B-plants.txt) | same |
| stage | keke -stage-1 rc 0 | 9 assets + SHA256SUMS + MANIFEST; `sha256sum -c` all OK | - |
| install instructions | keke -install-1 rc 0 | `inst: RESULT fails=0` | same |
| docs | local doccheck on tree/ | 23 problems, the same 23 lines as 20.1 | same |

**Jobs named vs evidence.** Hand-back JOBS: keke gate-2, plants-1, smoke-keke-1; mei smoke-mei-1, startup-mei-1,
y26-ref201-1, y26t-ref201-1, lint-1, many-1, dyn22-1, y19-1, y26-1, y26t-1 (precheck on n1 exit 0 for the keke ones;
`ops handback-check` ok). Evidence only, not named: debs-1, roots-1, yohane-1, y10-1 end rc 1 on their known,
unchanged failures (rows above); pkgdiff-1, stage-1, install-1, startup-keke-1 ran scripts under `~/work/` against
the staged assets, so jobrun kept no tested-content record for them (precheck: "not judged"), all rc 0. gate-1 is
superseded by gate-2 for the same reason.

Products: this run's products.tgz has 1205 entries vs 1233 for 20.1's: 20.1's held leftover test artefacts of its
role (`target/niji/native/`, `target/niji/rust/`, `rust-test/`), not `xtask so` products. Not a finding.

## #26 end to end (swapped trixie root: gcc-14 / libasan8 / liblsan0 / libtsan2 / libubsan1 14.2.0-19)

The issue's reproducer (`gcc -fsanitize=address`, an 8-byte malloc written one past its end), stock-built, run on
stock glibc and on libniji's swapped packages. Addresses and pids masked (`0xX`, `==N==`); full logs in mei
`~/work/RELEASE-SNAPSHOT-1120B/logs-y26/out/{stock,niji}-asan.log`.

Stock (rc 1, 48 lines):

```
main reached
=================================================================
==N==ERROR: AddressSanitizer: heap-buffer-overflow on address 0xX at pc 0xX bp 0xX sp 0xX
WRITE of size 1 at 0xX thread T0
    #0 0xX in main /tmp/y/asan.c:5
    #1 0xX  (/lib/x86_64-linux-gnu/libc.so.6+0xX) (BuildId: 5e5b51f4c39af9ac589408c1dc432ba390a273cc)
    #2 0xX in __libc_start_main (/lib/x86_64-linux-gnu/libc.so.6+0xX) (BuildId: 5e5b51f4c39af9ac589408c1dc432ba390a273cc)
    #3 0xX in _start (/tmp/y/sb/asan+0xX) (BuildId: 7a6a89657289da1a1fecbce86b34a5fe23226bf8)

0xX is located 0 bytes after 8-byte region [0xX,0xX)
allocated by thread T0 here:
    #0 0xX in malloc ../../../../src/libsanitizer/asan/asan_malloc_linux.cpp:69
    #1 0xX in main /tmp/y/asan.c:5
    #2 0xX  (/lib/x86_64-linux-gnu/libc.so.6+0xX) (BuildId: 5e5b51f4c39af9ac589408c1dc432ba390a273cc)

SUMMARY: AddressSanitizer: heap-buffer-overflow /tmp/y/asan.c:5 in main
Shadow bytes around the buggy address:
  (11 shadow rows, `=>0xX: fa fa 00[fa]fa ...`, and the 17-line shadow byte legend)
```

libniji 3f2a84520 (rc 1, 49 lines; the diff against stock is only these frames, the shadow rows and legend are
identical):

```
main reached
=================================================================
==N==ERROR: AddressSanitizer: heap-buffer-overflow on address 0xX at pc 0xX bp 0xX sp 0xX
WRITE of size 1 at 0xX thread T0
    #0 0xX in main /tmp/y/asan.c:5
    #1 0xX  (/lib/x86_64-linux-gnu/libc.so.6+0xX) (BuildId: ba44c638db655aec8d8b25c65a88ac20d65cf6c4)
    #2 0xX  (/lib/x86_64-linux-gnu/libc.so.6+0xX) (BuildId: ba44c638db655aec8d8b25c65a88ac20d65cf6c4)
    #3 0xX in __libc_start_main (/lib/x86_64-linux-gnu/libc.so.6+0xX) (BuildId: ba44c638db655aec8d8b25c65a88ac20d65cf6c4)
    #4 0xX in _start (/tmp/y/sb/asan+0xX) (BuildId: 7a6a89657289da1a1fecbce86b34a5fe23226bf8)

0xX is located 0 bytes after 8-byte region [0xX,0xX)
allocated by thread T0 here:
    #0 0xX in malloc ../../../../src/libsanitizer/asan/asan_malloc_linux.cpp:69
    #1 0xX in main /tmp/y/asan.c:5
    #2 0xX  (/lib/x86_64-linux-gnu/libc.so.6+0xX) (BuildId: ba44c638db655aec8d8b25c65a88ac20d65cf6c4)

SUMMARY: AddressSanitizer: heap-buffer-overflow /tmp/y/asan.c:5 in main
Shadow bytes around the buggy address:
  (the same 11 shadow rows and legend)
```

Same shape as stock: `thread T0`, the allocation stack, `SUMMARY`, rc 1. One extra libc frame (#2) between `main`
and `__libc_start_main`: libniji's start-up has one more frame there. INFORMATIONAL, not a test failure (gcc's tests
match `#0 ... in main` and the SUMMARY line; the sample below passes 20/20).

| sanitizer (`logs-y26/out/{stock,niji,nijibuilt}.txt`) | stock | libniji, stock-built | libniji-linked | matches stock |
|---|---|---|---|---|
| ASan | `ERROR: AddressSanitizer: heap-buffer-overflow`, rc 1 | same, full report | same | yes |
| LSan | `ERROR: LeakSanitizer: detected memory leaks`, rc 23 | same | same | yes |
| TSan, ASLR on | `FATAL: ThreadSanitizer: unexpected memory mapping`, rc 66, main not reached | same | `WARNING: ThreadSanitizer: data race`, rc 66 | yes (stock-built); libniji-linked runs where stock does not |
| TSan, ASLR off (`setarch -R`) | `WARNING: ThreadSanitizer: data race`, rc 66 | same | same | yes |
| UBSan | `ubsan.c:5:75: runtime error: signed integer overflow ...`, rc 0 | same | same | yes |

**gcc ASan test sample (y26t).** Issue #26 names Yohane's failing directories (gcc-14's `c-c++-common/asan`,
`gcc.dg/asan`, `g++.dg/asan`, `gfortran.dg/asan`), not a per-test list. Sample: gcc 14.2.0 sources (tag
releases/gcc-14.2.0, matching trixie's 14.2.0-19), the 40 `dg-do run` C tests of the first two directories with no
directive beyond dg-do/dg-options/dg-shouldfail/dg-output, sorted, every second = 20 (`cc_alloca_big_alignment`,
`cc_alloca_instruments_all_paddings`, `cc_alloca_overflow_right`, `cc_alloca_underflow_left`, `cc_asan-stack-small`,
`cc_bitfield-2`, `cc_bitfield-4`, `cc_heap-overflow-1`, `cc_pr59063-1`, `cc_pr61530`, `cc_pr63888`, `cc_pr71480`,
`cc_pr80308`, `cc_pr99168`, `cc_stack-overflow-1`, `cc_use-after-free-1`, `gd_pr78541`, `gd_pr81186`,
`gd_use-after-scope-11`, `gd_use-after-scope-4`). Built in the trixie root with `gcc -fsanitize=address -g -O0` +
dg-options (one torture level), run once; `judge.py` applies dg-shouldfail (exit status) and the joined dg-output
patterns (DejaGnu regexp semantics).

| packages | stock-built on libniji | libniji-linked |
|---|---|---|
| stock glibc | 20 / 20 | - |
| snapshot-2026-11-20.1 | 14 / 20 | 14 / 20 |
| 3f2a84520 | 20 / 20 | 20 / 20 |

20.1 failed exactly the six tests whose dg-output needs `thread T0` or the allocation stack
(`alloca_big_alignment`, `alloca_overflow_right`, `alloca_underflow_left`, `heap-overflow-1`, `stack-overflow-1`,
`use-after-free-1`). The other two directories (`g++.dg/asan`, `gfortran.dg/asan`) were not sampled.

## Start-up (execcount: user instructions / page faults, exec to exit, ASLR off, median of 101)

| program | class | glibc 2.39 | snapshot-2026-11-20.1 | 3f2a84520 |
|---|---|---|---|---|
| empty main | Skylake-SP | 107,660 / 49 | 782,439 / 59 | 785,924 / 57 |
| 3 libraries | Skylake-SP | 142,876 / 65 | 1,034,029 / 75 | 1,037,514 / 73 |
| clock_gettime | Skylake-SP | 109,770 / 50 | 816,379 / 63 | 819,864 / 61 |
| empty main | Zen 1 | 101,479 / 48 | 782,242 / 59 | 785,727 / 57 |
| 3 libraries | Zen 1 | 136,630 / 64 | 1,033,840 / 75 | 1,037,325 / 73 |
| clock_gettime | Zen 1 | 103,618 / 50 | 816,211 / 64 | 819,697 / 62 |

Product paths of equal length in every compared tree (jobs -startup-mei-1, -startup-keke-1).

## Findings (INFORMATIONAL; none blocking)

1. **niji6 / niji6-dev version order.** The coexist packages are versioned `0.0~snapshot<commit UTC date>+<12-hex
   commit>` (tools/deb/build.sh line 128). Both 20.1 and this snapshot fall on 2026-11-20, so the order is decided by
   the commit hex: `dpkg --compare-versions 0.0.snapshot20261120+3f2a84520118 gt 0.0.snapshot20261120+50abc44ed51f`
   is false. An installed 20.1 niji6 is "newer" than 20.2's, so upgrading with `apt install ./niji6_...deb` (the
   documented command) offers a downgrade (apt's behaviour NOT VERIFIED here; only dpkg's order was checked). The
   system packages are unaffected (`2.39+niji20261120.2-1` > `.1-1`). Any two same-day snapshots are ordered by
   their commit hex, so the second sorts lower about half the time (earlier same-day pairs not checked). Route to a packaging story: put the serial or the commit time in the coexist version (e.g.
   `0.0~snapshotYYYYMMDD.N+<hex>`). No packaging changed in this run (brief).
2. **Start-up +3,485 instructions** on every program and both classes (+0.3-0.45%), 2 fewer page faults, vs 20.1.
   Not traced to one story (ASAN-MAIN-THREAD's start-up words or LAYOUT-BYTE-BUDGETS-3 are the candidates).
3. **ASan stack: one extra libc frame** between `main` and `__libc_start_main` (above); the shape and every line
   gcc's tests match are as stock.
4. Unchanged from 20.1: dbg `Breakpoint 1.1`; bare-name dlopen through `$ORIGIN` DT_RUNPATH (y10); libc6-dev ships
   no C headers (yohane fail=2; the #26 probes compile on the stock root); DBG 17 of 19 (ruling #6321);
   `_dl_get_tls_static_info@@NIJI_PRIVATE`.
5. **Docs** (route to a docs story; install-packages.md passes against the staged assets, so not blocking): no user
   doc changed since 20.1 (README, RELEASE-NOTES, docs/RELEASE-NOTES, differences-from-glibc, compat-flags,
   install-packages, spec-symbol-gaps, tools/deb: no diff; architecture.md and the codemap README are developer
   docs). docs/RELEASE-NOTES.md still ends at 2026-11-17; its Known limitations (line 74) still say "no debugger
   rendezvous (`_r_debug`) ... yet"; no user doc mentions the sanitizers. doccheck: the same 23 broken-path lines as
   20.1. **User-visible change since snapshot-2026-11-20.1:** "under libniji's ld.so, gcc's AddressSanitizer reports
   name the main thread T0 and print the allocation/free stacks and the SUMMARY line (ASAN-MAIN-THREAD, D-22.20;
   GitHub #26)". Start-up cost: finding 2.

## Staged assets (keke `~/release-staging/snapshot-20261120.2/`, private)

libc6, libc6-dbg, libc6-dev, libc-bin, libc-dev-bin (`_2.39+niji20261120.2-1_amd64.deb`), libc-l10n, locales
(`_2.39+niji20261120.2-1_all.deb`), niji6, niji6-dev (`_0.0.snapshot20261120+3f2a84520118_amd64.deb`), SHA256SUMS,
MANIFEST. `sha256sum -c SHA256SUMS`: all OK. Nothing to GitHub, no tag.

## Exact next step

Coordinator: tag/publish snapshot-2026-11-20.2 from the staged directory at 3f2a84520 when the owner says so (tell
Yohane #26 is complete: full ASan report, 20/20 of the sample); route finding 1 to a packaging story and finding 5 to a
docs story. Not to redo: anything above. Plants: handoff/RELEASE-SNAPSHOT-1120B-plants.txt. Report:
handoff/reports/RELEASE-SNAPSHOT-1120B.json.
