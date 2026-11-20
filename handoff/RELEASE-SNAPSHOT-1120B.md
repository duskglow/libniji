IN PROGRESS (leg 1 of RELEASE-SNAPSHOT-1120B; a refresh leg continues): validation of 3f2a84520 for snapshot-2026-11-20.2

# RELEASE-SNAPSHOT-1120B: snapshot validation of 3f2a84520 (GitHub #26 complete)

**State.** Validated head `3f2a8452011850988cb6d0a61ffaf283241aad2e` "Regenerate train-owned views" (origin/next at
the start, 2026-11-20 10:40 UTC; commit time 09:45:50 UTC, epoch 1795167950). `git merge-base --is-ancestor
3f2a84520` holds (it is the head; ASAN-MAIN-THREAD's merge is 458e9c223). Branch `task/release-snapshot-1120b` = that
head + handoff/ only (no product or packaging code changed). Role keke `~/w/n1` (Zen 1); mei = Skylake-SP.

**Scripts.** keke and mei `~/work/RELEASE-SNAPSHOT-1120B/`, made on mei by `copy.sh` + `subst.sed` (both in that
dir) from mei `~/work/RELEASE-SNAPSHOT-1120/` (the full set: *.sh, d/ m/ y/ y10/ y19/ y26/ startup/, the
`binutils-src` link to 1119B's input), then copied unchanged to keke. Changed: head, epoch, serial 20261120.2, role
keke n1 (chain-role builds in keke and runs `smoke-keke-1`; mei's smoke runs on an unpacked tree `tree/` with keke's
products), labels, staging dir, pkgdiff against snapshot-20261120.1, startup sets p201 (20.1's products.tgz) and p202
(this head's), equal-length paths. `tree/` = `git archive 3f2a84520`. New this run: `y26t.sh` + `y26t/` (y26.sh's
recipe; 20 gcc 14.2.0 ASan tests and `judge.py`, below). Local scratch: `~/w/.scratch/RELEASE-SNAPSHOT-1120B/`
(gcct/ = the downloaded test sources and sample.txt, doccheck.py/.txt, y26t/).

**Landed since snapshot-2026-11-20.1** (`git log --oneline 50abc44ed..3f2a84520`: 38 lines; first-parent merges):
pass2-native-poison, release-snapshot-1120 (handoff only), mp-isqrt-fast, layout-byte-budgets-3,
test-reloc-etxtbsy, **asan-main-thread** (GitHub #26 second half, D-22.20), train view regenerations.
User docs unchanged since 20.1 (README, RELEASE-NOTES, docs/RELEASE-NOTES, differences, compat-flags,
install-packages, spec-symbol-gaps, tools/deb: no diff); developer docs: architecture.md, codemap README.

## Results so far (all on 3f2a84520's products)

| check | job | result | vs 20.1 |
|---|---|---|---|
| smoke lua_reload, cxx_dlclose | keke -smoke-keke-1 (builds products in n1), mei -smoke-mei-1 | `SMOKE: RESULT bad=0` both; libc.so.6 53768253..., ld.so 42a432dd... | same (new build) |
| gate | keke -gate-1 rc 0 | clippy --all-features, xtask clippy, static, records check, affected --base origin/next (0 of 303 targets): all RC 0 | same |
| system debs, check-system | keke -debs-1 | `check: PASS (7 packages, version 2.39+niji20261120.2-1)` | same |
| check-symbols | debs-1 | PASS, 4440 symbol lines, 13 libraries (20.1: 4440) | same |
| coexist debs + check | debs-1 | `check: PASS (2 packages)` | same |
| debs-1 rc 1 | debs-1 | DBG only: 17 of 19 ELF objects; no libc6-dbg file for `/usr/bin/iconv`, `/usr/bin/locale` (ruling #6321) | same |
| package comparison | keke -pkgdiff-1 rc 0 | 9 packages, file counts all unchanged (libc6 75, libc6-dbg 44, libc6-dev 38, libc-bin 22, niji6 29, niji6-dev 48, others 7); libc6-dbg: 6 build-ids replaced (06 39 42 4f 9e b2 gone; 0e 52 6e 7c ba fb new); control fields: versions only; sizes within 11 KB | same |
| stage | keke -stage-1 rc 0 | 9 assets + SHA256SUMS + MANIFEST in keke `~/release-staging/snapshot-20261120.2/` | - |
| install instructions | keke -install-1 rc 0 | `inst: RESULT fails=0` | same |
| start-up | keke -startup-keke-1, mei -startup-mei-1 | below | +3,485 instr, -2 faults |
| #26 on 20.1 (reference) | mei -y26-ref201-1 rc 0 | `YOHANE-26 pass=20 fail=0` (its checks see first lines only) | - |
| #26 gcc ASan sample on 20.1 | mei -y26t-ref201-1 rc 0 | judge: stock 20/20, 20.1 niji 14/20, nijibuilt 14/20 | - |
| docs | local doccheck.py on tree/ | 23 problems, the same 23 lines as 20.1 | same |

Products: this run's products.tgz has 1205 entries vs 1233 for 20.1's: 20.1's held leftover test artefacts of its
role (`target/niji/native/`, `target/niji/rust/`, `rust-test/`), not `xtask so` products. Not a finding.

**Start-up** (execcount, user instructions / page faults, exec to exit, ASLR off, median of 101):

| program | class | glibc 2.39 | snapshot-2026-11-20.1 | 3f2a84520 |
|---|---|---|---|---|
| empty main | Skylake-SP | 107,660 / 49 | 782,439 / 59 | 785,924 / 57 |
| 3 libraries | Skylake-SP | 142,876 / 65 | 1,034,029 / 75 | 1,037,514 / 73 |
| clock_gettime | Skylake-SP | 109,770 / 50 | 816,379 / 63 | 819,864 / 61 |
| empty main | Zen 1 | 101,479 / 48 | 782,242 / 59 | 785,727 / 57 |
| 3 libraries | Zen 1 | 136,630 / 64 | 1,033,840 / 75 | 1,037,325 / 73 |
| clock_gettime | Zen 1 | 103,618 / 50 | 816,211 / 64 | 819,697 / 62 |

INFORMATIONAL: +3,485 instructions per start-up on every program and both classes (+0.3-0.45%), 2 fewer faults.
Not traced to one story (ASAN-MAIN-THREAD's start-up words or LAYOUT-BYTE-BUDGETS-3 are the candidates).

**y26t (step 3 sample).** Issue #26 names Yohane's failing directories (gcc-14's `c-c++-common/asan`,
`gcc.dg/asan`, `g++.dg/asan`, `gfortran.dg/asan`), not a per-test list. Sample: gcc 14.2.0 sources
(gcc-mirror tag releases/gcc-14.2.0, matching trixie's 14.2.0-19), the 40 `dg-do run` C tests of the first two
directories with no directive beyond dg-do/dg-options/dg-shouldfail/dg-output, sorted, every second = 20. Built in
the trixie root with `gcc -fsanitize=address -g -O0` + dg-options (one torture level), run once; `judge.py` applies
dg-shouldfail (exit status) and the joined dg-output patterns (DejaGnu regexp semantics) to stock, libniji
(stock-built) and libniji-linked. 20.1 fails exactly the 6 tests that need `thread T0` / the allocation stack.

## Jobs still running at the end of leg 1 (started 11:35-11:40 UTC)

mei: release-snapshot-1120b-roots-1, -lint-1, -yohane-1, -many-1, -dyn22-1, -y10-1, -y19-1, -y26-1, -y26t-1.
keke: release-snapshot-1120b-plants-1 (`cargo xtask plants handoff/RELEASE-SNAPSHOT-1120B-plants.txt --dir
/dev/shm/claude/RELEASE-SNAPSHOT-1120B/plants-1`; it printed WARN-DEV-SHM, the brief's Zen 1 directory).

## Exact next step (leg 2)

1. mei: `~/bin/jobwait release-snapshot-1120b-{roots,lint,yohane,many,dyn22,y10,y19,y26,y26t}-1` (one call, <= 600 s;
   rc 3 = call again); keke: `~/bin/jobwait release-snapshot-1120b-plants-1`. Read each log's RESULT lines only
   (grep), compare with 20.1's: mei `~/work/RELEASE-SNAPSHOT-1120/logs-*` and the 1120 table in
   handoff/RELEASE-SNAPSHOT-1120.md (roots: root pass=19, swap `same=63 known=2 bad=0`, dbg pass=8 fail=1 both,
   fresh fail=0; lint 10 E / 60 W each, normalised lines identical; coexist proofs pass=21 x2; many pass=9; yohane
   masked lines as 19.2; dyn22 fail=0; y10 19 PASS / 1 FAIL bare-name dlopen; y19 fail=0 pass=14).
2. #26 in full: mei `~/work/RELEASE-SNAPSHOT-1120B/logs-y26/out/stock-asan.log` and `niji-asan.log` (masked) in full
   into the report; LSan/TSan(setarch -R)/UBSan lines from `logs-y26/out/{stock,niji}.txt` vs stock. y26t-1 judge
   lines (stock / 20.1 / this head pass counts).
3. Append plants results to handoff/RELEASE-SNAPSHOT-1120B-plants.txt (as the 1120 file did); rewrite this note as
   the final report (first line `COMPLETE: snapshot-ready at 3f2a84520` or the blocking findings; the results
   table, #26 reports, start-up table, staged assets: libc6, libc6-dbg, libc6-dev, libc-bin, libc-dev-bin
   (`_2.39+niji20261120.2-1_amd64.deb`), libc-l10n, locales (`_all.deb`), niji6, niji6-dev
   (`_0.0.snapshot20261120+3f2a84520118_amd64.deb`), SHA256SUMS, MANIFEST). Docs findings as in 1120 (release
   notes end at 2026-11-17; Known limitations line 74 still says no `_r_debug`; add: nothing mentions the
   sanitizers). User-visible change since 20.1: "under libniji's ld.so, gcc's AddressSanitizer reports name the
   main thread T0 and print the allocation/free stacks and the SUMMARY line (ASAN-MAIN-THREAD, D-22.20; GitHub #26)".
4. handoff/reports/RELEASE-SNAPSHOT-1120B.json (with "jobs"), commit, push, sync keke n1, `cargo xtask precheck
   --jobs <labels> --head <sha>` on n1, `ops handback-check`, hand back. JOBS (host:label): keke smoke-keke-1,
   gate-1, debs-1, pkgdiff-1, stage-1, install-1, startup-keke-1, plants-1; mei smoke-mei-1, startup-mei-1,
   y26-ref201-1, y26t-ref201-1 and the nine above (all prefixed release-snapshot-1120b-).
