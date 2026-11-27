# RELEASE-SNAPSHOT-1127B: snapshot validation of 16e03637e (serial 2026-11-27.2) - WIP, leg 1 handoff

## State (end of leg 1, ~12:32Z)
* Branch `task/release-snapshot-1127b` = origin/next 16e03637e + handoff/ only. Head under validation:
  `16e03637e78a141164666743d627418749153bdf` "Regenerate train-owned views", commit time 2026-11-27T10:58:36Z
  (epoch 1795777116). First-parent log since 26.3 (0f7aa0180): release-snapshot-1126c (1f8d402ce),
  gcc-fixincludes-probe (9c04be00f), headers-stage2-gaps-h (2dfb5aa3b), regen (68af8ce95), headers-stage2-gaps-i
  (8643d0109), regen (dbb8c346f), release-snapshot-1127a (887d5ab58), headers-stage2-gaps-j (c380b7efe), regen
  (16e03637e). GAPS-J's merge is in.
* Serial **2**: system `2.39+niji20261127.2-1`, coexist `0.0~snapshot20261127.2+16e03637e78a`. Why 2: 27.1 was staged by
  1127A (dbb8c346f, keke `~/release-staging/snapshot-20261127.1/`) and not published; reusing 1 would give one
  version string two different builds. Sorting above 26.3: still to check with `dpkg --compare-versions`.
* Role: **mei `~/w/s1`** (Skylake-SP), not shiki sh1: sync_role refused shiki sh1 (held by HEADERS-STAGE2-GAPS-K,
  rc 3, not forced); coordinator #8802 agreed. libc.so.6 sha256 `1d9e2e6e25c4a536...`, ld.so `20a09f9f187602e7...`.
* Job dirs: mei and keke `~/work/RELEASE-SNAPSHOT-1127B/` = 1127A's via its copy.sh + this run's subst.sed (scratch
  copy `~/w/.scratch/RELEASE-SNAPSHOT-1127B/subst.sed`); no 1127A values left outside comments; data file names kept
  (y23p/ref241.txt). **Copy defect found and fixed**: copy.sh copies regular files only, so 85 symlinks in mei's g14/,
  gi/, x/ (x/r241's `asm/*.h`) were missing; links.sh restored them (12 dangling in both 1127A's and this dir).

## Running (started 12:2xZ; all through jobrun)
* mei drivers (outputs `~/work/RELEASE-SNAPSHOT-1127B/<name>.out`): run-role.sh (smoke-s1-1 DONE bad=0 -> products ->
  debs-1 -> plants-1 (bg, /mnt/zram/claude/release-snapshot-1127b/plants-1) + gate-1 -> heapnode-1), run-mei.sh,
  run-extra.sh, run-hdr.sh, run-hand.sh (new: gi-1, g14-1, cxxp-1, dupval-1 (1127A's 6583-row list), dupval-2 (this
  head's list), yb9-1, yb10-1, yb11-1). All wait for products.sha / debs.sha themselves.
* keke: run-keke2.sh (waits for products.sha / debs.sha in keke's dir; smoke-keke-1, startup/kstart/startup215/
  rstart-keke-1, gaimass-1, units-1, stage-1, install-1, pkgdiff-1, hdrdiff-1). Products already relayed (12:31Z).
* Done: yb11-ref263-2 (yb11 on 26.3's .debs: same=430 diff=470, the before picture; every both-fail row has a real
  stock reason). yb11-ref263-1 SUPERSEDED (probe defects fixed in -2).

## Exact next steps
1. When mei `debs-1` ends (`~/bin/jobwait release-snapshot-1127b-debs-1`): `bash ~/w/.scratch/RELEASE-SNAPSHOT-1127B/relay.sh
   debs` (mei -> local -> keke, sha-checked), then on keke start units2-1 by hand:
   `setsid nohup ~/bin/jobrun release-snapshot-1127b-units2-1 test -- bash ~/work/RELEASE-SNAPSHOT-1127B/units2.sh
   >/dev/null 2>&1 </dev/null &` (step 6 llvm-19: report the first stop; GAPS-J expects `<netipx/ipx.h>`).
2. Wait for the jobs; read verdicts: step 4 yb11-1 (must be diff=0 except recorded 2.41 rows, which compare with 2.41),
   cxxp-1, dupval-1/-2 (0); step 5 gi-1 fixincl 0 rewritten; step 6 gi-1 units + uuid `uuid_time64@@UUID_2.40`, keke
   units-1, units2-1; step 7 ybuild-1 (util-linux AND inetutils PASS), g14-1; step 8 ystage2-niji, yapt, ypk, sweeps.
3. cmp13 / tails on mei (lists: write new `cmp13-a.list` / `cmp13-b.list` in the 1127B dir with 1127A's job sets;
   1127A's lists: `~/work/RELEASE-SNAPSHOT-1127A/cmp13-{a,b}.list`; tails jobs = 1127A tails-1's set: hp hp2 hp3 hp4
   ypk yapt ystage2cmp ystage2-niji startup-mei kstart-mei rstart-mei startup215-mei mint eetwet y13s y25c fp yb6 yb7
   yb8 exp gai6 gai6s yh yhc smoke-mei sig32): jobs `release-snapshot-1127b-cmp13a-1`, `-cmp13b-1`, `-tails-1`
   (`sh cmp13.sh $(cat cmp13-a.list)`).
4. lint: `cargo clippy --workspace --all-targets --all-features -- -D warnings` + `cargo xtask static` on mei s1 (gate-1
   covers both), then precheck on the final head, report json, handback-check.
* Scratch (local, ~/w/.scratch/RELEASE-SNAPSHOT-1127B/): yb11.sh, objval.py, run-role.sh, run-hand.sh, relay.sh,
  links.sh, stockchk.sh, copyrun.sh, tree.tgz.

## Findings so far
* 1127A's yb10 had two vacuous row sets: g++ ftp-tables failed on BOTH sides (C++ brace list narrowing `REC_ESC`'s -1
  to unsigned long), so it proved nothing in C++; yb11 uses long long. yb11's value rows compare per object.
