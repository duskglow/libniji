BLOCKING: not snapshot-ready at 8e1837914 (serial 4, niji6 sorts above 22.3's): B1 dladdr and _dl_find_object name the loader for an address inside a dlopen'd library (dlc, new since 22.3)

# RELEASE-SNAPSHOT-1122D: snapshot validation of 8e1837914 (HEAP-SECOND-NODE-MAP #21, THREAD-EXIT-NO-UNWINDER #25)

**State.** Head under validation `8e1837914bb99339e2ea4da6dd2893382c1c9782` (origin/next at the start, 2026-11-22 23:06
UTC, "Regenerate train-owned views"; commit time 2026-11-22T22:18:37Z, epoch 1795385917); contains 2786f94e5
(heap-second-node-map) and d258cd270 (thread-exit-no-unwinder). Commit date 2026-11-22, 22.3 was serial 3, so
**serial 4**: system `2.39+niji20261122.4-1`, coexist `0.0~snapshot20261122.4+8e1837914bb9`.
`dpkg --compare-versions`: `0.0~snapshot20261122.4+8e1837914bb9 gt 0.0~snapshot20261122.3+2a76723f2ed4` yes;
`2.39+niji20261122.4-1 gt 2.39+niji20261122.3-1` yes. Branch `task/release-snapshot-1122d` = head + handoff/ only.
Role keke `~/w/n4` (Zen 1, AMD EPYC 7551); root checks and Skylake-SP counts on mei (products and .debs relayed through
the local machine, sha256 checked on arrival: 9 of 9 .debs OK). Nothing to GitHub, no tag.

**Landed since 22.3** (first-parent, 2a76723f2..8e1837914): release-snapshot-1122c, sl-headers-s06-residue,
dl-process-objects-3, ldso-run-program-2, dl-reader-costs, headers-null-type, stdio-caller-alloc, locale-charmaps-gen,
libc-init-array-refs, sl-headers-s18-3, heap-second-node-map, thread-exit-no-unwinder, ldso-program-options-rest-2,
plus the train's view regenerations.

## Blocking finding

**B1 (correctness, new since 22.3): dladdr and _dl_find_object attribute an address inside a dlopen'd library to
the dynamic loader.** dlc (DL-PROCESS-OBJECTS' 300-library corpus, swapped trixie root, Skylake-SP): `dladdr(dlsym(h,
"fK"))` names `libpK.so` for 299 of 300 objects (stock 300, 22.3 300), in both rounds and both link modes; dlc-2
reproduces it. Diagnostic (dlcd-1/-2: dlc with a probe printing each miss): the one miss is k=127; `dladdr` returns
`/lib64/ld-linux-x86-64.so.2`, the loader's base, no symbol; `_dl_find_object` returns the loader's link map, map
[base, base+1 MiB) and **the loader's eh_frame** (the lookup the C++ unwinder uses). libp127.so's own base (from
dl_iterate_phdr) is base+0x62000, i.e. inside the loader's span. Cause, from the products' program headers: this
head's ld.so has a 60 KiB hole between its text segment (vaddr 0x1000, memsz 0x60ea7) and its read-only segment
(vaddr 0x71000); 22.3's hole was 16 KiB (text ends 0x5c2e7, read-only at 0x61000). The kernel maps the loader and
leaves the hole free; libp127.so is mapped into it (base+0x62000, the hole's first page), and both lookups match by the loader's whole
span [first PT_LOAD, last PT_LOAD end) instead of its segments. So the lookup defect predates this head (22.3's 16 KiB
hole could take a smaller mapping); the larger hole makes it reachable by an ordinary library, and any mapping placed
there (an anonymous JIT region too) would be misattributed. Outside a root (`ld.so --library-path`, the program
mapped by the loader) the same corpus gives 0 misses, so this is layout-dependent. Recommend a story (ruling asked):
the address lookups (dladdr, dladdr1, _dl_find_object, and any other span-based reader) match an object only
within its PT_LOAD segments, with a test that maps an object into the loader's hole (or the hole reserved
PROT_NONE at start-up, if measured cheaper); then rerun dlc.

## Findings (non-blocking)

* **F1 (#25, niji-g++, confirms D3):** a C++ program built with the coexist flavour's niji-g++ (which links
  libstdc++ and the unwinder statically: tools/deb/niji-gcc) whose thread calls pthread_exit or is cancelled with
  destructors on its stack now aborts (status 134) with `FATAL: an exiting or cancelled thread has frames to unwind
  (cleanups, destructors or handlers) and no unwinder can be found; link libgcc_s dynamically (not -static-libgcc) in
  a program whose threads exit or are cancelled`. On 22.3 the same build exited 0 without running a destructor (y25c
  ref223: `join: 7`, no dtor lines). g++ controls unwind on both. The new behaviour is the accepted D-22.23 design and
  better than a silent skip, but the fix the message names cannot be applied through niji-g++, and
  docs/install-packages.md:70 does not say so. Recommend (ruling asked): niji-g++ links libgcc_s dynamically for
  threaded C++ (or the docs say to use g++ with -L/-rpath for such programs); not blocking.
* **F2 (#21, address space, find at depth 200k):** the find formula holds on both (5 of 5 under `ulimit -v`
  need3 + 35000), but at depth 200k this head needs 53,968 KiB (22.3 49,296; stock 31,616), 32,368 KiB over depth 3
  (22.3 27,648; stock 26,976; allowance 35,000): 4,720 KiB more than 22.3, margin 2,632 KiB. Single-node rows equal
  22.3 (the change targets the second node). Recommend the #21 owner look at the second-node growth step's share.
* **F3 (#14 check out of date):** y14l's two FAIL lines are its 22.3 expectations (no charmap shipped); with
  LOCALE-CHARMAPS-GEN they now read: UTF-8 charmap present, and a supplied source compiles with `-f UTF-8` (rc 0).
  Update y14l for the next snapshot.
* **D1 (stale):** docs/install-packages.md:150 and the `locales` package Description ("locale support
  (placeholder)") still call locales a placeholder; it now ships 10 generated charmaps.
* **D2 (missing):** no docs sentence for HEAP-SECOND-NODE-MAP's address-space behaviour.
* lintian: errors 10 (as 22.3), warnings 50 -> 49: `locales: empty-binary-package` is gone (the charmaps).
* gaimass prints six `cannot open target/test-oracles/.../gaimass.timing` lines, as 1122B's and 1122C's runs did
  (not new); `GAIMASS bad=0`.

## Results

| check | job(s) | result vs 22.3 |
|---|---|---|
| build + smoke, Zen 1 / Skylake-SP | keke smoke-keke-1, mei smoke-mei-1 | `SMOKE: RESULT bad=0` both (mei differs only in layout-verdict cache lines) |
| gate | keke gate-1 | `GATE bad=0`, peak 1.8G (clippy all-features, xtask clippy, static, records, direct tier) |
| #21 heap products test | keke heapnode-1 | 6 passed |
| .debs, check, check-symbols | keke debs-1 | `DEBS bad=0`; check PASS 7 + 2 packages; check-symbols PASS (4440 lines, 13 libraries) |
| package diff vs 22.3 | keke pkgdiff-1 | versions; build-ids; locales 7 -> 19 files (10 charmaps); libc6 +28 KiB installed |
| staging, install steps (docs/install-packages.md) | keke stage-1, install-1 | 9 assets; `inst: RESULT fails=0` |
| plants (6 release-check plants) | keke plants-1 | 6 CAUGHT, 0 WARN |
| getaddrinfo_a | keke gaimass-1 | `GAIMASS bad=0` |
| roots, lint, yohane, many, dyn22, y10, y19, y26, y26t, y23, c19, y24, y28, y6, y25s, yres, y23p, y27, y13, y14, y6c, y14r, eetwet, sig32, y20, dpo600, drp, y13s, y13v, y6r, y21 | mei (-1 each) | masked verdict lines equal to 22.3's (cmp5.sh); rc 1 by design as on 22.3: roots, yohane, y14, y14r, drp |
| dlc | mei dlc-1, dlc-2 | **FAIL 2 (B1)**; 22.3 PASS |
| mint | mei mint-1 | `MINT fail=0`; internal rows differing from stock 40 -> 30 (STDIO-CALLER-ALLOC) |
| #14 | mei y14l-1, y14l-ref223-1 | see F3 and the #14 table |
| #25 | mei y25-1, y25-ref223-1, y25n-1, y25n-ref223-1, y25c-1, y25c-ref223-1 | see the #25 table |
| side-by-side 22.3 | mei drp-ref223, y13s/y6r/y13v/y21-ref223 | equal to the head's except y21 (F2) |

**#25** (fu.cc, the issue's rows; trixie root, libniji swapped in; stock = trixie's C library):

| row | stock | 22.3 | head |
|---|---|---|---|
| destructor in a frame calling pthread_exit (exit-dtor) | rc 0, dtors run | same as stock | same as stock |
| catch abi::__forced_unwind and rethrow (exit-fu) | rc 0 | same | same |
| cancelled thread blocked in pause() (cancel-dtor) | rc 0, dtors run | same | same |
| y25's other rows (cancel-fu, exit-all, cancel-all, exit-lock, ptask, async) | | same, `RESULT fail=0` | same, `RESULT fail=0` |
| -static-libgcc, libstdc++ dynamic (fug, 3 rows) | rc 0, dtors | same | same |
| -static-libgcc -static-libstdc++, no libgcc_s loaded (fus, 3 rows) | rc 134, stderr 8 B, no dtor line | **rc 0**, no dtor line (`join: 7`) | rc 134, FATAL message (229 B, cause and fix), no dtor line |
| plain-C thread calling pthread_exit (pc) | rc 0, 19 B | same | same |
| niji-g++ build (coexist, y25c, 3 rows) | (g++ control: rc 0, dtors) | rc 0, no dtor lines | rc 134 + FATAL (F1) |

**#21** (y21, issue probe and find formula under `ulimit -v`): no-limit steps pass; malloc to the limit: room left
0 / 0 / 48 / 696 / 11,960 KiB for 100 B / 4 KiB / 128 KiB / 1 MiB / 16 MiB (head = 22.3; stock 64 / 48 / 64 / 640 /
16,240); probe need 40,816 KiB (stock 29,568), under 35,000 the probe's malloc returns NULL (its exit 11), under 41,296
rc 0, head = 22.3; find need depth 3: 21,600 (22.3 21,648, stock 4,640); depth 200k: F2; find formula 5 of 5 on all.

**#14 delta** (y14l, bash's exact command in the swapped root with only libniji's packages): head and 22.3 both
status 4, `[error] cannot open locale definition file 'en_US': No such file or directory`, no file written (stock's
status for a missing source). Still missing: the en_US source (LOCALE-SOURCES-GEN). New: with a source supplied,
`-f UTF-8` resolves the generated charmap (rc 0; 22.3 rc 4 `character map file 'UTF-8' not found` plus a default
ANSI_X3.4-1968 line); the missing-charmap stderr now equals stock's. The test's own en_US + UTF-8 compile still reads
back equal to stock (37 lines). L1 (six POSIX categories only) unchanged.

**Start-up** (user instructions / page faults per start, medians; p220 = 22.3, p221 = head):

| row | Zen 1: glibc | 22.3 | head | change | Skylake-SP: glibc | 22.3 | head | change |
|---|---|---|---|---|---|---|---|---|
| empty main | 101,533 / 48 | 260,618 / 51 | 253,613 / 51 | -7,005 | 107,752 / 49 | 260,857 / 51 | 253,852 / 51 | -7,005 |
| 3 libraries | 136,734 / 64 | 370,892 / 66 | 361,279 / 66 | -9,613 | 143,094 / 65 | 371,146 / 66 | 361,533 / 66 | -9,613 |
| clock_gettime | 103,672 / 50 | 274,120 / 55 | 267,119 / 55 | -7,001 | 109,862 / 50 | 274,330 / 54 | 267,329 / 54 | -7,001 |
| libresolv pair (res0) | 162,716 / 54 | 373,764 / 56 | 365,778 / 56 | -7,986 | 169,020 / 55 | 374,316 / 56 | 366,330 / 56 | -7,986 |

Headline: LIBC-INIT-ARRAY-REFS's ~7,000 fewer user instructions per start on every row and both classes (its own
claim -6,981 on Zen 1); faults unchanged. Kernel-side counts (kstart/rstart KCOUNT lines) are noisy run to run
(min-max spread ~100k) and not compared.

**Staged** (keke `~/release-staging/snapshot-20261122.4/`, MANIFEST + SHA256SUMS, `sha256sum -c` 9 of 9 OK; staged,
NOT for publication while B1 stands): libc-bin, libc-dev-bin, libc-l10n, libc6-dbg, libc6-dev, libc6
(`2.39+niji20261122.4-1_amd64`, l10n `_all`), locales (`_all`), niji6 and niji6-dev
(`0.0.snapshot20261122.4+8e1837914bb9_amd64`, '.' for '~'). libc6 sha256 43c60b67...; niji6 99774322....

## Procedure (for the next snapshot)

1122C's scripts copied on both hosts to `~/work/RELEASE-SNAPSHOT-1122D/` (copy.sh, subst.sed: head, epoch, serial 4,
22.3 as the comparison side, `-ref223`, role keke n4). keke `run-keke.sh` (role jobs in sequence, then plants,
gaimass, pkgdiff/stage/install); mei `run-mei.sh` (waits for products.sha / debs.sha, then every root check).
New this run: y25n (`-static-libgcc` and `-static-libgcc -static-libstdc++` builds, plain-C pthread_exit), y25c
(niji-g++ in a trixie root with niji6/niji6-dev), cmp5.sh (cmp4.sh's mask widened to the 22nd and 1122x ids;
cmp4.sh compared 22.x version strings as differences), dlcdiag/ and dlcd.sh (B1's diagnostic: diag.c prints each
dladdr / _dl_find_object miss). Next run: update y14l (F3), keep dlcd beside dlc until B1 is fixed.

## Exact next step

Coordinator: rule on B1 (fix story; snapshot 22.4 not published) and F1; after B1's fix lands, a new snapshot run.
