COMPLETE: snapshot-ready at 302dd7c3a (snapshot-2026-11-22.2, serial 2 of the commit date 2026-11-22)

# RELEASE-SNAPSHOT-1122B: snapshot validation of 302dd7c3a (ICONV-CJK-STARTUP-4, VALGRIND-LOADER-VIEW-3)

**State.** Head under validation `302dd7c3a311aee56a23853dbf7f887916eaf3de` (origin/next at the start, 11:53 UTC;
"Regenerate train-owned views"; commit time 2026-11-22T11:14:41Z, epoch 1795346081). Commit date 2026-11-22, 22.1 was
serial 1 of that date = **serial 2**: `NIJI_DEB_SERIAL=2`, `2.39+niji20261122.2-1` (coexist
`0.0~snapshot20261122.2+302dd7c3a311`), both `dpkg --compare-versions gt` 22.1's. Branch `task/release-snapshot-1122b`
= that head + handoff/ only. Role **mei `~/w/s2` (Skylake-SP)**; keke = Zen 1 (jobs on mei's products and .debs,
sha256 checked on arrival). 22.1 = 9548c734e (RELEASE-SNAPSHOT-1122A). Nothing to GitHub, no tag.
**HEAP-SPILL-COLD-3 is not in this head**: it landed after the head was fixed and the builds had run (coordinator
note 12:24); it goes into the next snapshot.

**Landed since 22.1** (`git log --first-parent 9548c734e..302dd7c3a`): ci-products-once (442d77c97), view regeneration,
heap-test-node-pin-2 (5f07429d3), startup-reprofile (ae6053d1e), strncmp-avx2-writer (ed299cc42),
timed-wait-past-deadline (d77950f4b), view regeneration, release-snapshot-1122a (a72580980), sl-headers-s19-4
(7b475b489), **iconv-cjk-startup-4 (f563dbbed)**, **valgrind-loader-view-3 (8d6c7a1c5)**, view regeneration (302dd7c3a).
**LD-PHDR-ONCE: not in this head. UTIL-12: not in this head** (no commit of either since 9548c734e).

**Procedure.** 1122A's scripts (mei and keke `~/work/RELEASE-SNAPSHOT-1122B/`, made by copy.sh + subst.sed: head,
epoch, serial, role and labels only), with the hosts swapped back (role on mei: run-mei.sh builds and runs the .deb
checks; run-keke.sh runs keke's smoke round, Zen 1 counts, getaddrinfo_a, pkgdiff, staging and install check). Plants
file handoff/RELEASE-SNAPSHOT-1122B-plants.txt (1122A's six, each pattern present once in the head). New this run:
`startup215` (startup.sh with snapshot-2026-11-21.5's products only: 21.5's faults measured beside the head) and
`y13v` (VALGRIND-LOADER-VIEW-3's swapped-root acceptance, its probes unchanged, plus a shown-only still-reachable
listing), each y13v also on 22.1's .debs. Comparison: per check, the masked verdict lines of 1122A's log against this
run's (scratch cmp.py); `-ref221` runs repeat probes on 22.1's .debs.

## Results (all on 302dd7c3a's products and .debs; labels prefixed `release-snapshot-1122b-`)

| check | jobs | result | vs 22.1 |
|---|---|---|---|
| smoke | mei smoke-mei-1, keke smoke-keke-1 | `SMOKE: RESULT bad=0` both | same (the layout-cache line moves with the role) |
| gate (direct tier, clippy all-features, static, records) | mei gate-1 | `GATE bad=0`, peak 2.3G | same |
| .debs | mei debs-1 | `DEBS bad=0`; check PASS 7 packages; check-symbols PASS 4440 lines, 13 libraries; DBG 21 of 21 | same |
| plants | mei plants-1 | 6 of 6 CAUGHT by their messages; no WARN | same |
| package diff | keke pkgdiff-1 | file lists equal (24, 7, 7, 75, 7, 48, ... files each); libc-bin .deb 602,448 -> 335,612 bytes (Installed-Size 2663 -> 1763) | finding P1 |
| stage, install | keke stage-1, install-1 | staged below; mmdebstrap rc 0, system debs different: 0 | same |
| trixie root, libniji swapped in | mei roots-1 | rc 1 on the known lines (gdb `??` frame in /usr/bin/true) | 0 differ but a loader address in that line |
| lint, yohane, many, dyn22 | mei lint-1, yohane-1, many-1, dyn22-1 | yohane rc 1 on the known lines | 0 differ each (lint: package names) |
| Yohane loop checks | mei y10, y19, y26, y26t, y23, c19, y25, y24, y28, y6, y25s, y21, yres, y23p, dlc, y27, y13, y14, y6c, y14r, eetwet (-1 each) | y14 / y14r rc 1 on the known lines | 0 differ each but y26 (finding T2) |
| valgrind string redirect (#13) | mei y13s-1, y13s-ref221-1 | `YOHANE-13S fail=0` both | same |
| valgrind loader view (#13, new) | mei y13v-1, y13v-ref221-1 | head 8 of 8 PASS; 22.1 5 of 8 | below |
| DL-RUNPATH | mei drp-1, drp-ref221-1 | fail=2 both (setgid `$ORIGIN` rows, the known secure-mode difference) | same |
| CJK iconv (y6r) | mei y6r-1, y6r-ref221-1 | pass=13 fail=0 both | same |
| signal 32 / #20 / 600 libraries | mei sig32-1, y20-1, dpo600-1 | `fail=0` each | same |
| getaddrinfo_a | keke gaimass-1 | `GAIMASS bad=0` (head and 22.1's products) | same |
| malloc replacement | mei mint-1 (previous run: 1122A's mint-1) | `MINT fail=0 internal_rows_differing_from_stock=40 changed_from_previous=0` | 0 rows changed; M |

## Start-up (execcount, ASLR off, median of 101; startup/kstart/rstart/startup215-{mei,keke}-1, all rc 0)

User instructions / page faults. 21.5's instructions are RELEASE-SNAPSHOT-1121D's recorded numbers; its faults were
measured again in this run (startup215, same host and kernel: identical to the record on every row).

| program | class | glibc 2.39 | 21.5 | 22.1 | 302dd7c3a | change from 22.1 | faults vs 21.5 |
|---|---|---|---|---|---|---|---|
| empty main | Skylake-SP | 107,752 / 49 | 741,532 / 51 | 324,068 / 56 | 320,731 / 51 | -3,337 (-1.03%) / **-5** | **0** |
| empty main | Zen 1 | 101,533 / 48 | 741,300 / 51 | 323,829 / 56 | 320,492 / 51 | -3,337 (-1.03%) / **-5** | **0** |
| 3 libraries | Skylake-SP | 143,094 / 65 | 984,518 / 65 | 486,648 / 71 | 483,296 / 66 | -3,352 (-0.69%) / **-5** | **+1** |
| 3 libraries | Zen 1 | 136,734 / 64 | 984,271 / 65 | 486,394 / 71 | 483,042 / 66 | -3,352 (-0.69%) / **-5** | **+1** |
| clock_gettime | Skylake-SP | 109,862 / 50 | 763,624 / 54 | 338,401 / 59 | 335,064 / 54 | -3,337 (-0.99%) / **-5** | **0** |
| clock_gettime | Zen 1 | 103,672 / 50 | 763,420 / 55 | 338,191 / 60 | 334,854 / 55 | -3,337 (-0.99%) / **-5** | **0** |

Kernel instructions / syscalls (kstart, the three sets in one run; kernel counts vary by up to ~50k between runs,
syscall counts are exact):

| program | class | glibc 2.39 | 22.1 | 302dd7c3a | change from 22.1 |
|---|---|---|---|---|---|
| empty main | Skylake-SP | 688,518 / 46 | 740,008 / 29 | 717,629 / 29 | -22,379 / 0 |
| empty main | Zen 1 | 701,665 / 42 | 757,884 / 28 | 736,698 / 28 | -21,186 / 0 |
| 3 libraries | Skylake-SP | 987,790 / 72 | 1,148,719 / 59 | 1,116,977 / 59 | -31,742 / 0 |
| 3 libraries | Zen 1 | 997,735 / 68 | 1,166,994 / 58 | 1,143,904 / 58 | -23,090 / 0 |
| clock_gettime | Skylake-SP | 691,829 / 46 | 768,421 / 29 | 746,702 / 29 | -21,719 / 0 |
| clock_gettime | Zen 1 | 711,827 / 42 | 793,172 / 28 | 769,831 / 28 | -23,341 / 0 |

libresolv pair (rstart; res0 needs libresolv.so.2, pie0 empty):

| class | row | glibc 2.39 | 22.1 | 302dd7c3a | change from 22.1 |
|---|---|---|---|---|---|
| Skylake-SP | res0 user / faults | 169,020 / 55 | 452,124 / 61 | 448,781 / 56 | -3,343 / -5 |
| Skylake-SP | res0 kernel / syscalls | 821,173 / 58 | 927,818 / 42 | 904,323 / 42 | -23,495 / 0 |
| Skylake-SP | res0 - pie0 user | 61,375 | 128,088 | 128,082 | -6 |
| Zen 1 | res0 user / faults | 162,716 / 54 | 451,572 / 61 | 448,229 / 56 | -3,343 / -5 |
| Zen 1 | res0 kernel / syscalls | 842,716 / 54 | 946,796 / 40 | 924,453 / 40 | -22,343 / 0 |
| Zen 1 | res0 - pie0 user | 61,235 | 127,825 | 127,819 | -6 |

libc.so.6 relocations (readelf -r): 22.1 1,609 RELATIVE + 55 others; head **1,441 RELATIVE** (-168: the CJK tables
hold offsets) + 55 (4 JUMP_SLOT, 48 GLOB_DAT, 3 R_X86_64_64).

## valgrind (y13v: trixie's valgrind, the whole libc6-dbg, swapped trixie root)

* **Leak record** (`asprintf` block lost in leak.c), head: malloc (vg_replace_malloc.c), malloc (caller_alloc.rs:58),
  niji_boundary::printf::vasprintf (printf.rs:957), `__vasprintf_chk` (s12_stdio.rs:1209), asprintf (in libc.so.6,
  no source line), **main (leak.c:9)**; no unnamed frame. Stock: malloc, the library's internal vasprintf, asprintf,
  main (leak.c:9). 22.1: the record stops in 3 unnamed frames, main not reached (y13v-ref221: FAIL).
* **Still reachable at exit** (memcheck --show-leak-kinds=reachable, shown, not judged): head 4,104 bytes in 1
  block, allocated by puts (leak.c:11) through stdout's buffer setup (stdio_compat.rs ensure_buffer); the same on
  bad and cxx; stock 0 bytes in 0 blocks on every program. 22.1: the same 4,104 bytes. Finding V1.
* **helgrind**: thr and thr_old 0 errors, race 2 errors at race.c (2 races), exactly as stock; 22.1: thr and race end
  in a valgrind assertion (rc 1), thr_old passes.
* memcheck true / bad (invalid read at bad.c:11) / cxx (32 bytes lost at cxx.cc): ERROR SUMMARY equal to stock.

## Staged (keke `~/release-staging/snapshot-20261122.2/`; MANIFEST + SHA256SUMS)

libc6, libc6-dev, libc6-dbg, libc-bin, libc-dev-bin (amd64), libc-l10n, locales (all) `2.39+niji20261122.2-1`;
niji6, niji6-dev `0.0.snapshot20261122.2+302dd7c3a311` (asset names use '.' for '~'). libc6 sha256
726596e576af35f743fbe2ae84a37da4abfdc601b9e99ecb5483550678fb229a.

## Findings

- **F1 (headline):** the 22.1 regression of 5 page faults per dynamic start is gone: empty main and clock_gettime are
  at 21.5's fault level on both classes; 3 libraries is still +1 fault against 21.5 (66 vs 65) on both classes.
  Every start-up row is also -3,337 to -3,352 user instructions and ~21k to ~32k kernel instructions below 22.1.
- **V1 (INFORMATIONAL, pre-existing):** under valgrind libniji leaves stdout's 4,104-byte buffer reachable at exit;
  stock shows 0 reachable bytes. Not an error (memcheck reports it only with --show-leak-kinds=reachable or in the
  summary line). Recommend: a small story to release stdio buffers at exit when running under valgrind.
- **V2 (INFORMATIONAL):** the asprintf frame of the leak record has no source line (named, in libc.so.6).
- **P1 (INFORMATIONAL):** libc-bin's .deb is 266,836 bytes smaller (installed 900 KiB less) with the same 24 files;
  every check that runs its programs (roots, lint, yohane, y6r, y14) is unchanged. Not attributed in this run
  (ICONV-CJK-STARTUP-4 is the only landed change touching the tables the static programs carry).
- **M (INFORMATIONAL, pre-existing):** mint unchanged from 22.1 (40 internal rows differ from stock, none changed).
  Owner ruling #7583 (internal allocations through a replaced malloc) is being implemented (STDIO-CALLER-ALLOC,
  INTERNAL-ALLOC-ROUTING); expect these rows to change in a later snapshot.
- **T2 (INFORMATIONAL, host):** y26's TSan row with ASLR on read `Segmentation fault` (rc 139) in 1122A and
  `FATAL: ThreadSanitizer: unexpected memory mapping` (rc 66) now, on the stock root and libniji alike (INFO-only
  row; host mmap randomness, recorded in RELEASE-SNAPSHOT-1120).
- **S2 (INFORMATIONAL, tooling):** startup215's instruction counts are not comparable with startup's (its longer RAM
  directory name adds ~243 instructions to the glibc rows); only its faults are used.
- **K2 (INFORMATIONAL, run):** starting both runners in one ssh call left the local ssh attached; killing it let a
  second keke runner start, killed by PID within seconds (before any job of it was queued). Every job ran once.

## Next step

None for this story: the coordinator announces snapshot-2026-11-22.2 to Yohane with the start-up tables above.
The next snapshot picks up HEAP-SPILL-COLD-3 (y21 with the ulimit -v formula and the node-change test, per the
coordinator's note).
