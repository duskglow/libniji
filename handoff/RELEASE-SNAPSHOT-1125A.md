COMPLETE: NOT snapshot-ready for apt at 522900c6f — every 24.2 gap is closed (identifier probe 104 SAME / 0 DIFF; 7 of 7 Debian packages build, tree and xxhash included; header sweeps 0 FAIL in C and C++, both flavours); apt now configures (sig_t found) but does not build: 3 more header gaps (siginfo_t si_syscall, HEADER, C++ socket-call sockaddr arguments), briefed as HEADERS-APT-GAPS; static busybox still stops on net/if.h / sys/mtio.h (not blocking, HEADERS-BUSYBOX-GAPS); every other check equals 24.2's; start-up identical (same libc.so.6 and ld.so)

# RELEASE-SNAPSHOT-1125A: snapshot validation of 522900c6f (HEADERS-SNAPSHOT-GAPS: apt's sig_t, tree, xxhash, static busybox)

**State.** Head under validation `522900c6fc202464bc87b8ba570f67bb85b3fcda` (origin/next at the start, 2026-11-25
02:42Z, "Merge task/headers-snapshot-gaps into next"; commit time 2026-11-25T02:14:27Z, epoch 1795572867).
First-parent log since 24.2 (44304e709): 4 merges: headers-form-breaks (460d23b8f), release-snapshot-1124d
(685bc1ac7), sl-headers-p4-callsites (623550dac), headers-snapshot-gaps (522900c6f). No 2026-11-25 snapshot yet, so
**serial 1**: system `2.39+niji20261125.1-1`, coexist `0.0~snapshot20261125.1+522900c6fc20`; `dpkg --compare-versions`
gt 24.2's for both. Branch `task/release-snapshot-1125a` = head + handoff/ only. Role keke `~/w/n1` (Zen 1: products,
.debs, gate, plants, staging); root checks and Skylake-SP counts on mei; products and .debs relayed keke -> local ->
mei, `sha256sum -c` OK at each hop (9 .debs).

**Job dirs.** keke and mei `~/work/RELEASE-SNAPSHOT-1125A/` = 1124D's scripts through `copy.sh` + `subst.sed` (head,
epoch, serial 1, labels; comparison side 24.1 -> 24.2: 1124D's debs, products and logs, `-ref242`, p224; this head is
p225). New (mei): `hp.sh` (1124D's 22 probes + sig_t, sighandler_t, S_ISDIR/S_ISREG per feature-macro family in C and
C++, the path macros in default mode, RE_TRANSLATE_TYPE's type: 104 probes), `hp2.sh` (the three apt gaps),
`yapt.sh` + `yapt/` (apt from trixie, `dpkg-buildpackage -b -uc -us -d`, stock then swapped; on failure `ninja -k 0`
lists every failing unit), `run-hdr.sh` (yh, yhc, ypk, hp, yapt once the .debs arrive), `cmp9.sh` (cmp8 with the mask
widened to 25 / 522900c6f / ref242 / p225). **Copy defect fixed:** `subst.sed`'s `ref241 -> ref242` also renamed
y23p's data file `ref241.txt` (the trixie 2.41 stock table, not snapshot 24.1), so y23p-1 judged against a missing
file (fail=2); `y23p/post.sh` restored, y23p-2 rc 0. Next copy: exclude data-file names from that rule.
Nothing of this run is left running on either host.

## The gaps (step 4)

| step | job | result |
|---|---|---|
| 4a identifier probe | mei hp-1 (hp-ref242-1: 24.2) | **104 SAME, 0 DIFF** (72 both PASS, 32 both FAIL): sig_t, sighandler_t, u_long, RE_TRANSLATE_TYPE (and its type), _PATH_WTMP/_PATH_UTMP/WTMP_FILE/UTMPX_FILE/WTMPX_FILE, S_IFMT/S_IFREG/S_IFDIR and S_ISDIR/S_ISREG in default, c99, c11, gnu11, GNU, DEFAULT, BSD, POSIX 1/200112L/200809L, XOPEN 500/700, LFS64, fortify; C and C++. On 24.2 the same script: 48 DIFF (sig_t, S_ISDIR under -std=c11/c99/POSIX: the xxhash configuration) |
| 4b apt 3.0.4 | mei yapt-2 (yapt-1 the same, no -k 0; yapt-ref242-1: 24.2) | **FAIL on libniji only.** Stock: rc 0, 10 debs, aptTests passed. 24.2: configure stops "Platform defines neither sig_t nor sighandler_t" (Yohane's error reproduced). This head: configure finds sig_t and sighandler_t, the build fails: `ninja -k 0` 15 failed units, 3 gaps, each probed against stock 2.39 (mei hp2-1: 20 DIFF of 24): (1) `siginfo_t` has no `si_syscall` (si_arch, si_call_addr probed with it; C and C++, every mode; methods/aptmethod.h, 11 units); (2) `<arpa/nameser.h>` declares no `HEADER` (C and C++, every mode; apt-pkg/contrib/srvrec.cc); (3) **C++ only**: `struct sockaddr *` to connect/bind/accept/getsockname/getpeername/recvfrom/sendto does not convert to `__SOCKADDR_ARG` / `__CONST_SOCKADDR_ARG`, nor `accept(s, 0, 0)` (methods/connect.cc, gtest.cc, aptwebserver.cc); C is fine. Fix story HEADERS-APT-GAPS (coordinator 04:05Z) |
| 4c Debian packages | mei ypk-1 | **7 of 7 build on libniji** (lz4, dash, tree, libyaml, xxhash, tinyxml2, libjsoncpp; 24.2: tree and xxhash failed) |
| 4d static busybox | mei ypk-1 | **does not compile** (stock builds; make rc 2; not blocking per coordinator 02:44Z, HEADERS-BUSYBOX-GAPS): `struct ifreq` members ifr_name, ifr_hwaddr, **ifr_qlen** (the last not in HEADERS-SNAPSHOT-GAPS's list; networking/arp.c, libiproute/ipaddress.c, libbb/xconnect.c); `<sys/mtio.h>` struct mtop / mtpos, MTIOCTOP, MTIOCPOS and the MT* operation codes (miscutils/mt.c). make stops at the first failing directory, so arping.c's PACKET_* (that report) was not reached. No u_long, RE_TRANSLATE_TYPE, wtmp or S_IFMT error is left. The applet checks could not run |
| 4e header sweep | mei yh-1, yhc-1 | system flavour: 236 names, 234 compile/link/run in C and C++, 2 refuse direct inclusion as Debian's do, **0 FAIL**; 0 stale, 0 differing; C++ programs 4 of 4 equal to stock. Coexist: niji-gcc/niji-g++ 0 FAIL, C++ 4 of 4, /usr/include unchanged; the niji-cc rows fail "not found" (233) as in 24.2 (the packages ship no niji-cc; the job script still probes it). Every summary line equals 24.2's |

## Everything else rerun (labels `release-snapshot-1125a-<x>`)

| check | job(s) | result |
|---|---|---|
| build + smoke | keke smoke-keke-1, mei smoke-mei-1 | `SMOKE: RESULT bad=0` both |
| gate | keke gate-1 | `GATE bad=0` |
| #21 heap products test | keke heapnode-1 | 6 passed, 1 FAILED, as 24.2: `find_over_200000_entries_bounds_its_address_space` 36,672..38,464 KiB over 3 entries > 35,000 (numbers identical to 24.2's) |
| debs, check, check-symbols | keke debs-1 | `DEBS bad=0`; check PASS (7); check-symbols PASS (4490 lines, 14 libraries; = 24.2) |
| plants | keke plants-1 | 6 CAUGHT, 0 SURVIVED, no WARN-RC-ONLY / WARN-NO-MSG (WARN-DEV-SHM as briefed): handoff/RELEASE-SNAPSHOT-1125A-plants.txt |
| package diff vs 24.2 | keke pkgdiff-1 | file counts equal for every package (libc6-dev 296, niji6-dev 317, locales 387, ...); no package dropped |
| stage, install | keke stage-1, install-1 | staged 9 + SHA256SUMS + MANIFEST; `inst: RESULT fails=0`, system debs identical 7 |
| getaddrinfo_a | keke gaimass-1 | `GAIMASS bad=0` |
| root checks | mei: smoke-mei, roots, lint, yohane, many, dyn22, y10, y19, y26, y26t, c19, y23, y25, y24, y28, y6, y25s, y21, yres, y12, ysk, dlc, dlcd, y13, y14, y14l, y14r, y2411, y25n, y25c, y6c, y6r, y13s, y13v, drp, y27, y23p (-2), sig32, y20, dpo600, eetwet, mint (+ -ref242 sides) | `cmp9.sh`: **all 45 masked verdict sets equal 24.2's** (0 differing lines). rc 1 by design as in 24.2: dlcd, y14, y14r, drp |
| #21 (packaged find) | mei y21-1 / y21-ref242-1 | verdicts equal 24.2's; formula 5 of 5; find over 3 files 12,544 KiB (= 24.2) |
| lint | mei lint-1 | errors 10, warnings 50 (= 24.2, masked lines equal) |

**Start-up** (user instructions / page faults per start, medians): this head's libc.so.6 (ad64d0d4af11f451) and ld.so
(534bec363e074f93) are byte-identical to 24.2's, so every row equals 24.2: Zen 1 empty main 156,289 / 52, 3 libraries
241,564 / 67, clock_gettime 167,237 / 56 (glibc 101,533 / 136,734 / 103,672); Skylake-SP 156,531 / 241,821 / 167,450
(glibc 107,738 / 143,080 / 109,848). The user-side rows of kstart and rstart (libresolv pair) equal 24.2's too.

## Docs (step 5)
The "no headers" texts are gone: tools/deb/description/libc6-dev describes the headers; README.md says the system
packages ship libniji's own C headers; the SL-HEADERS-S20 fragment no longer names niji-cc; docs/install-packages.md no
longer names a.out.h. docs/RELEASE-NOTES.md still says "no C headers" only in the 24.1 and earlier sections, correct
for those snapshots. Release notes for this snapshot should state: apt does not build on the system flavour yet (the
three gaps), static busybox does not compile (net/if.h, sys/mtio.h).

## Findings for the coordinator
1. apt: three header gaps (4b), HEADERS-APT-GAPS. (3) breaks any C++ program passing a `sockaddr *` to a socket call.
2. busybox static: `ifr_qlen` adds to HEADERS-BUSYBOX-GAPS's list.
3. heap_meta_products `find_over_200000_entries...` still red on next (unchanged from 24.2).
4. Tooling: copy.sh/subst.sed renames data files whose names contain a ref label (y23p's ref241.txt).

## Exact next step
Coordinator: publish 522900c6f with apt and static busybox as stated limitations (ruling 04:05Z), re-cut when
HEADERS-APT-GAPS lands (rerun yapt.sh and hp2.sh: both must pass). Staged: keke
`~/release-staging/snapshot-20261125.1/` (7 system + 2 coexist .debs, SHA256SUMS, MANIFEST). Nothing to GitHub, no tag.
