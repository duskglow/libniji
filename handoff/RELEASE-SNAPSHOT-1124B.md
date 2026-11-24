COMPLETE: snapshot-ready at 80d1f809b (serial 1; niji6 `0.0~snapshot20261123.1+80d1f809b2d1` and system `2.39+niji20261123.1-1` sort above 22.3's): B1 fixed (dlc 300/300 named, `_dl_find_object` bad 0 of 300, both link modes and rounds); #25 niji-g++ rows unwind and equal g++ (3 of 3); #24, #11 and `ld.so --library-path` equal stock; #14 bash's command compiles; no check worse than 22.3; start-up -40% user instructions, +1 fault per start

# RELEASE-SNAPSHOT-1124B: snapshot validation of 80d1f809b (B1 fix, #25 unwinder, #24, #11, #14 sources)

**State.** Head under validation `80d1f809b2d162fef2530e06571a203cb9452772` (origin/next at the start, 2026-11-23
23:51Z, "Regenerate train-owned views"; commit time 2026-11-23T22:40:35Z, epoch 1795473635). Contains
416991679 (ld-tables-layout), d671bf726 (niji-gxx-unwinder-2), 1144a6e77 (dl-object-paths), 77064dd2e
(c-locale-identification). No snapshot tag of 2026-11-23 or -24, so **serial 1**: system `2.39+niji20261123.1-1`,
coexist `0.0~snapshot20261123.1+80d1f809b2d1`. First-parent log 2a76723f2..80d1f809b: 88 commits (local scratch
`fplog.txt`). Branch `task/release-snapshot-1124b` = head + handoff/ only. Role keke `~/w/n1` (Zen 1); root checks
and Skylake-SP counts on mei; products and .debs relayed through the local machine, sha256 checked on arrival.

**Job dirs.** keke and mei `~/work/RELEASE-SNAPSHOT-1124B/` = 1122D's scripts through `copy.sh` + `subst.sed` (head,
epoch, serial 1, role n1, labels; 22.3 stays the comparison side: 1122C's debs/products, `-ref223`, p220; this head is
p222). Local scratch `~/w/.scratch/RELEASE-SNAPSHOT-1124B/` holds the edited copies (mei/*). Changed beyond the
substitution: y14l/post.sh (expects en_US source + UTF-8 charmap present; a, p1, p2 compile), y25c.sh + y25c/in.sh
(niji-g++ rows must unwind and equal the g++ control: D-22.24), new y2411.sh + y2411/ (#24 object names, #11
LC_IDENTIFICATION of C/POSIX/C.UTF-8 with contact fields as lengths only, `ld.so --library-path`), run-mei.sh (adds
dlcd, y25c, y2411 to the debs list), run-ref.sh (22.3 sides, done), cmp6.sh (cmp5 mask widened to 23.x / 1124 / p222).

**Done (leg 2, 00:46-01:30Z).** keke's .debs (debs-1, `DEBS bad=0`) relayed keke -> local -> mei, `sha256sum -c` OK
for all 9 at each hop; every keke and mei job below ended; nothing of this run is left running on either host.

## Exact next step

None for this story: the coordinator decides on the tag and publication (nothing to GitHub, no tag from this run).
Staged assets: keke `~/release-staging/snapshot-20261123.1/` (7 system + 2 coexist .debs with GitHub-safe names
(`~` -> `.`), SHA256SUMS (`sha256sum -c`: 9 OK), MANIFEST).

## Results

| check | job(s) | result |
|---|---|---|
| build + smoke, Zen 1 / Skylake-SP | keke smoke-keke-1, mei smoke-mei-1 | `SMOKE: RESULT bad=0` both; mei vs 22.3 differs only in layout-verdict cache lines (as 1122D) |
| gate | keke gate-1 | `GATE bad=0` (clippy all-features, xtask clippy, static, records check, direct tier) |
| #21 heap products test | keke heapnode-1 | rc 0 |
| both flavours, check, check-symbols | keke debs-1 | `DEBS bad=0`; check PASS (7 packages; coexist 2); check-symbols PASS (4490 symbol lines, 14 libraries; 22.3/22.4: 4440, 13) |
| versions | keke (dpkg --compare-versions) | `0.0~snapshot20261123.1+80d1f809b2d1` gt 22.3's `0.0~snapshot20261122.3+2a76723f2ed4`; `2.39+niji20261123.1-1` gt `2.39+niji20261122.3-1` |
| plants (release checks) | keke plants-1 | 6 plants: 6 CAUGHT, 0 SURVIVED, no WARN (results in handoff/RELEASE-SNAPSHOT-1124B-plants.txt) |
| package diff vs 22.3 | keke pkgdiff-1 | locales: +287 sources, SUPPORTED, charmaps; niji6 +libthread_db.so.1; niji6-dev +libthread_db.so, +niji-unwinder.o; no package dropped |
| stage, install steps of docs/install-packages.md | keke stage-1, install-1 | staged 9 + SHA256SUMS + MANIFEST; `inst: RESULT fails=0`, mmdebstrap rc 0, system debs identical 7 / different 0 |
| getaddrinfo_a (Zen 1) | keke gaimass-1 | `GAIMASS bad=0` |
| **B1** dlc (both rounds, both link modes) | mei dlc-1 | `RESULT fail=0`: prog and prog-niji print stock's 10 lines (dladdr 300, phdr 301, lmap 301 in both rounds; closed 0/0); 1122D: 299/300 |
| **B1** diagnostic | mei dlcd-1 | prog and prog-niji: `bad total 0 of 300; bad-find total 0 of 300` (each library's own map and eh_frame); rc 1 only from the by-design "stock run is not the recorded reference" line |
| **#25** niji-g++ (unwinder) | mei y25c-1 | 3 of 3 rows (pthread_exit dtor; catch abi::__forced_unwind + rethrow; cancelled in pause()) unwind and equal the g++ control (stdout bytes, rc 0); 22.3: exit 0 without destructors |
| **#25** g++ / -static-libgcc / plain C | mei y25n-1, y25-1, y25s-1 | `RESULT fail=0`: fug rows equal stock; fus rows rc 134 with the FATAL message naming cause and fix (stock rc 134, 8 B stderr); plain-C pthread_exit (pc) equals stock |
| **#24, #11, --library-path** | mei y2411-1 | `RESULT fail=0`: p24 every object name equals stock (24 lines); p11 reproducer prints `[ISO]`; C.UTF-8 items and C/POSIX non-contact items equal stock (39 lines), contact fields empty (D-7.10); 5 `ld.so --library-path` rows equal stock. 22.3: 6 FAIL |
| **#14** | mei y14l-1 | `RESULT fail=0`: bash's command compiles (rc 0, stock 0, same file list); p1/p2 compile; status of help/version/missing-source/missing-charmap/no-operand/bad-option as stock; finding L2 below. 22.3: no sources, fails |
| **#21** | keke heapnode-1; mei y21-1 / y21-ref223-1 | rc 0; masked verdicts equal 22.3's |
| roots (empty root F1 `ldconfig -p`, swapped trixie root's 47 commands, dynamic C++, apt, dbg) | mei roots-1 | rc 0 (22.3: rc 1). Differs from 22.3 only by: dbg 9/9 (gdb stops at setlocale with source line; 22.3 8/9) and 18 installed objects with libc6-dbg .debug (22.3: 17, + libthread_db) |
| lintian E/W, trixie and excalibur | mei lint-1 | errors 10 (= 22.3), warnings 43 (22.3: 50). Gone: copyright-without-copyright-notice (9 packages), locales empty-binary-package. New: `W: libc-bin: extended-description-line-too-long line 2` (finding L3) |
| other root checks | mei yohane, many, dyn22, y10, y19, y26, y26t, y23, c19, y24, y28, y6, yres, y23p, y27, y13, y14, y6c, y14r, eetwet, mint, y13s, drp, y6r, y13v (-1 each; drp/y13s/y6r/y13v also -ref223-1) | masked verdicts equal 22.3's (cmp6.sh), except y13 and y13v: valgrind `lost=0` -> `lost=none` (no leak summary at all; same pass verdict). rc 1 as on 22.3 by design: yohane, y14, y14r (zdbad), drp |
| sig32, y20, dpo600 | mei -1 | masked verdict lines equal to 22.3's (cmp6.sh) |
| y2411 on 22.3 | mei y2411-ref223-2 (-1 superseded: C.UTF-8 contact verdict and dli_sname fixed in post.sh) | 22.3: #24 names equal stock; fails #11 (issue prints `[]`) and `ld.so --library-path` (rc 127): 6 FAIL as expected |
| 22.3 sides | mei y25-ref223-1 (rc 0), y25n-ref223-1 (rc 1 by design, as 1122D), y25c-ref223-1 (rc 1: niji-g++ on 22.3 exits 0 without destructors), y14l-ref223-1 (rc 1: no sources/charmaps in 22.3), dlc-ref223-1 (rc 0), dlcd-ref223-1 (0 misses; rc 1 by design) | |

**Loader program headers (ld-linux-x86-64.so.2, vaddr / memsz):**

| build | R (headers) | R E (text) | R (read-only) | RW | hole text end -> read-only |
|---|---|---|---|---|---|
| 22.3 (2a76723f2) | 0x0 / 0x600 | 0x1000 / 0x5b2e7 | 0x61000 / 0x8534 | 0x6ad60 / 0x952a0 | 0x5d000-0x61000 = 16 KiB |
| 1122D (8e1837914) | 0x0 / 0x600 | 0x1000 / 0x60ea7 | 0x71000 / 0x9334 | 0x7bd60 / 0x842a0 | 0x62000-0x71000 = 60 KiB |
| head (80d1f809b) | 0x0 / 0x600 | 0x1000 / 0x669a7 | 0x71000 / 0x9b64 | 0x7bd50 / 0x842b0 | 0x68000-0x71000 = 36 KiB |

The hole remains (36 KiB); LD-TABLES-LAYOUT's fix is the segment-run match in the lookups (D-27.89), so dlc/dlcd on
this head are the proof.

**Start-up** (user instructions / page faults per start, medians; glibc 2.39 / 22.3 (p220) / head (p222)):

| row | Zen 1: glibc | 22.3 | head | Skylake-SP: glibc | 22.3 | head |
|---|---|---|---|---|---|---|
| empty main | 101,533 / 48 | 260,618 / 51 | 156,382 / 52 | 107,752 / 49 | 260,857 / 51 | 156,623 / 52 |
| 3 libraries | 136,734 / 64 | 370,892 / 66 | 231,514 / 67 | 143,094 / 65 | 371,146 / 66 | 231,770 / 67 |
| clock_gettime | 103,672 / 50 | 274,120 / 55 | 167,387 / 56 | 109,862 / 50 | 274,330 / 54 | 167,599 / 55 |
| libresolv pair (res0) | 162,716 / 54 | 373,764 / 56 | 227,995 / 57 | 169,020 / 55 | 374,316 / 56 | 228,551 / 57 |

User instructions -40% (empty main) to -39% (res0) against 22.3 on both classes; **+1 page fault on every row, both
classes** (finding, non-blocking; not attributed yet). Movers by their own reports: LD-OBJECT-ONCE-3 (empty main
268,959 -> 196,630 Skylake-SP; per extra object 36,417 -> 31,356), LD-RELR-LOOP-4 (189,818 -> 159,047),
LD-SYMBOL-VALIDATE-ONCE (per import 1,525 -> 996), LIBC-INIT-ARRAY-REFS (-7,000, already in 1122D), TLS-DYNAMIC-BLOCKS
(-750). Kernel-side (kstart/rstart KCOUNT) medians are noisy (min-max ~100k) and not compared.

## Findings (none blocking)

* **L2 (new behaviour difference, #14 area):** a locale compiled from libniji's shipped en_US source formats `%c`
  (and `%Ec`) as `Sun, Nov 22, 2026, 02:05:09 PM`; the reference glibc 2.39's installed en_US.UTF-8 (black-box:
  `LC_ALL=en_US.UTF-8 TZ=UTC date +%c` on the host) and the trixie stock both give `Mon 23 Nov 2026 02:05:09 PM UTC`.
  These are the only y14l read-back lines that differ (8 of 37 lines, stock and libniji sides counted); collation,
  ctype, mbrtowc and the other strftime rows are equal, and the test's own source reads back equal to stock, so
  the difference is in the shipped source data (d_t_fmt), not localedef. 22.3 shipped no sources. Recommend a
  story: compare every shipped locale's LC_TIME/LC_NUMERIC/LC_MONETARY items with the reference's compiled locales
  (black-box `locale -k`).
* **L3 (lintian, new):** `W: libc-bin: extended-description-line-too-long line 2` (trixie and excalibur);
  line 2 of tools/deb/description/libc-bin is "This package holds the programs $programs, the", and the program
  list it expands to (longer since 22.3) is not wrapped.
* **Page faults:** +1 per start on every start-up row, both classes (above), not attributed.

* **D1 (stale, still):** docs/install-packages.md:159 and tools/deb/description/locales ("locale support
  (placeholder)", "Locale source data is not part of this build yet") are false now: locales ships 287 sources,
  SUPPORTED and charmaps (LOCALE-SOURCES-GEN-2).
* **D3 (missing):** docs/RELEASE-NOTES.md "Unreleased" has only libthread_db; no sentence yet for the B1 fix, #25
  (niji-g++ unwinds), #24, #11, the locale sources / localedef stories, the start-up drop. (Release notes may be the
  train's: RELEASE-NOTES-FRAGMENTS was running.)
* **D4 (dangling path):** docs/compat-flags.md:33 (COMPAT-12) names `crates/boundary/tests/s29_rpc_reply_source.rs`,
  which does not exist.
* F1 of 1122D is answered: docs/install-packages.md:84-90 describe niji-unwinder.o.
* p24: `dladdr`'s `dli_sname` for an address several symbols share differs from stock (printf: stock `_IO_printf`,
  libniji `printf`; cos: stock no name, libniji `cosf64`); same on 22.3; not #24's subject; shown.

## Process note

Leg 2 wrote two small helper scripts to mei's work dir (meiwait.sh: the label list for one jobwait; fulldiff.sh:
cmp6.sh's mask without the 8-line cut). eetwet-1 starts only after y14r-1, so a jobwait naming it early returns rc 4
(never queued); waited separately. Leg 1: one `sed -i` was used on a local scratch copy (y2411/post.sh: one line, the contact-length INFO grep's pattern, so it
matches the new C/POSIX-anchored `$C`) against the rules (Edit tool only); the edit is correct; reported, not repeated.
