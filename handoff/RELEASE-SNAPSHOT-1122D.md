WIP (leg 1 of RELEASE-SNAPSHOT-1122D; jobs running unattended on keke and mei)

# RELEASE-SNAPSHOT-1122D: snapshot validation of 8e1837914 (HEAP-SECOND-NODE-MAP #21, THREAD-EXIT-NO-UNWINDER #25)

**State.** Head under validation `8e1837914bb99339e2ea4da6dd2893382c1c9782` (origin/next at the start, 23:06 UTC,
"Regenerate train-owned views"; commit time 2026-11-22T22:18:37Z, epoch 1795385917). Contains 2786f94e5
(heap-second-node-map) and d258cd270 (thread-exit-no-unwinder). Commit date 2026-11-22; 22.3 was serial 3, so this is
**serial 4**: `NIJI_DEB_SERIAL=4`, `2.39+niji20261122.4-1` (coexist `0.0~snapshot20261122.4+8e1837914bb9`; the
`dpkg --compare-versions` check against 22.3's niji6 is still to do). Branch `task/release-snapshot-1122d` = head +
handoff/ only. Role **keke `~/w/n4` (Zen 1)**; mei = Skylake-SP (products and .debs relayed from keke through the
local machine, sha256 checked on arrival). 22.3 = 2a76723f2 (RELEASE-SNAPSHOT-1122C; its .debs in mei and keke
`~/work/RELEASE-SNAPSHOT-1122C/debs/system`). Nothing to GitHub, no tag.

**Landed since 22.3** (`git log --oneline --first-parent 2a76723f2..8e1837914`): f1a02c7a0 release-snapshot-1122c,
fd973fc44 sl-headers-s06-residue, 3230573ce dl-process-objects-3, views, ab62095bf ldso-run-program-2, views,
621a75e43 dl-reader-costs, views, c344cce34 headers-null-type, 84e8f1972 stdio-caller-alloc, views, 87ddb01e8
locale-charmaps-gen, views, b69d50e78 libc-init-array-refs, 6251d16c9 sl-headers-s18-3, 2786f94e5
heap-second-node-map, views, d258cd270 thread-exit-no-unwinder, 4382ecae5 ldso-program-options-rest-2, 8e1837914 views.

**Procedure.** 1122C's scripts copied on both hosts to `~/work/RELEASE-SNAPSHOT-1122D/` by 1122C's `copy.sh`
(args `~/work/RELEASE-SNAPSHOT-1122C ~/work/RELEASE-SNAPSHOT-1122D`) with a new `subst.sed` (head, epoch, serial 4,
22.2 -> 22.3 as the comparison side, `-ref222` -> `-ref223`, p220 -> p221 (head) / p219 -> p220 (22.3), role
`w/s2` -> `w/n4`, labels). tree.tgz = `git archive 8e1837914` (sha256 3aa98be9...). Plants file
`handoff/RELEASE-SNAPSHOT-1122D-plants.txt` (1122C's six; each pattern checked present once in the head).
* keke `run-keke.sh` (new, 1122C's run-mei.sh role part with hosts swapped): smoke-keke-1 (build) -> products.tgz
  (sha256 6248f7f7...) -> Zen 1 startup/startup215/kstart/rstart (background) -> gate-1 -> heapnode-1 -> debs-1 ->
  debs.sha -> plants-1 (dir /dev/shm/claude/release-snapshot-1122d/plants-1) + gaimass-1 + chain-keke-debs.sh
  (pkgdiff-1 vs staged 22.3, then stage-1, install-1). Output in `run-keke.out`.
* mei `run-mei.sh` (new): waits for products.sha / debs.sha (until-loops), then chain-mei-pre.sh (smoke-mei,
  startup-mei, kstart-mei), startup215-mei, rstart-mei, sig32, y20, dpo600; after the .debs: chain-mei-debs.sh
  (roots lint yohane many dyn22 y10 y19 y26 y26t y23 c19 y25), y24 y28 y6 y25s yres y23p dlc y27 y13 y14 y6c y14l,
  y14r -> eetwet, y25n, y25-ref223, y25n-ref223, y13s/drp/y6r/y13v/y21 each + -ref223, mint (prev = 1122C's logs-mint).
  Started separately: y14l-ref223-1 (done: pass 12 fail 0, 22.3's .debs).
* New checks (mei): **y25n** (`y25n.sh`, `y25n/`: fu.cc built `-static-libgcc` (fug) and `-static-libgcc
  -static-libstdc++` (fus, no libgcc_s loaded), plain-C `pc.c` thread calling pthread_exit; post.sh judges pc/fug
  = stock bytes+rc, fus = rc 134 + FATAL message + no dtor line). **y25c** (`y25c.sh`, `y25c/in.sh`): coexist
  niji6 + niji6-dev installed in a trixie root, fu.cc built with **niji-g++** (links libstdc++ and libgcc_eh
  statically, tools/deb/niji-gcc:16,71,82) and with g++ as control; rows exit-dtor/exit-fu/cancel-dtor. NOT
  STARTED YET: start it after the .debs reach mei.

## Results so far

| check | job | result |
|---|---|---|
| smoke Zen 1 (build) | keke smoke-keke-1 | `SMOKE: RESULT bad=0` |
| smoke Skylake-SP | mei smoke-mei-1 | `SMOKE: RESULT bad=0` |
| gate | keke gate-1 | `GATE bad=0`, peak 1.8G (clippy all-features, xtask clippy, static, records check, direct tier) |
| #14 on 22.3 | mei y14l-ref223-1 | pass 12 fail 0 |

Start-up, Skylake-SP (mei startup-mei-1 / rstart-mei-1, user instructions / faults; p220 = 22.3, p221 = head):
empty main 260,857/51 -> 253,852/51 (-7,005); 3 libraries 371,146/66 -> 361,533/66 (-9,613); clock_gettime
274,330/54 -> 267,329/54 (-7,001); res0 374,316/56 -> 366,330/56 (-7,986); glibc 107,752/49, 143,094/65,
109,862/50, 169,020/55 (equal to 1122C's record). Kernel/syscall columns: read the KCOUNT lines of kstart/rstart.

## Docs currency (step 7, done in leg 1)

`doccheck.py` (scratch) over README, RELEASE-CRITERIA, CONTRIBUTING, AI_POLICY, SECURITY, docs/*.md (31 files):
223 links, 265 repo paths, 173 `cargo xtask` names: 2 misses, both benign (`cargo xtask help` is xtask's default
command, main.rs:158; `csys/include/x86_64/` in docs/arch-syscalls.md:47 is named as a hypothetical). Findings:
* **D1 (stale, non-blocking):** docs/install-packages.md:150 says `locales` is a placeholder (documentation only);
  LOCALE-CHARMAPS-GEN now ships generated charmaps in it (confirm from pkgdiff-1's file list).
* **D2 (missing, non-blocking):** no docs sentence for #21's heap address-space behaviour (HEAP-SECOND-NODE-MAP).
* **D3 (to confirm with y25c):** docs/install-packages.md:70 documents niji-g++ as linking libstdc++ and the
  unwinder statically; D-22.23 (differences row at docs/differences-from-glibc.md:189) makes a thread exit with C++
  frames abort with a FATAL message whose fix ("link libgcc_s dynamically") niji-g++ users cannot apply.
* docs/RELEASE-NOTES.md "Unreleased" holds only COMPAT-19; THREAD-EXIT-NO-UNWINDER has its differences row and
  COMPAT-38; ld.so options have row 339. STDIO-CALLER-ALLOC matches the reference (no row needed).

## Exact next step (leg 2)

1. keke: `~/bin/jobwait --max 560 release-snapshot-1122d-heapnode-1 release-snapshot-1122d-debs-1` (heapnode starts
   after gate-1). Read `=== DEBS bad=` and check-symbols PASS lines.
2. Relay .debs: scp keke `work/RELEASE-SNAPSHOT-1122D/debs.sha` and `debs/system/*.deb`, `debs/coexist/*.deb` to
   local scratch `relay/debs/`, `sha256sum -c`, then to mei `work/RELEASE-SNAPSHOT-1122D/debs/{system,coexist}/` and
   `debs.sha` LAST (run-mei.sh starts the debs part when `sha256sum -c ../debs.sha` passes). Then start
   `release-snapshot-1122d-y25c-1` on mei (`setsid nohup ~/bin/jobrun ... test -- sh $W/y25c.sh ... &`).
3. dpkg --compare-versions of the coexist niji6 version against 22.3's (`0.0~snapshot20261122.3+2a76723f2ed4`).
4. Wait for everything (keke: plants-1, gaimass-1, pkgdiff-1, stage-1, install-1, startup*-keke-1; mei: all labels
   above), compare with 1122C: `cmp4.sh` (masked verdict lines, -1 logs vs 1122C's) and last verdict line of each
   other job. Known rc 1 by design on 22.3 too: roots, yohane, y14, y14r, drp, drp-ref223 (SUPERSEDED list).
5. Tables: start-up both classes (keke startup/kstart/rstart/startup215 logs), #21 (y21 vs y21-ref223, stock in
   pre), #25 (y25, y25-ref223, y25n, y25n-ref223, y25c), #14 delta (y14l vs y14l-ref223: step 4(a) line, now with
   generated charmaps: is the UTF-8 charmap error gone, only the en_US source missing?).
6. Staged list from keke `~/release-staging/snapshot-20261122.4/` (MANIFEST, SHA256SUMS).
7. Final report here (first line `COMPLETE: snapshot-ready at 8e1837914` or blocking findings), reports json,
   push, sync role, `cargo xtask precheck --jobs <labels> --head <sha>` on keke n4, handback-check, hand-back.
   Precheck caveat (1122C): jobs outside the role tree read "no tested-content record"; keke/mei split.
