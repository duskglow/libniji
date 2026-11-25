WIP (leg 1 of RELEASE-SNAPSHOT-1125B): every job is started on the fixed head; results are being collected.

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
on mei; products and .debs relayed keke -> local -> mei, `sha256sum -c` OK at each hop (9 .debs, mei 11:54Z).

**Job dirs.** keke and mei `~/work/RELEASE-SNAPSHOT-1125B/` = 1125A's scripts through `copy.sh` + `subst.sed`
(head, epoch, serial 2, role n1 -> n3, labels; comparison side 24.2 -> 25.1: 1125A's debs, products and logs,
`-ref251`, p225; this head is p226). Copy defect of 1125A avoided: subst.sed renames only `-ref242`, so y23p's data
file `ref241.txt` keeps its name (mei's y23p/post.sh names ref241.txt; checked). New on mei: `run-hdr.sh` (hp, hp2,
yapt, ypk, yh, yhc, ystage2 niji, then ystage2cmp), `ystage2.sh` + `ystage2/` (steps 4c and 4f: C++ socket program,
`<stddef.h>`/`<sys/un.h>` in both orders over C and C++ modes, the 43 `<langinfo.h>` items of the five families (the
brief says 34; the header has 43 with the CODESET items) plus `_NL_ITEM`/`_NL_ITEM_INDEX`, nl_langinfo in C and
en_US.UTF-8, and trixie's perl 5.40.1 built with dpkg-buildpackage), `ystage2cmp.sh`, `cmp10.sh` (cmp9 against
1125A's -1 logs; y23p against 1125A's y23p-2). Local relay script and notes: `~/w/.scratch/RELEASE-SNAPSHOT-1125B/`.

## Results so far (labels `release-snapshot-1125b-<x>`)
- keke: smoke-keke-1 `SMOKE: RESULT bad=0`; gate-1 `GATE bad=0`; debs-1 `DEBS bad=0`, check PASS (7 + 2),
  check-symbols PASS (4490 lines, 14 libraries; = 25.1).
- mei: smoke-mei-1 bad=0; startup/kstart/startup215/rstart-mei-1, sig32-1, y20-1, dpo600-1 rc 0; every -ref251
  side rc 0 except dlcd-ref251-1 rc 1 (rc 1 by design, as in 1125A).
- ystage2-stock-3 (stock side, rc 0): socket program builds and runs; un rows 18 PASS, 4 FAIL (max_align_t undeclared
  in c99 and c++98, expected on both sides); li 8 builds, 16 runs rc 0, 45 lines each; perl: the checked build fails
  on STOCK (Net-Ping's 4 ICMP tests, twice: unshare root, no raw sockets), the nocheck pass packages 7 debs.
  SUPERSEDED: ystage2-stock-1 (stopped before start: sock.cc lacked <cstdlib>), ystage2-stock-2 (probe defects: AF_UNIX
  without <sys/socket.h>; li.c read non-string items as strings; perl tests stop the build).

## Exact next step (leg 2)
1. keke: `~/bin/jobwait` heapnode-1, plants-1, gaimass-1, pkgdiff-1, stage-1, install-1, startup-keke-1,
   startup215-keke-1, kstart-keke-1, rstart-keke-1 (run-keke.out ends "run-keke: done"). Plants: append the results
   block to handoff/RELEASE-SNAPSHOT-1125B-plants.txt as 1125A did.
2. mei: run-mei.out / run-hdr.out / run-ref.out end "done". Then `sh cmp10.sh` (default list) and
   `sh cmp10.sh y24 y28 y6 y25s yres y23p dlc dlcd y27 y13 y14 y6c y14l y25n y25c y2411 y12 ysk y14r eetwet y13s drp y6r
   y13v y21 mint startup-mei kstart-mei sig32 y20 dpo600 hp yh yhc ypk` into a file; read the summary.
3. Blocking (step 4): hp2-1 (24 rows, 0 DIFF), hp-1 (104 rows, 0 DIFF), yapt-1 (PASS, 10 debs, tests as stock),
   ystage2-niji-1 + ystage2cmp-1 (sock OK on libniji; un rows equal; li files equal; perl debs and failing tests equal
   to stock), ypk-1 (7 of 7; busybox: what still stops it), yh-1 / yhc-1 (0 FAIL).
4. Start-up rows (startup-*, kstart, rstart) against 25.1's; lint-1 against 25.1.
5. Read the scratch dir's `notes-private.txt` (hand-back only, never the repo). Rewrite this note (first line COMPLETE: snapshot-ready at 5a0acbb35, or the blocking findings, apt first), the json
   report, clippy + static + precheck on the role, handback-check, hand-back with every label in JOBS.
