WIP (leg 1 of RELEASE-SNAPSHOT-1122A; refresh at ~09:35 UTC): products, gate, start-up counts and the getaddrinfo_a
load case done on 9548c734e; the .debs build (debs-1) and everything that depends on it are still running or to start.

# RELEASE-SNAPSHOT-1122A: snapshot validation of 9548c734e (LIBC-SELF-BIND-2, LD-LOOKUP-COST)

**State.** Head under validation `9548c734eea6af843ad36865a44a1af8cc3d6220` (origin/next at the start, 08:33 UTC;
"Regenerate train-owned views"; commit time 2026-11-22T07:35:33Z, epoch 1795332933). Commit date 2026-11-22 =
**serial 1 of 2026-11-22**: `NIJI_DEB_SERIAL=1`, `2.39+niji20261122.1-1`, staged as `snapshot-20261122.1`. Branch
`task/release-snapshot-1122a` = that head + handoff/ only. Role **keke `~/w/r2` (Zen 1)**; mei = Skylake-SP (test jobs
on keke's products and .debs, sha256 checked on arrival). 21.7 = 2c64fde8a (RELEASE-SNAPSHOT-1121F; its products and
.debs are in `~/work/RELEASE-SNAPSHOT-1121F/` on both hosts; staged keke `~/release-staging/snapshot-20261121.7/`).
Deadline 2026-11-22T12:32:44Z.

**Landed since 21.7** (`git log --first-parent 2c64fde8a..9548c734e`): test-scratch-cleanup-4 (5a9b5d0a1),
mp-ln10-series (fee055126), sl-headers-s12-7 (b9d53ef9e), dl-runpath-secure-test (859349661), view regen (90390071c),
oracle-value-cache (90db37d78), release-snapshot-1121f (5a3e0985a), ld-lookup-cost (751bf7ddf), wl-levels-rerecord
(6403f8ae0), ci-instr-rebuilds (194586919), view regen (c9f54d512), dl-slot-reuse-2 (1d81eb91c), view regen
(fa79409c7), ziv-test-isolation (58783f973), sl-headers-s16-5 (47caa0de7), view regen (8a4f7ad91), **libc-self-bind-2
(c716aab6f)**, root-harness-contain-3 (184144797), tz-secure-path (312533572), view regen (9548c734e).

**Scripts** (keke and mei `~/work/RELEASE-SNAPSHOT-1122A/`; local copies in this run's scratch): 1121F's set made by
copy.sh + subst.sed on mei (head, epoch, serial 1, labels, p217 = 21.7 / p218 = head, ref216 -> ref217, role `w/n3` ->
`w/r2`), copied to keke; keke's run-keke.sh, run-gai.sh, gaimass.sh, cmp5.sh substituted there from keke's 1121F copies.
tree.tgz = `git archive 9548c734e` (9082 files = ls-tree; sha checked on both). Plants file
handoff/RELEASE-SNAPSHOT-1122A-plants.txt (1121F's six; each pattern present once in the head). **New: mint.sh +
mint/** (malloc-replacement probe, below). Comparison: scratch `cmp.py OLDDIR NEWDIR check...` (cmp4/5's filter and
mask, 2026-11-22 added; 1121F's logs fetched into scratch `old/`, this run's into `new/`); start-up tables: scratch
`tables.py <dir with startup|kstart|rstart-{mei,keke}-1.log>`.

## Results so far (all on 9548c734e's products)

| check | job | result | vs 21.7 |
|---|---|---|---|
| smoke | keke smoke-keke-1 (build), mei smoke-mei-1 | `SMOKE: RESULT bad=0` both | same (the role path in the layout-cache line) |
| gate | keke gate-1 rc 0 | clippy all-features, xtask clippy, static, records check, affected: `GATE bad=0`, peak 1.8G | same |
| signal 32 / #20 / 600 libraries | mei sig32-1, y20-1, dpo600-1 | `fail=0` each; 600 libraries load (604 objects) | same (set labels only) |
| getaddrinfo_a | keke gaimass-1 rc 0 | dns_products 2 passed both sets; host, libniji cache on and off: `done: ok=25000 other=0 answer 192.0.2.10 only=25000; signals: 5000` (head and 21.7) | same; finding T1 |
| malloc replacement, 21.7 side | mei mint-ref217-2 rc 0 | `MINT fail=0`, 40 internal rows differ from stock (finding M) | baseline |

SUPERSEDED: mei mint-ref217-1 (rc 1: the relinked variants failed to compile after the swap, libniji's libc6-dev ships
no C headers, as y26 notes; mint now compiles objects on the stock root and only links after the swap; and its judge
treated internal allocations as failures; rerun as -2).

### Start-up (execcount, ASLR off, median of 101; startup/kstart/rstart-{mei,keke}-1, all rc 0)

User instructions / page faults. 21.7's numbers reproduce 1121F's exactly.

| program | class | glibc 2.39 | snapshot-2026-11-21.7 | 9548c734e | change from 21.7 |
|---|---|---|---|---|---|
| empty main | Skylake-SP | 107,752 / 49 | 726,461 / 57 | 324,068 / 56 | -402,393 (-55.4%) / -1 |
| empty main | Zen 1 | 101,533 / 48 | 726,223 / 57 | 323,829 / 56 | -402,394 (-55.4%) / -1 |
| 3 libraries | Skylake-SP | 143,094 / 65 | 966,077 / 72 | 486,648 / 71 | -479,429 (-49.6%) / -1 |
| 3 libraries | Zen 1 | 136,734 / 64 | 965,824 / 72 | 486,394 / 71 | -479,430 (-49.6%) / -1 |
| clock_gettime | Skylake-SP | 109,862 / 50 | 748,574 / 60 | 338,401 / 59 | -410,173 (-54.8%) / -1 |
| clock_gettime | Zen 1 | 103,672 / 50 | 748,365 / 61 | 338,191 / 60 | -410,174 (-54.8%) / -1 |

Kernel instructions / syscalls (kernel counts move by up to ~50k between runs; syscalls exact):

| program | class | glibc 2.39 | 21.7 | 9548c734e | change from 21.7 |
|---|---|---|---|---|---|
| empty main | Skylake-SP | 687,249 / 46 | 743,796 / 29 | 738,146 / 29 | -5,650 / 0 |
| empty main | Zen 1 | 701,397 / 42 | 762,953 / 28 | 757,026 / 28 | -5,927 / 0 |
| 3 libraries | Skylake-SP | 981,335 / 72 | 1,147,590 / 59 | 1,138,486 / 59 | -9,104 / 0 |
| 3 libraries | Zen 1 | 999,357 / 68 | 1,175,629 / 58 | 1,166,383 / 58 | -9,246 / 0 |
| clock_gettime | Skylake-SP | 691,453 / 46 | 776,846 / 29 | 769,860 / 29 | -6,986 / 0 |
| clock_gettime | Zen 1 | 707,456 / 42 | 797,567 / 28 | 792,242 / 28 | -5,325 / 0 |

libresolv pair (rstart; res0 needs libresolv.so.2, pie0 empty):

| class | row | glibc 2.39 | 21.7 | 9548c734e | change from 21.7 |
|---|---|---|---|---|---|
| Skylake-SP | pie0 / res0 user | 107,645 / 169,020 | 726,429 / 889,150 | 324,036 / 452,124 | -402,393 / -437,026 |
| Skylake-SP | res0 faults; kernel / syscalls | 55; 818,760 / 58 | 62; 940,228 / 42 | 61; 926,230 / 42 | -1; -13,998 / 0 |
| Skylake-SP | res0 - pie0 user | 61,375 | 162,721 | 128,088 | -34,633 |
| Zen 1 | pie0 / res0 user | 101,481 / 162,716 | 726,140 / 888,599 | 323,747 / 451,572 | -402,393 / -437,027 |
| Zen 1 | res0 faults; kernel / syscalls | 54; 843,017 / 54 | 62; 961,048 / 40 | 61; 948,959 / 40 | -1; -12,089 / 0 |
| Zen 1 | res0 - pie0 user | 61,235 | 162,459 | 127,825 | -34,634 |

libc.so.6 relocations (readelf -r, the two products.tgz): 21.7 1,606 RELATIVE + 288 others (210 JUMP_SLOT, 75
GLOB_DAT, 3 R_X86_64_64); head 1,609 RELATIVE + 55 (4 JUMP_SLOT, 48 GLOB_DAT, 3 R_X86_64_64). The symbolic ones that
remain name the malloc family (malloc, free, calloc, realloc, reallocarray, posix_memalign, aligned_alloc), data
(stdin/stdout/stderr, environ, optarg..., __tzname...) and _init/_fini/array bounds.

## Malloc replacement (mint.sh, new)

mint/alloc.c replaces malloc, free, calloc, realloc, posix_memalign, aligned_alloc, memalign, valloc, pvalloc and
malloc_usable_size with a counting bump allocator; mint/main.c runs 24 phases (strdup, strndup, asprintf,
reallocarray, getline, getdelim, open_memstream, realpath, getcwd, scandir: memory handed to the caller; glob,
wordexp, regcomp, tsearch, qsort of 200,000 ints, getpwnam, getaddrinfo, opendir, setlocale, iconv_open,
pthread_create, dlopen: libc's own state; fopen/fclose, fmemopen: the stream). Variants: exe (allocator in the
program), so (DT_NEEDED libmint.so), pre (that + LD_PRELOAD), exe_r / so_r (the stock objects linked again on
libniji). Trixie root via rootrun.sh (y13s's recipe). judge.py: caller rows must reach the replacement and return
its memory (own=1) as on stock; no foreign frees; internal rows reported (DIFFERS) and, with the previous snapshot's
log, compared row by row (CHANGED). On the host's glibc 2.39 every phase reaches the replacement.

## Findings so far

- **M (INFORMATIONAL, pre-existing, a premise challenge):** the brief's "it must still see every malloc call the
  reference shows, except fopen/fclose's stream" does not hold on 21.7 either: under libniji the replacement sees no
  call in qsort (200,000 ints), opendir, setlocale, iconv_open, pthread_create and dlopen (stock: 1, 1, 194, 17, 1, 13
  calls), nor for fopen/fclose and fmemopen (the known STDIO-CALLER-ALLOC); caller-visible memory (strdup ... scandir)
  is the replacement's on both. The gate for this snapshot is therefore "no row changed from 21.7" (CHANGED = 0).
- **T1 (INFORMATIONAL, tooling):** the head's test harness writes its oracle output under
  `target/test-oracles/resolv_diff-*` (was `resolv-*`) and dns_products has 2 tests (21.7: 3;
  `a_killed_run_leaves_no_process` left with ROOT-HARNESS-CONTAIN-3); gaimass.sh's evidence grep found nothing, the
  numbers above were read from the copies it saved (`gai-p218/`, `gai-p217/`).
- **S1 (INFORMATIONAL):** the head's empty main is 324,068 / 323,829 against LIBC-SELF-BIND-2's merged-branch
  323,388 / 323,179 (+680 / +650; three libraries +944 / +891): the train merged TZ-SECURE-PATH and
  ROOT-HARNESS-CONTAIN-3 beside it; not examined.

## Running at the refresh (labels prefixed release-snapshot-1122a-)

keke: `run-keke.sh` (detached; out `~/work/RELEASE-SNAPSHOT-1122A/run-keke.out`) is in **debs-1** (started ~09:25);
after it, the script writes `debs.sha`, starts plants-1 (role r2, `/dev/shm/claude/release-snapshot-1122a/plants-1`)
and pkgdiff-1, then stage-1 and install-1 (chain-keke.sh). mei: `run-mei.sh` (detached; out `run-mei.out`) products
part done; its debs part polls for `~/work/RELEASE-SNAPSHOT-1122A/debs.sha` on mei and then starts roots, lint,
yohane, many, dyn22, y10, y19, y26, y26t, y23, c19, y25, y24, y28, y6, y25s, y21, yres, y23p, dlc, y27, y13, y14, y6c,
y14r -> eetwet, and y13s / drp / y6r each with a `-ref217-1` run on 21.7's .debs.

## Exact next steps

1. `~/bin/jobwait --max 580 release-snapshot-1122a-debs-1` on keke until rc 0 (then check `debs.sha` exists).
2. Relay the .debs keke -> mei from the local machine (debs.sha LAST, checked):
   keke `cd ~/work/RELEASE-SNAPSHOT-1122A && tar cf - debs/system/*.deb debs/coexist/*.deb` piped to mei
   `cd ~/work/RELEASE-SNAPSHOT-1122A && tar xf -`; then keke `cat debs.sha` piped to mei `cat > debs.sha.tmp && (cd
   debs && sha256sum -c ../debs.sha.tmp) && mv debs.sha.tmp debs.sha`. run-mei.sh's debs part then starts by itself.
3. On mei start (setsid nohup, `</dev/null`): `~/bin/jobrun release-snapshot-1122a-mint-1 test -- sh
   ~/work/RELEASE-SNAPSHOT-1122A/mint.sh ~/work/RELEASE-SNAPSHOT-1122A/debs/system
   ~/work/RELEASE-SNAPSHOT-1122A/logs-mint ~/work/RELEASE-SNAPSHOT-1122A/logs-mint-ref217` (expect fail=0,
   changed_from_previous=0; any CHANGED or FAIL row is a finding about LIBC-SELF-BIND-2).
4. jobwait keke plants-1, pkgdiff-1, stage-1, install-1 and the mei jobs; fetch logs into scratch `new/` and run
   `cmp.py old new <checks>` (1121F's logs are in scratch `old/`; the -ref217 runs compare with 1121F's -ref216).
   Evidence-only (rc 1 on known lines in 1121F): roots, yohane, y14, y14r, drp; the -ref217 runs rc 1 by design.
5. Finish this file (results table for every check, staged list from stage-1, findings), write
   handoff/reports/RELEASE-SNAPSHOT-1122A.json with "jobs", commit, push, sync r2, precheck on r2
   (`cargo xtask precheck --jobs <labels> --head <sha>`), `ops handback-check`, hand back.
