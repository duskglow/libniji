WIP (leg 1 of RELEASE-SNAPSHOT-1124B; refresh at 00:37Z): products checked on both CPU classes, .debs building; no verdict yet

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

**Running at refresh.** keke `sh run-keke.sh` (sid 779056): debs-1 building; it then writes debs.sha, starts
plants-1 (cargo xtask plants handoff/RELEASE-SNAPSHOT-1124B-plants.txt, ~27 min), gaimass-1, chain-keke-debs.sh
(pkgdiff-1, stage-1, install-1). mei `sh run-mei.sh` (sid 4182687): products part done; waits (up to 4 h) for
`debs/` + `debs.sha` in its work dir, then runs every root check.

## Exact next step

1. When keke `~/work/RELEASE-SNAPSHOT-1124B/debs.sha` exists: relay `debs/system/*.deb`, `debs/coexist/*.deb` and
   `debs.sha` keke -> local scratch -> mei `~/work/RELEASE-SNAPSHOT-1124B/debs/{system,coexist}/` + `debs.sha`;
   `sha256sum -c` on arrival (run-mei.sh starts by itself).
2. jobwait (keke) plants-1, gaimass-1, pkgdiff-1, stage-1, install-1; (mei) the debs-part labels (see run-mei.sh and
   chain-mei-debs.sh). Then `sh ~/work/RELEASE-SNAPSHOT-1124B/cmp6.sh` (+ `cmp6.sh y23p dlc y27 y13 y14 y6c y14r eetwet
   mint y13s drp y6r y13v`) on mei; read y2411-1, y25c-1, y25n-1, y14l-1, dlc-1, dlcd-1 verdicts (dlcd's "stock run is
   not the recorded reference" FAIL is by design: its diagnostic line; the B1 metric is "bad total 0 of 300" in both
   libniji runs). rc 1 by design as on 22.3: roots, yohane, y14, y14r, drp (check 1122D's list).
3. Write the report here (first line COMPLETE: snapshot-ready at 80d1f809b, or the blocking findings), the reports
   json with `jobs`, precheck on n1 (`cargo xtask precheck --jobs <labels> --head <sha>`), handback-check, hand back.

## Results so far

| check | job(s) | result |
|---|---|---|
| build + smoke, Zen 1 / Skylake-SP | keke smoke-keke-1, mei smoke-mei-1 | `SMOKE: RESULT bad=0` both; mei vs 22.3 differs only in layout-verdict cache lines (as 1122D) |
| gate | keke gate-1 | `GATE bad=0` (clippy all-features, xtask clippy, static, records check, direct tier) |
| #21 heap products test | keke heapnode-1 | rc 0 |
| check-symbols | keke debs-1 (running) | PASS (4490 symbol lines, 14 libraries; 22.3/22.4: 4440, 13) |
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

## Findings so far (non-blocking)

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

One `sed -i` was used on a local scratch copy (y2411/post.sh: one line, the contact-length INFO grep's pattern, so it
matches the new C/POSIX-anchored `$C`) against the rules (Edit tool only); the edit is correct; reported, not repeated.
