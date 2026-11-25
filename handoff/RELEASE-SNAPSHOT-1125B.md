COMPLETE: apt snapshot-ready at 5a0acbb35 — apt 3.0.4 builds on the system flavour (10 debs as stock, aptTests pass), hp2 24 SAME / 0 DIFF, hp 104 / 0 DIFF, the C++ socket program builds and runs, 7 of 7 packages, header sweeps 0 FAIL; stage-2 4f: (a) <stddef.h>/<sys/un.h> equal to stock, (b) the langinfo constants compile and every value equals stock 2.39 except en_US.UTF-8's LC_IDENTIFICATION metadata (7 items, RULING NEEDED), (c) trixie's perl 5.40.1 does NOT build on libniji (<sys/resource.h> __priority_which_t, <netdb.h> h_addr); every other check equals 25.1's

# RELEASE-SNAPSHOT-1125B: snapshot validation of 5a0acbb35 (apt: siginfo_t members, nameser.h HEADER, C++ sockaddr arguments; stage-2 gaps)

**State.** Head under validation `5a0acbb35533d110bf24398ead22ef59e984b756` (origin/next at the start, 2026-11-25
10:51Z, "Regenerate train-owned views"; commit time 2026-11-25T10:17:29Z, epoch 1795601849). It contains
HEADERS-APT-GAPS-2's merge (5691ac746). First-parent log since 25.1 (522900c6f): wl-prep-headers (8e7d7bf1b),
sl-headers-complex-tgmath-2 (36a749ba7), release-snapshot-1125a (de74d9286), headers-busybox-gaps (855efa170),
headers-cxx-enum-types-3 (ffc84009f), headers-stage2-gaps-a (bde77db27), provenance-fragments (a89a9f9b4),
sl-headers-unshipped-rest-4 (22240e072), headers-apt-gaps-2 (5691ac746), plus train view regenerations (77e2b4b49,
88d22c9a2, ea51e210f, e125f7e34, 5a0acbb35). snapshot-2026-11-25.1 exists, so **serial 2**: system
`2.39+niji20261125.2-1`, coexist `0.0~snapshot20261125.2+5a0acbb35533`. Branch `task/release-snapshot-1125b` = head +
handoff/ only. Role keke `~/w/n3` (Zen 1: products, .debs, gate, plants, staging); root checks and Skylake-SP counts
on mei; products and .debs relayed keke -> local -> mei, `sha256sum -c` OK at each hop (9 .debs).

**Job dirs.** keke and mei `~/work/RELEASE-SNAPSHOT-1125B/` = 1125A's scripts through `copy.sh` + `subst.sed` (head,
epoch, serial 2, role n1 -> n3, labels; comparison side 24.2 -> 25.1: 1125A's debs, products and logs, `-ref251`,
p225; this head is p226). 1125A's copy defect avoided: subst.sed renames only `-ref242`, so y23p's data file
`ref241.txt` keeps its name. New on mei: `run-hdr.sh`, `ystage2.sh` + `ystage2/` (steps 4c and 4f), `ystage2cmp.sh`,
`cmp10.sh` (cmp9 against 1125A's logs), `hp3.sh` (perl's header gaps, hp2's method), `li239.sh` (li.c on the host's
glibc 2.39, because ystage2's stock side is trixie's 2.41). Nothing of this run is left running on either host.

## apt and the stage-2 gaps (step 4, blocking)

| step | job | result |
|---|---|---|
| 4a apt-gap probe | mei hp2-1 | **24 SAME, 0 DIFF** (si_syscall/si_arch/si_call_addr, HEADER, sockaddr arguments incl. `accept(s, 0, 0)`; gcc and g++, 4 feature modes); 25.1: 20 DIFF |
| 4a identifier probe | mei hp-1 | **104 SAME, 0 DIFF** (= 25.1) |
| 4b apt 3.0.4 | mei yapt-1 | **PASS**: stock rc 0, libniji rc 0, the same 10 debs, `aptTests` passed on both (1 of 1). libniji's build has 14 more warnings: 13 are in the hand-back only, 1 is `-Wundef` "`__RES` is not defined" (apt-pkg/contrib/srvrec.cc `#if __RES >= 19991006`: libniji's `<resolv.h>` defines no `__RES`, so apt compiles its other branch) |
| 4c C++ socket program | mei ystage2-niji-1 (stock: ystage2-stock-3) | g++ builds and runs on both: stream + dgram, `accept(s, 0, 0)`, getsockname/getpeername/recvfrom names equal |
| 4d packages, sweeps | mei ypk-1, yh-1, yhc-1 | **7 of 7** build on libniji; yh: 0 FAIL, C++ 4 of 4; yhc: niji-gcc/niji-g++ 0 FAIL, C++ 4 of 4, /usr/include unchanged; its 239 niji-cc rows fail "not found" (233 in 25.1 + the 6 headers new since: aio.h, complex.h, mqueue.h, proc_service.h, tgmath.h, thread_db.h; no niji-cc is shipped) |
| 4f(a) `<stddef.h>` / `<sys/un.h>` | ystage2cmp-1 | 22 rows each side (both orders, gcc 5 modes, g++ 6 modes), **0 differing**: 18 PASS; 4 FAIL on both (max_align_t is not in C99 / C++98) |
| 4f(b) `<langinfo.h>` | ystage2-niji-1, ystage2cmp-1, li239-2 | the 43 items of the address, identification, measurement, name and telephone families (34 in the brief; the header's 43 count the CODESET items) and `_NL_ITEM`/`_NL_ITEM_INDEX` compile in C and C++, default and `_GNU_SOURCE`; 8 programs run rc 0. Values equal stock 2.39's (li239-2: host glibc 2.39; equal to trixie 2.41's too) **except LC_IDENTIFICATION**: "C": address, contact, email, tel, fax empty (documented, D-7.22, differences row); **en_US.UTF-8: 7 items differ** (title "English locale for United States" vs "English locale for the USA", source "libniji" vs the reference's publisher, address and email empty, language "English" vs "American English", revision "49" vs "1.0", date empty vs "2000-06-24"): no differences row found; D-31.30's probed-data rule (#7890) seems to say these should be the reference's. RULING NEEDED |
| 4f(c) perl 5.40.1 | ystage2-*, ystage2cmp-1 (bad=4: the 8 langinfo files + perl), hp3-1 | **does not build on libniji.** Stock (trixie root): the checked build fails only on Net-Ping's 4 ICMP tests (no raw sockets under unshare), the nocheck pass packages 7 debs. libniji: `pp_sys.c` stops all three builds: (1) `<sys/resource.h>` declares no `__priority_which_t` (perl casts getpriority/setpriority's first argument to it under glibc); (2) `<netdb.h>`'s `struct hostent` has no `h_addr`. hp3-1 (stock 2.39 vs the niji6-dev sysroot): 22 DIFF of 32: `__priority_which_t` 8 of 8 (gcc and g++, all 4 modes), `h_addr` 7 (all but gcc `_POSIX_C_SOURCE=200809L`), `struct ip_mreq_source` 7 (`<netinet/in.h>`; perl's Configure sets d_ip_mreq_source undef on libniji); `BSD` undefined after the usual headers on both (SAME), so what makes perl's Configure set `d_bsd` on libniji is not found. Other Configure differences: SIGRTMIN 33 (documented, D-20.1), gnulibc_version 2.39 |
| 5 static busybox | mei ypk-1 | still does not compile (not blocking). The net/if.h and sys/mtio.h gaps are gone (HEADERS-BUSYBOX-GAPS); it stops on: `struct ip6_hdr` has no `ip6_plen`/`ip6_nxt`/`ip6_vfc` (`<netinet/ip6.h>`, networking/udhcp/d6_dhcpc.c), implicit `IN_MULTICAST` (`<netinet/in.h>`, libiproute/iptunnel.c), `TCP_NODELAY` undeclared (networking/nbd-client.c). The applet checks could not run |

## Everything else rerun (labels `release-snapshot-1125b-<x>`)

| check | job(s) | result |
|---|---|---|
| build + smoke | keke smoke-keke-1, mei smoke-mei-1 | `SMOKE: RESULT bad=0` both |
| gate | keke gate-1 | `GATE bad=0` |
| #21 heap products test | keke heapnode-1 | 6 passed, 1 FAILED, as 25.1: `find_over_200000_entries_bounds_its_address_space` 38,464 KiB over 3 entries > 35,000 (every "needs" number in the log identical to 25.1's) |
| debs, check, check-symbols | keke debs-1 | `DEBS bad=0`; check PASS (7 + 2); check-symbols PASS (4490 lines, 14 libraries; = 25.1) |
| plants | keke plants-1 | 6 CAUGHT, 0 SURVIVED, no WARN-RC-ONLY / WARN-NO-MSG (WARN-DEV-SHM as briefed): handoff/RELEASE-SNAPSHOT-1125B-plants.txt |
| package diff vs 25.1 | keke pkgdiff-1 | file counts equal except libc6-dev 296 -> 302 and niji6-dev 317 -> 323 (aio.h, complex.h, mqueue.h, proc_service.h, tgmath.h, thread_db.h added); no package dropped |
| stage, install | keke stage-1, install-1 | staged 9 + SHA256SUMS + MANIFEST in `~/release-staging/snapshot-20261125.2/`; `inst: RESULT fails=0`, system debs identical 7 |
| getaddrinfo_a | keke gaimass-1 | `GAIMASS bad=0` |
| root checks | mei: roots, lint, yohane, many, dyn22, y10, y19, y26, y26t, c19, y23, y25, y24, y28, y6, y25s, y21, yres, smoke-mei, y23p, dlc, dlcd, y27, y13, y14, y6c, y14l, y25n, y25c, y2411, y12, ysk, y14r, eetwet, y13s, drp, y6r, y13v, mint, startup-mei, kstart-mei, sig32, y20, dpo600, hp, yh, yhc, ypk (+ -ref251 sides) | `cmp10.sh` (both lists, 54 sets): every masked verdict set equals 25.1's except yhc (the 6 new niji-cc rows above) and roots (4 STEP/RC lines whose only difference is the job directory's path). rc 1 by design as in 25.1: dlcd, y14, y14r, drp |
| lint | mei lint-1 | masked lines equal 25.1's |

**Start-up** (user instructions / page faults per start, medians): libc.so.6 changed since 25.1 (c78ee370c65bdadf vs
ad64d0d4af11f451) but every row is unchanged: Zen 1 empty main 156,289 / 52, 3 libraries 241,564 / 67, clock_gettime
167,237 / 56 (glibc 101,533 / 136,734 / 103,672); Skylake-SP 156,531 / 241,821 / 167,450 (glibc 107,738 / 143,080 /
109,848); p215 rows and the kstart / rstart user-side rows equal 25.1's.

**Docs.** No "no C headers" text left in README.md, tools/deb/description/ or docs/install-packages.md. Release notes
for this snapshot should state: static busybox does not compile (ip6_hdr accessors, IN_MULTICAST, TCP_NODELAY); perl
5.40.1 does not build from source (`__priority_which_t`, `h_addr`).

## Findings for the coordinator
1. perl (stage 2, 4f(c)): `__priority_which_t` (`<sys/resource.h>`), `h_addr` (`<netdb.h>`) stop the build;
   `struct ip_mreq_source` (`<netinet/in.h>`) changes its configuration. A header story like HEADERS-APT-GAPS.
2. en_US.UTF-8 LC_IDENTIFICATION metadata differs from the reference's (7 items): RULING NEEDED (recommend: take the
   reference's values per D-31.30, or a differences row if the metadata should name libniji).
3. apt: `<resolv.h>` defines no `__RES` (apt builds the other srvrec.cc branch; it builds and its tests pass).
4. busybox static: ip6_hdr accessors, IN_MULTICAST, TCP_NODELAY (non-blocking).
5. heap_meta_products `find_over_200000_entries...` still red on next (unchanged).

## Exact next step
Coordinator: publish 5a0acbb35 as snapshot-2026-11-25.2 (apt unblocked) with perl-from-source and static busybox as
stated limitations, or hold for the perl header story (your call: perl is a stage-2 build, apt was the blocker).
Staged: keke `~/release-staging/snapshot-20261125.2/`. Nothing to GitHub, no tag.
