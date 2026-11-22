COMPLETE: snapshot-ready at 9548c734e (snapshot-2026-11-22.1, serial 1 of the commit date 2026-11-22)

# RELEASE-SNAPSHOT-1122A: snapshot validation of 9548c734e (LIBC-SELF-BIND-2, LD-LOOKUP-COST)

**State.** Head under validation `9548c734eea6af843ad36865a44a1af8cc3d6220` (origin/next at the start, 08:33 UTC;
"Regenerate train-owned views"; commit time 2026-11-22T07:35:33Z, epoch 1795332933). Commit date 2026-11-22 =
**serial 1 of 2026-11-22**: `NIJI_DEB_SERIAL=1`, `2.39+niji20261122.1-1` (coexist `0.0~snapshot20261122.1+9548c734eea6`),
both `dpkg --compare-versions gt` 21.7's. Branch `task/release-snapshot-1122a` = that head + handoff/ only. Role
**keke `~/w/r2` (Zen 1)**; mei = Skylake-SP (jobs on keke's products and .debs, sha256 checked on arrival). 21.7 =
2c64fde8a (RELEASE-SNAPSHOT-1121F). Nothing to GitHub, no tag.

**Landed since 21.7** (`git log --first-parent 2c64fde8a..9548c734e`): test-scratch-cleanup-4, mp-ln10-series,
sl-headers-s12-7, dl-runpath-secure-test, oracle-value-cache, release-snapshot-1121f, ld-lookup-cost, wl-levels-rerecord,
ci-instr-rebuilds, dl-slot-reuse-2, ziv-test-isolation, sl-headers-s16-5, **libc-self-bind-2 (c716aab6f)**,
root-harness-contain-3, tz-secure-path, and four view regenerations (last 9548c734e).

**Procedure.** 1121F's scripts (keke and mei `~/work/RELEASE-SNAPSHOT-1122A/`), changed in head, epoch, serial, role and
labels only; plants file handoff/RELEASE-SNAPSHOT-1122A-plants.txt (1121F's six). New: mint.sh + mint/ (malloc
replacement probe, below). Comparison: per check, the masked PASS/FAIL/RESULT/verdict lines of 1121F's log against this
run's (scratch cmp.py); the `-ref217` runs repeat three probes on 21.7's .debs.

## Results (all on 9548c734e's products and .debs; job labels prefixed `release-snapshot-1122a-`)

| check | jobs | result | vs 21.7 |
|---|---|---|---|
| smoke | keke smoke-keke-1, mei smoke-mei-1 | `SMOKE: RESULT bad=0` both | same (role path in one line) |
| gate (direct tier, clippy all-features, static, records) | keke gate-1 | `GATE bad=0`, peak 1.8G | same |
| .debs | keke debs-1 | `DEBS bad=0`; check PASS 7 packages; check-symbols PASS 4440 lines, 13 libraries; DBG 21 of 21 | same |
| plants | keke plants-1 | 6 of 6 CAUGHT (debug-rows-unstripped, postinst-configure-ldconfig, postrm-purge-cache, postrm-not-packaged, cxx-needs-libstdcxx, swap-not-done); no WARN | same (incl. the "caught by no test the log names" line) |
| package diff | keke pkgdiff-1 | file lists equal except libc6-dbg's .build-id names (50 -> 51 entries: one more 2-hex directory); versions and sizes | as expected |
| stage, install | keke stage-1, install-1 | staged below; mmdebstrap rc 0, system debs different: 0 | same |
| trixie root, libniji swapped in (every command) | mei roots-1 | rc 1 on the known lines | 121 lines, 0 differ |
| lint, yohane, many, dyn22 | mei lint-1, yohane-1, many-1, dyn22-1 | yohane rc 1 on the known lines | 0 differ each |
| Yohane loop checks | mei y10, y19, y26, y26t, y23, c19, y25, y24, y28, y6, y25s, y21, yres, y23p, dlc, y27, y13, y14, y6c, y14r (-1 each) | y14 / y14r rc 1 on the known lines | 0 differ each (y14: 1437 lines) |
| valgrind string redirect (#13) | mei y13s-1, y13s-ref217-1 | `YOHANE-13S fail=0` both | same |
| DL-RUNPATH | mei drp-1, drp-ref217-1 | fail=2 both (setgid `$ORIGIN` rows, the known secure-mode difference) | same |
| CJK iconv (y6r) | mei y6r-1, y6r-ref217-1 | pass=13 fail=0 both | same |
| tz EET/WET | mei eetwet-1 | the known Europe/Warsaw 1918 isdst row | same |
| signal 32 / #20 / 600 libraries | mei sig32-1, y20-1, dpo600-1 | `fail=0` each; 604 objects | same (set labels only) |
| getaddrinfo_a | keke gaimass-1 | ok=25000 other=0, 5000 signals, cache on and off | same; T1 |
| malloc replacement | mei mint-1 (head), mint-ref217-2 (21.7) | `MINT fail=0 internal_rows_differing_from_stock=40 changed_from_previous=0` | 0 rows changed; M |

SUPERSEDED: mei mint-ref217-1 (rc 1: the relinked variants did not compile after the swap, and its judge counted
internal allocations as failures; fixed and rerun as -2).

## Start-up (execcount, ASLR off, median of 101; startup/kstart/rstart-{mei,keke}-1, all rc 0)

User instructions / page faults. 21.7's numbers are the same as 1121F's.

| program | class | glibc 2.39 | snapshot-2026-11-21.7 | 9548c734e | change from 21.7 |
|---|---|---|---|---|---|
| empty main | Skylake-SP | 107,752 / 49 | 726,461 / 57 | 324,068 / 56 | -402,393 (-55.4%) / -1 |
| empty main | Zen 1 | 101,533 / 48 | 726,223 / 57 | 323,829 / 56 | -402,394 (-55.4%) / -1 |
| 3 libraries | Skylake-SP | 143,094 / 65 | 966,077 / 72 | 486,648 / 71 | -479,429 (-49.6%) / -1 |
| 3 libraries | Zen 1 | 136,734 / 64 | 965,824 / 72 | 486,394 / 71 | -479,430 (-49.6%) / -1 |
| clock_gettime | Skylake-SP | 109,862 / 50 | 748,574 / 60 | 338,401 / 59 | -410,173 (-54.8%) / -1 |
| clock_gettime | Zen 1 | 103,672 / 50 | 748,365 / 61 | 338,191 / 60 | -410,174 (-54.8%) / -1 |

Kernel instructions / syscalls (kernel counts vary by up to ~50k between runs; syscall counts are exact):

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

libc.so.6 relocations (readelf -r): 21.7 1,606 RELATIVE + 288 others (210 JUMP_SLOT, 75 GLOB_DAT, 3 R_X86_64_64);
head 1,609 RELATIVE + 55 (4 JUMP_SLOT, 48 GLOB_DAT, 3 R_X86_64_64): the malloc family (malloc, free, calloc, realloc,
reallocarray, posix_memalign, aligned_alloc), data (stdin/stdout/stderr, environ, optarg..., __tzname...) and
_init/_fini/array bounds.

## Staged (keke `~/release-staging/snapshot-20261122.1/`, built 2026-11-22T09:52:43Z; MANIFEST + SHA256SUMS)

libc6, libc6-dev, libc6-dbg, libc-bin, libc-dev-bin (amd64), libc-l10n, locales (all) `2.39+niji20261122.1-1`;
niji6, niji6-dev `0.0.snapshot20261122.1+9548c734eea6` (asset names use '.' for '~'). libc6 sha256
9035ef9a1483b1551b222c4eb491d3957d69deeeb993c328f1b780ad3c936aaa.

## Malloc replacement (mint.sh)

mint/alloc.c replaces malloc, free, calloc, realloc, posix_memalign, aligned_alloc, memalign, valloc, pvalloc and
malloc_usable_size with a counting bump allocator; mint/main.c runs 24 phases (caller memory: strdup, strndup,
asprintf, reallocarray, getline, getdelim, open_memstream, realpath, getcwd, scandir; libc state: glob, wordexp,
regcomp, tsearch, qsort of 200,000 ints, getpwnam, getaddrinfo, opendir, setlocale, iconv_open, pthread_create, dlopen;
streams: fopen/fclose, fmemopen) in 5 variants (exe, so, pre = LD_PRELOAD, exe_r / so_r = stock objects relinked on
libniji), trixie root. judge.py: caller rows must reach the replacement and return its memory as on stock, no foreign
frees; internal rows reported (DIFFERS) and compared row by row with the previous snapshot (CHANGED).

## Findings

- **M (INFORMATIONAL, pre-existing, premise challenge):** the brief's "it must still see every malloc call the
  reference shows, except fopen/fclose's stream" does not hold on 21.7 either: under libniji the replacement sees no
  call in qsort (200,000 ints), opendir, setlocale, iconv_open, pthread_create and dlopen (stock: 1, 1, 194, 17, 1, 13),
  nor in fopen/fclose and fmemopen (STDIO-CALLER-ALLOC); caller-visible memory is the replacement's in every variant.
  LIBC-SELF-BIND-2 changed none of the 40 internal rows (CHANGED = 0). Whether libniji's internal allocations should go
  through an interposed malloc is a ruling (recommend: decide per area; a story to route them where glibc's do).
- **T1 (INFORMATIONAL, tooling):** the head's harness writes oracle output under `target/test-oracles/resolv_diff-*`
  (was `resolv-*`) and dns_products has 2 tests (21.7: 3); gaimass.sh's evidence grep found nothing, so the numbers come
  from the copies it saved.
- **S1 (INFORMATIONAL):** empty main 324,068 / 323,829 against LIBC-SELF-BIND-2's merged-branch 323,388 / 323,179
  (+680 / +650; three libraries +944 / +891): the train merged TZ-SECURE-PATH and ROOT-HARNESS-CONTAIN-3 beside it; not
  examined (STARTUP-REPROFILE's to rank).
- **K1 (INFORMATIONAL, host tool):** keke `~/bin/slots` printed `line 333: cg: unbound variable` once in run-keke.out;
  every job still ran and recorded its row.

## Next step

None for this story: the coordinator announces snapshot-2026-11-22.1 to Yohane with the start-up tables above.
