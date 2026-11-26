WIP (leg 1 of RELEASE-SNAPSHOT-1126A): snapshot validation of cd31515dc running; nothing judged yet

# RELEASE-SNAPSHOT-1126A: snapshot validation of cd31515dc (Yohane stage-2 header fixes: GAPS-D-6, GAPS-E-5, FEATURES-FORMS-5)

**State.** Head under validation `cd31515dc8be14b795d04d0a7924395e8832f42c` (origin/next at the start, 2026-11-26
10:28Z, "Regenerate train-owned views"; commit time 2026-11-26T08:48:25Z, epoch 1795682905). First-parent log since
25.2 (5a0acbb35): headers-enum-followups-2 (674b648b8), headers-net-sys-names-3 (88b76d217), release-snapshot-1125b
(db602e58c), headers-nonnull-match (818a8ec83), headers-gettext-clang (c74d35d7e), headers-stage2-gaps-c-3
(dc2a0f8e2), nl-locale-name-abi-4 (23b91a0a2), headers-stage2-gaps-b-4 (9f73af04d), review-20261126-1 (6d7c91f36),
headers-stage2-gaps-d-6 (d3bb2362a), affected-layout-rows-2 (6778b0b80), headers-stage2-gaps-e-5 (0323a4a9d),
plants-products-once (da67b9a02), headers-features-forms-5 (6df2e02ef), plus train view regenerations. The three
required merges are in; HEADERS-FRAGMENTS is NOT in (no byte-identity check owed). No 2026-11-26 snapshot staged on
keke, so **serial 1**: system `2.39+niji20261126.1-1`, coexist `0.0~snapshot20261126.1+cd31515dc8be`. Branch
`task/release-snapshot-1126a` = head + handoff/ only. Role keke `~/w/n1` (Zen 1: products, .debs, gate, plants,
staging); root checks, header probes and package builds on mei (Skylake-SP, Ubuntu glibc 2.39 = stock headers).

**Job dirs.** keke and mei `~/work/RELEASE-SNAPSHOT-1126A/` = 1125B's scripts through `copy.sh` + `subst.sed` (local
scratch copies in `~/w/.scratch/RELEASE-SNAPSHOT-1126A/`): head, epoch, serial 1, role n3 -> n1, labels
`release-snapshot-1126a-*`, comparison side 25.1 -> 25.2 (1125B's debs, products, logs; `-ref252`; p226 -> this head
p227). Copy defect avoided: only `-ref251` is renamed; `y23p/ref241.txt` kept its name (copy.sh's check printed it).
tree.tgz = `git archive cd31515dc` (sha checked on both hosts). Plants file: handoff/RELEASE-SNAPSHOT-1126A-plants.txt
(1125B's six, ids rs1126a-*, each pattern present once in this head's files).
New on mei: `hp4.sh` (every item of Yohane's T1240Z, T1310Z, T1320Z reports + perl's rows + the known instances,
code-shaped probes, stock 2.39 vs coexist AND system flavour, gcc/g++, 4 modes), `fp.sh` (repo probes
feature_probe.py use / forms (--pairs from stock) and header_probe.py --names on the rows GAPS-D, GAPS-E,
FEATURES-FORMS, NL-LOCALE-NAME added; stock vs both flavours), `ybuild.sh` + `ybuild/` (step 6: npth gnupg2
util-linux libev keyutils inetutils, yapt's method, nocheck), `cmp11.sh` (cmp10 with mask widened to 26 / 1126A /
ref252 / p227; y23p's 25.2 side is -1), `fptree/` (tree for fp.sh). `logs-ystage2-stock` is COPIED from 1125B's
(stock side does not depend on libniji; job release-snapshot-1125b-ystage2-stock-3) - say so in the report.

## Running at the end of leg 1 (10:58Z)
* keke: `run-keke.sh` (setsid; out in run-keke.out): smoke-keke-1 DONE (`SMOKE: RESULT bad=0`), then gate-1,
  heapnode-1, debs-1 in sequence, then plants-1 and gaimass-1 + chain-keke-debs.sh (stage-1, install-1), with
  startup-keke-1 / startup215-keke-1 / kstart-keke-1 / rstart-keke-1 beside them.
* mei: `run-ref.sh` (the -ref252 sides), `run-mei.sh` (products RELAYED and sha-checked at 10:57Z; it now runs
  chain-mei-pre etc. and waits for debs.sha), `run-hdr.sh` (waits for debs: hp-1, hp2-1, yapt-1, ypk-1, yh-1, yhc-1,
  ystage2-niji-1, then ystage2cmp-1).
* Done baselines on 25.2's debs (mei): hp4-ref252-2 (`hp4: TOTAL same=172 diff=452`; -1 superseded: flag leak bug),
  fp-ref252-1 (older fp.sh: all names, forms without --pairs; use: 1488 differing lines on 25.2).

## Exact next steps
1. When keke debs-1 ends: relay `~/work/RELEASE-SNAPSHOT-1126A/debs/{system,coexist}/*.deb` and `debs.sha`
   keke -> local -> mei (same layout under W/debs; `cd debs && sha256sum -c ../debs.sha` at each hop). run-mei.sh and
   run-hdr.sh start by themselves.
2. Then on mei (jobrun, detached): `release-snapshot-1126a-hp3-1` (`sh $W/hp3.sh $W/debs/coexist`),
   `-hp4-1` (`sh $W/hp4.sh $W/debs/coexist $W/debs/system`), `-fp-1` (`bash $W/fp.sh $W/fptree $W/debs/coexist
   $W/debs/system $W/logs-fp`), `-ybuild-1` (`sh $W/ybuild.sh`). Step 4 blocking = hp/hp2/hp3/hp4 0 DIFF and fp `use`
   0 differing lines; step 5 = ystage2cmp (perl rc, failing tests, Configure d_ip_mreq_source / d_bsd).
3. After the root checks: `sh cmp11.sh` with both of cmp10's lists (see 1125B's cmp10-a.out / cmp10-b.out headers).
4. keke: pkgdiff, stage, install, gaimass, startup numbers vs 25.2; `dpkg --compare-versions` both flavours gt 25.2.
5. Report (first line COMPLETE: snapshot-ready at <sha> or the blocking findings, steps 4 and 5 first), json,
   precheck/handback-check with JOBS.
