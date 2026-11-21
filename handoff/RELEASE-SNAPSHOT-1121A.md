WIP (leg 1 done, jobs running): snapshot validation of 71597d2a4 (serial 2, snapshot-2026-11-21.2).

# RELEASE-SNAPSHOT-1121A: snapshot validation of 71597d2a4 (GitHub #21 partial fix, libresolv, dbg)

**State.** Validated head `71597d2a40ac281b2d0cd2bf4e03d17ba160bf88` (origin/next at the start, 05:09 UTC; "Regenerate
train-owned views"; commit time 2026-11-21T04:06:43Z, epoch 1795234003); `git merge-base --is-ancestor 1509aeb3` holds.
Branch `task/release-snapshot-1121a` = that head + handoff/ only. Role **mei `~/w/s1` (Skylake-SP)**; keke = Zen 1.
21.1 = 15a2d6482 (RELEASE-SNAPSHOT-1120F). Commit date 2026-11-21 = the day's second snapshot: NIJI_DEB_SERIAL=2,
`2.39+niji20261121.2-1`, coexist `0.0~snapshot20261121.2+71597d2a40ac`, stage keke
`~/release-staging/snapshot-20261121.2/`, tag snapshot-2026-11-21.2.

**Landed since 21.1** (`git log --oneline --first-parent 15a2d6482..71597d2a4`): libresolv-own-code (142011daa),
dyn-start-quick-2 (429cdf93f), release-snapshot-1120f (df83365cd, handoff only), sl-headers-s08-6 (a98d5c496),
window-gate-stable (743cc2907), heap-address-space-10 (1509aeb35, #21), sys-dbg-dwarf (e6dfe4332), five train view
regenerations (10f7ac236, 0122a0d8a, e6d3dc76c, 7ac731794, 71597d2a4).

**Scripts** (mei and keke `~/work/RELEASE-SNAPSHOT-1121A/`; local scratch `~/w/.scratch/RELEASE-SNAPSHOT-1121A/`):
copy.sh + subst.sed (1120F's copy.sh, names changed; y6/ y25s/ y28/ added) made the set from 1120F's on mei; tree/ =
`git archive 71597d2a4` (tree.tgz sha256 1d3cd7d3..., checked on mei and keke); copied unchanged to keke (tar).
run-mei.sh = 1120F's run-mei.sh + run-mei2.sh in one chain, -1 labels. Plants file
handoff/RELEASE-SNAPSHOT-1121A-plants.txt (1120F's, ids/paths/role/version only; all 6 patterns present at head).
New this run: y21.sh + y21/ (#21: the issue's probe probe21.c with VmSize/VmRSS per step and a bisected headroom need;
loop21.c malloc-until-NULL under start + 65536 KiB for 5 sizes; find over d3 / d200k with the bisected needs and the
issue's formula), yres.sh + yres/ (libresolv: forked local responder, res_init/res_query/res_search/res_querydomain,
silent server, res_mkquery, dn_comp/dn_expand, ns_*/inet_net_*/p_*/b64_*/dn_count_labels; line-by-line vs stock),
both made by mknew.sh from y28.sh's recipe; rstart.sh (made by mkrstart.sh from kstart.sh: pie0 vs res0 = empty main
needing libresolv.so.2; glibc, p211 = 21.1, p212 = head); run-new.sh (rstart-mei-1 after products, y21-1 and yres-1
after the debs).

**Running (started leg 1, all labels prefixed `release-snapshot-1121a-`):** mei run-mei.sh (out: run-mei.out):
smoke-mei-1 -> gate-1 -> startup-mei-1 -> kstart-mei-1 -> debs-1 -> plants-1, y24-1, y28-1, y6-1, y25s-1, roots-1,
lint-1, yohane-1, many-1, dyn22-1, y10-1, y19-1, y26-1, y26t-1, y23-1, c19-1, y25-1. mei run-new.sh (run-new.out):
rstart-mei-1, y21-1, yres-1. mei: y21-ref211-1, yres-ref211-1 (21.1's debs, logs-y21-ref211 / logs-yres-ref211).

## Exact next steps (leg 2)

1. When mei `~/work/RELEASE-SNAPSHOT-1121A/products.sha` exists: copy products.tgz + products.sha to keke's work dir,
   start `setsid nohup sh ~/work/RELEASE-SNAPSHOT-1121A/chain-keke-pre.sh > .../chain-keke-pre.out 2>&1 </dev/null &`
   (smoke-keke-1, startup-keke-1, kstart-keke-1), then jobrun `release-snapshot-1121a-rstart-keke-1 test -- sh
   ~/work/RELEASE-SNAPSHOT-1121A/rstart.sh /dev/shm/claude/release-snapshot-1121a/rstart-1`.
2. When mei's roots-1 log exists (debs.sha complete): copy debs/system/*.deb, debs/coexist/*.deb, debs.sha to keke
   `~/work/RELEASE-SNAPSHOT-1121A/debs/{system,coexist}/`, start chain-keke-debs.sh detached (pkgdiff-1, stage-1,
   install-1). Then `dpkg --compare-versions` 21.2 gt 21.1 for both flavours.
3. cmp: the copied cmp2.sh compares `1121a-$j-$k` with `1120f-$j-1`; 1120F's labels were roots-3 lint-3 yohane-2
   many-2 dyn22-2 y10-2 y19-2 y26-2 y26t-2 c19-2 y23-2 y25-2 y24-2 y28-1 y6-1 y25s-1: write cmp3.sh with per-job old
   labels (new labels all -1).
4. Docs: run 1120F's `~/w/.scratch/RELEASE-SNAPSHOT-1120F/doccheck.py` on the head, compare with its doccheck.txt (23).
5. dbg: debs-1's DBG line vs 21.1's 17 of 19 (SYS-DBG-DWARF should change it).
6. Report here (1120F's layout + #21 tables + libresolv + rstart), json, precheck on s1, handback-check.

**Leg-2 cautions.** Do not sync the role while plants-1 or gate-1 runs (both use ~/w/s1); sync once at the end for
precheck. Local new-script sources: scratch y21/, yres/, mknew.sh, mkrstart.sh, run-mei.sh, run-new.sh.
