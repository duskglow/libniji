COMPLETE: NOT snapshot-ready at 44304e709 — headers work (every shipped header compiles, links and runs in C and C++ in both flavours; yohane-1's cmocka program PASSES; 5 of 7 Debian packages build) but 3 header gaps break real builds (tree, xxhash, static busybox), so step 4e could not run; #21's in-tree products test fails its 35,000 KiB bound (packaged formula still 5 of 5); start-up +1.9k..+12.1k user instructions per start over 24.1

# RELEASE-SNAPSHOT-1124D: snapshot validation of 44304e709 (SL-HEADERS-S20: libniji's own C headers; LD-START-MAPS-3)

**State.** Head under validation `44304e709fe9e577d49b44274f25aaa84a764e64` (origin/next at the start, 2026-11-24
22:32Z, "Regenerate train-owned views"; commit time 2026-11-24T22:05:35Z, epoch 1795557935). First-parent log since
24.1 (88c0518f6): 28 commits, among them the merges of sl-headers-s20-4, ld-start-maps-3, ldso-program-mode-space,
shstk-hw-enable, headers-gen-shared, headers-small-fixes-2, headers-macro-form-audit, headers-sys-param,
headers-sigstksz-match, util-tzselect-pldd, util-locale-gen, locale-translit-table, sl-headers-s07-tail-3. 24.1 exists
for 2026-11-24, so **serial 2**: system `2.39+niji20261124.2-1`, coexist `0.0~snapshot20261124.2+44304e709fe9`
(both sort above 24.1's). Branch `task/release-snapshot-1124d` = head + handoff/ only. Role keke `~/w/n1` (Zen 1:
products, .debs, gate, plants, staging); root checks and Skylake-SP counts on mei; products and .debs relayed keke ->
local -> mei, `sha256sum -c` OK at each hop (9 .debs).

**Job dirs.** keke and mei `~/work/RELEASE-SNAPSHOT-1124D/` = 1124C's scripts through `copy.sh` + `subst.sed`
(head, epoch, serial 2, labels; comparison side 23.1 -> 24.1: 1124C's debs, products and logs, `-ref241`, p223; this
head is p224). New (mei): `yh.sh` + `yh/` (4a, 4d), `yhc.sh` + `yhc/` (4b), `ypk.sh` + `ypk/` (4c, 4e), `hp.sh`
(identifier probe, host glibc 2.39 vs the coexist sysroot), `cmp8.sh` (cmp7.sh with 88c0518f6 and p224 masked).
Nothing of this run is left running on either host.

## Headers (the point of this snapshot)

| step | job | result |
|---|---|---|
| 4a system flavour | mei yh-1 (yh-ref241-1: 24.1, no headers) | swapped trixie root, Debian's libc6-dev gone: `/usr/include` owned by libniji's libc6-dev, 0 stale Debian headers, 0 files differing from the package. 236 header names shipped: **234 compile, link and run in C and in C++** (`#include <h>` + main), 2 refuse direct inclusion with an `#error` (regexp.h, sys/elf.h; Debian's refuse too); **0 FAIL**. C++ `<iostream>`, `<locale>`, `<cwchar>`, `<thread>` with trixie g++/libstdc++: output and status equal stock's, 4 of 4 |
| 4a yohane-1 | mei yohane-1 | **PASS**: `gcc cm.c -lcmocka compiles and links in the swapped root`, `cm-full runs: 1 cmocka test passed`; `YOHANE-LDCONFIG pass=18 fail=0` (24.1: fail=2) |
| 4b coexist | mei yhc-1 (rc 1: niji-cc only) | `/usr/include` digest unchanged by installing niji6/niji6-dev and by the compiles; niji-gcc / niji-g++ take `<stdio.h>` from `/usr/lib/niji/usr/include` (-M); all 236 names: 234 OK, 2 BOTH, 0 FAIL in C and C++; the four C++ programs through niji-g++ equal g++'s. **niji-cc is not in the packages** (niji6-dev ships niji-gcc and the niji-g++ link only), so its 236 rows fail "not found": a notes finding, not a header one |
| 4c Debian packages | mei ypk-1 | built with `dpkg-buildpackage -b -uc -us -d` (checks run) on stock, then on libniji in the swapped root: **lz4, dash, libyaml (C), tinyxml2, libjsoncpp (C++) build**; **tree and xxhash FAIL on libniji only** (stock builds) |
| 4d coverage | mei yh-1 | Debian libc6-dev 2.41: 478 names; ours 236; **242 missing** (11 outside bits/, 231 bits/): handoff/RELEASE-SNAPSHOT-1124D-headers.txt |
| 4e static busybox | mei ypk-1 | **not run: busybox 1.37.0 (Debian's static config, own passwd code off) does not compile** against libniji's headers (below). Stock's build and applets work in the same root |

**Header gaps found by real builds** (mei hp-1: `-fsyntax-only`, host glibc 2.39 headers vs the coexist sysroot;
"stock PASS, libniji FAIL"):
* `<sys/types.h>`, default mode: `u_long` undeclared (tree: `tree.h:192: unknown type name 'u_long'`, then cascades).
  Not under `-std=c11` (both fail there), so tree reaches it through its own feature macros.
* `<regex.h>` with `_GNU_SOURCE`: `RE_TRANSLATE_TYPE` missing (busybox findutils/grep.c:180).
* `<paths.h>` `_PATH_WTMP`; `<utmp.h>` `WTMP_FILE`, `_PATH_UTMP`; `<utmpx.h>` `WTMPX_FILE`, `UTMPX_FILE` missing
  (busybox libbb/messages.c:53 `#error unknown path to wtmp file`).
* `<sys/stat.h>`: xxhash's `cli/xsum_os_specific.c:509/521` misses `S_IFMT`, `S_IFREG`, `S_IFDIR`; none of 8 probed
  configurations (default, `_POSIX_C_SOURCE` 1/200112L/200809L, `_XOPEN_SOURCE=700`, `-std=c99`, fortify) reproduces
  it (all SAME), so the configuration xxhash uses is **not isolated yet**.

## Everything else rerun (labels `release-snapshot-1124d-<x>`)

| check | job(s) | result |
|---|---|---|
| build + smoke | keke smoke-keke-1, mei smoke-mei-1 | `SMOKE: RESULT bad=0` both |
| gate | keke gate-1 | `GATE bad=0` (clippy all-features, xtask clippy, static, records check, direct tier) |
| **#21 heap products test** | keke heapnode-1 | **6 passed, 1 FAILED**: `find_over_200000_entries_bounds_its_address_space`: needs 36,672..38,464 KiB over 3 entries at every layout phase > #21's 35,000 (24.1: 7 passed) |
| debs, check, check-symbols | keke debs-1 | `DEBS bad=0`; check PASS (7; coexist 2); check-symbols PASS (4490 lines, 14 libraries; = 24.1) |
| plants | keke plants-1 | 6 CAUGHT, 0 SURVIVED, no WARN-RC-ONLY / WARN-NO-MSG (WARN-DEV-SHM as briefed): handoff/RELEASE-SNAPSHOT-1124D-plants.txt |
| package diff vs 24.1 | keke pkgdiff-1 | files: libc6-dev 39 -> 296 and niji6-dev 50 -> 317 (the headers), libc-bin 25 -> 28 (/usr/bin/ld.so, pldd, tzselect), locales 376 -> 387, libc6-dbg 55 -> 59, niji6 30 -> 32; others equal; no package dropped |
| stage, install | keke stage-1, install-1 | staged 9 + SHA256SUMS + MANIFEST; `inst: RESULT fails=0` |
| getaddrinfo_a | keke gaimass-1 | `GAIMASS bad=0` |
| root checks | mei: 42 jobs (roots ... y21, smoke-mei; -ref241 sides) | `cmp8.sh`: 37 of 42 masked verdict sets equal 24.1's. Differ: yohane (FAIL -> PASS, the header fix), roots (pass 21 -> 31, fail 0: new PASS rows for the ld.so link, tzselect, pldd, locale-gen / validlocale / update-locale, setlocale under /etc/locale.conf; /etc/locale.alias now a conffile), y12 (24.1's comparison log is 1124C's superseded y12-1; equal to its y12-2: 17 PASS), y21 (numbers only), lint (below). rc 1 by design: dlcd, y14, y14r, drp (yohane now rc 0) |
| **#21** (issue's probe, packaged find) | mei y21-1 / y21-ref241-1 | formula 5 of 5 (both). Start VmSize 8,656 (24.1 8,640; stock 3,988). find over 3 files **12,544** (24.1 22,144: -9,600); over 200,000 45,744 (24.1 45,728); 200,000 over 3: **33,200** (24.1 23,584; allowance 35,000) |
| lint | mei lint-1 | errors 10 (= 24.1), warnings 43 -> 50: L3 fixed (`extended-description-line-too-long` gone); new `no-manual-page` (libc-bin +3, locales 3), locales `missing-debconf-dependency`, `postrm-does-not-purge-debconf`, niji6-dev `absolute-symlink-in-top-level-folder` (10: the kernel-header links of the sysroot) |

**Start-up** (user instructions / page faults per start, medians; glibc 2.39 / 24.1 (p223) / head (p224)):

| row | Zen 1: glibc | 24.1 | head | Skylake-SP: glibc | 24.1 | head |
|---|---|---|---|---|---|---|
| empty main | 101,533 / 48 | 154,296 / 52 | 156,289 / 52 | 107,738 / 49 | 154,535 / 52 | 156,531 / 52 |
| 3 libraries | 136,734 / 64 | 229,422 / 67 | **241,564** / 67 | 143,080 / 65 | 229,676 / 67 | **241,821** / 67 |
| clock_gettime | 103,672 / 50 | 165,300 / 56 | 167,237 / 56 | 109,848 / 50 | 165,510 / 55 | 167,450 / 55 |

+1,937..+1,996 per start (empty main, clock_gettime) and **+12,142 / +12,145 (+5.3%) with 3 libraries**, both
classes; faults equal. 24.1's numbers reproduce 1124C's exactly.

## Docs (step 6)
* Release-notes fragments cover the switch (docs/release-notes.d/SL-HEADERS-S20.md, -2.md), but SL-HEADERS-S20.md
  says `niji-cc` compiles against the headers: the packages ship no niji-cc.
* **Still saying there are no headers:** tools/deb/description/libc6-dev lines 4-5 ("libniji's C headers are not part
  of this build yet": the system libc6-dev package's own description, `apt show`), README.md:27 ("it has no C headers
  of its own yet"). docs/install-packages.md is current (it documents the headers; its libc6-dev row names `a.out.h`,
  which is not shipped). D1 (locales "placeholder") unchanged.

## Findings for the coordinator
1. Header gaps above (u_long, RE_TRANSLATE_TYPE, the utmp/wtmp path macros, xxhash's S_IF* configuration): fix stories;
   not patched around. 4e waits for them.
2. heap_meta_products `find_over_200000_entries_bounds_its_address_space` red on next at 44304e709 (keke, Zen 1).
   Inference from the numbers, not verified: the absolute need is unchanged (packaged 45,744 vs 45,728) while the
   3-file base fell 9,600 KiB with this head's smaller start-up reservation, so the "over 3 entries" difference grew.
3. Start-up +12.1k user instructions with 3 libraries (not chased; candidates by subject only: ld-start-maps-3,
   ldso-program-mode-space, shstk-hw-enable).
4. Docs: the two stale "no headers" texts; niji-cc in the fragment.

## Exact next step
Coordinator: rule on publishing 44304e709 with the gaps (recommendation in the hand-back); brief the header gaps.
Staged: keke `~/release-staging/snapshot-20261124.2/` (7 system + 2 coexist .debs, SHA256SUMS, MANIFEST). Nothing to
GitHub, no tag.
