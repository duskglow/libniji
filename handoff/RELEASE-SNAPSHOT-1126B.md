IN PROGRESS (leg 1 -> refresh): snapshot validation of dff0cc5c1 (HEADERS-STAGE2-GAPS-F: util-linux, inetutils)

# RELEASE-SNAPSHOT-1126B: snapshot validation of dff0cc5c1 (serial 2026-11-26.2)

**State.** Head under validation `dff0cc5c1c071b0a9980428ceb7405303c3e1008` (origin/next at the start, 15:42Z, "Merge
task/release-snapshot-1126a into next"; commit time 2026-11-26T14:49:44Z, epoch 1795704584). Contains GAPS-F's merge
(e5901d8f8; 088bc95e3 regen). First-parent log since 26.1 (cd31515dc): headers-fragments (05995ca18),
docs-catchup-1126 (9cfa6166c), headers-stage2-gaps-f (e5901d8f8), train regen (088bc95e3), release-snapshot-1126a
(dff0cc5c1). No crates/ or csys/ change since 26.1. **Serial 2**: system `2.39+niji20261126.2-1`, coexist
`0.0~snapshot20261126.2+dff0cc5c1c07`. Branch = head + handoff/ only.

**Hosts (coordinator #8745, plan accepted):** the dispatcher gave role shiki `~/w/sh2` (Haswell-EP); builds only there
(smoke products, gate, .debs, plants, heapnode: `~/work/RELEASE-SNAPSHOT-1126B/run-shiki.sh`). Products and .debs
relayed shiki -> local -> keke, mei by the local `~/w/.scratch/RELEASE-SNAPSHOT-1126B/relay.sh` (sha256 checked at
every hop: products 3 OK, debs 27 OK). Job dirs keke and mei `~/work/RELEASE-SNAPSHOT-1126B/` = 1126A's via `copy.sh` +
`subst.sed` (head, epoch, serial 2, role n1 -> sh2, 25.2 -> 26.1 comparison, `-ref252` -> `-ref261`, p227 -> p228;
data file names untouched: `y23p/ref241.txt` kept). `logs-ystage2-stock` copied from 1126A (stock side). New: mei
`yb7.sh` (all 59 names of GAPS-F's probe list, yb6's method, 4 modes), mei `run-extra.sh` (hp3, hp4, yb6, yb7, gai6,
ybuild, fp; the 26.1 side of these is 1126A's own logs, no -ref261 rerun), keke `run-keke2.sh` (no role on keke).

**Products sha256 (both sides, as the coordinator asked):** libc.so.6 `cf5043d13be5f2be...` and
ld-linux-x86-64.so.2 `6c0bda72da3223c2...`: identical to 26.1's (built on keke Zen 1); of 1686 product files 144
differ, all 143 headers under target/niji/include plus .products-manifest.

## Results so far (labels release-snapshot-1126b-<x>)
* shiki: smoke-shiki-1 `SMOKE: RESULT bad=0`; gate-1 `GATE bad=0` (clippy, xtask clippy, static, records check,
  affected); debs-1 `DEBS bad=0`, check PASS, check-symbols PASS (4490 lines, 14 libraries).
* keke (Zen 1): smoke-keke-1 bad=0; pkgdiff-1 file counts equal to 26.1 for every package (only version Depends
  change); stage-1 `~/release-staging/snapshot-20261126.2/` (9 debs + SHA256SUMS + MANIFEST); install-1 `inst: RESULT
  fails=0`, system debs identical 7; gaimass-1 `GAIMASS bad=0`; hdrdiff-1 (26.1 libc6-dev vs this) 143 headers changed,
  **0 headers with removals** (rc 0; 26.1 had S_TYPEISTMO), added public names = GAPS-F's (AUTHTYPE_NAME...).
* Start-up (medians, user instructions / faults) equal 26.1's: Zen 1 156,289/52, 241,564/67, 167,237/56 (glibc
  101,359 / 136,560 / 103,498); Skylake-SP 156,531/52, 241,821/67, 167,450/55 (glibc 107,390 / 142,732 / 109,500); p215
  Zen 1 medians 741,443 / 763,564 (26.1 +1 each: ±1 noise, min equal). kstart/rstart: not yet compared (no COUNT lines;
  compare their user-side rows as 1126A did).
* mei finished rc 0: smoke-mei-1 bad=0, sig32, y20, dpo600 fail=0, and the root checks listed in jobs.tsv; drp-1 and
  dlcd-1 rc 1 (by design, as 26.1).

## Still running at the refresh (16:45Z)
* shiki: plants-1 (6 plants, 2 copies), then heapnode-1 (run-shiki.sh sequential).
* mei: run-mei.sh / run-hdr.sh / run-extra.sh: ystage2-niji-1 (perl, ~30 min), ybuild-1 (~20 min), yapt-1, ypk-1, fp-1,
  then ystage2cmp-1; the hp*/yb* probes may be done.

## Exact next step (leg 2)
1. `~/bin/jobwait` on shiki plants-1 heapnode-1; copy the plants result lines into handoff/RELEASE-SNAPSHOT-1126B-plants.txt.
2. mei: jobwait every release-snapshot-1126b-* label; read TOTAL lines: hp-1 (26.1: 104/0), hp2-1 (24/0), hp3-1 (32/0),
   hp4-1 (26.1 hp4-2: same=626 diff=46, the 32 intN rows must now be SAME: expect diff=14 syslog-names-alone only),
   yb6-1 (26.1: 84 DIFF -> expect 0), yb7-1 (expect diff=0), fp-1 (`use` 0 differing; forms vs 1126A's logs-fp, run
   formsx.sh/visx.py as 1126A), ybuild-1 (util-linux and inetutils must build: step 5 blocking), yapt-1, ypk-1 (7 of
   7, static busybox), yh-1, yhc-1, ystage2-niji-1 + ystage2cmp-1 (perl 7 debs, bad=1 = langinfo only), gai6-1.
3. mei `sh ~/work/RELEASE-SNAPSHOT-1126B/cmp11.sh` (already substituted: compares 1126b-<job>-1 with 1126a-<job>-1)
   with both job lists 1126A used: the default list and the job names in `=== <job>:` lines of mei
   `~/work/RELEASE-SNAPSHOT-1126A/cmp11-b.out` (grep them out; do not guess), plus lint-1 masked lines.
4. Write the report (first line `COMPLETE: snapshot-ready at dff0cc5c1` or the blocking findings, steps 4 and 5
   first), json with jobs, precheck on sh2 (`cargo xtask precheck --jobs <labels> --head <sha>` in a jobrun), push,
   handback-check.
