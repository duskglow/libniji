COMPLETE: validated head 23b9dfdb588080239fd28b1266c0742ae939eba3: every functional check green, dynamic C++ and apt pass; one finding (F1) on a release check, RULING NEEDED before the tag

# RELEASE-SNAPSHOT-CXX: validation of the head carrying DL-LIBSTDCXX, assets staged

**State:** branch `task/release-snapshot-cxx`, cut from origin/next `0c562e2d6`. Validated head (the commit to tag)
`23b9dfdb5` "Drop the single high bit from the dlopen unknown-mode sentence" (the owner's 11-15 plan wording fix in
plan/ph14/dl-16-dlopen.md; wording only); only handoff/ commits on top (plants file, this note, the report json).
Role keke `~/w/n1`; job scripts and logs in keke `~/work/RELEASE-SNAPSHOT-CXX/` (FINAL's debs.sh, validate.sh,
gate.sh, chain-role.sh, smoke.sh with labels, head, epoch and serial changed; stage.sh now calls
tools/deb/stage-release.sh); mei `~/work/RELEASE-SNAPSHOT-CXX/tree` (git archive of `23b9dfdb5` plus keke's release
products, same libc.so.6 sha256 6885a175...).

## Step 1: head

origin/next at start: `0c562e2d6`. `git log --oneline bade54e07..0c562e2d6`: 102 commits (list in this run's
scratch). Product code changed since `snapshot-2026-11-15`: DL-LIBSTDCXX(-2..-4) (STB_GNU_UNIQUE by scope order at
start, dlopen's growing unique table), DL-LD-PRELOAD(-2,-3) (LD_PRELOAD and /etc/ld.so.preload at start),
UTIL-GETCONF / UTIL-GETENT(-2) (getconf and getent, shipped in libc-bin), LAYOUT-WINDOW-BINS-7 (start-up fault bounds
tightened), SL-HEADERS slices 2-4 (abi-gen); tools/deb: root-system.sh shared-libstdc++ programs, swap-cmds.sh apt
and fakeroot cases, stage-release.sh (RELEASE-ASSET-NAMES).

## Step 2: reruns on 23b9dfdb5 (keke unless noted) vs RELEASE-SNAPSHOT-FINAL

| check | job | result | vs FINAL |
|---|---|---|---|
| system build | debs-1 | 6 packages, 2.39+liella20261116.1-1; libc-bin 70088 -> 269672 bytes (adds ./usr/bin/, getconf, getent; libc6 and libc6-dev file lists identical to FINAL) | getconf/getent new |
| check-system | debs-1 | **FAIL, 2 problems: `libc-bin: path not in the system layout: ./usr/bin/` and `./usr/bin/getent`** (F1) | **differs** (FINAL PASS) |
| check-symbols | debs-1 | `PASS (4135 symbol lines, 10 libraries)`, the four dpkg-shlibdeps samples as in FINAL | same |
| coexist build + check | debs-1 | `check: PASS (2 packages)` | same |
| packaged ldconfig | debs-1 | static-pie, stripped (0 .symtab/.debug sections) | same |
| empty root | roots-1 | `root: RESULT pass=15 fail=0`: FINAL's 13 plus **iostream-so (`iostream: 42`) and throwmain (`caught: throwlib: value 3 (throwlib)`) against the shared libstdc++.so.6** | +2 new C++ passes |
| swapped trixie root | roots-1 | `swap: RESULT same=52 differ=0`: FINAL's 47 plus **apt-1 (apt-get check), apt-2, apt-3 (apt-cache policy)** and fakeroot-1/-2 (DL-LD-PRELOAD-3); root's libc.so.6/loader are the package's; cache 55 libs (stock 65) | +5 new passes; both caches +1 (fakeroot), gap 10 as before |
| lintian --pedantic, trixie and excalibur | lint-1 | **7 E / 48 W** each; E set identical to FINAL; 3 new W: `no-manual-page [usr/bin/getconf]`, `no-manual-page [usr/bin/getent]`, `shared-library-lacks-prerequisites [usr/bin/getconf]` (ldconfig carries the same two W already) | +3 W (F2) |
| coexist proof.sh | lint-1 | trixie pass=21 fail=0 wait=0 known=0; excalibur pass=21 fail=0 wait=0 known=0 | same |
| lua_reload, cxx_dlclose | smoke-keke-1, mei smoke-mei-1 | ok 1/1 both sides, 0 Liella-only failures, keke and mei (same libc.so.6) | same |
| gate (role) | gate-1 | cargo clippy --all-features -D warnings, xtask clippy, static, records check: green; direct tier `--base origin/next`: 0 suites selected (a plan .md change), peak 1.7G | - |
| plants | plants-2 | **4 CAUGHT**, 0 survived, 0 WARN-RC-ONLY/NO-MSG (WARN-DEV-SHM only): postinst-configure-ldconfig, postrm-purge-cache, cxx-needs-libstdcxx, swap-not-done; the two check-system plants set aside (F1) | 4 of FINAL's 6 |

## Findings

* **F1 (RULING NEEDED: blocking or not).** `tools/deb/check-system.sh` fails on this head's system packages:
  its libc-bin layout list allows `./usr/bin/getconf` but not the directory `./usr/bin/` nor `./usr/bin/getent`,
  which tools/deb/system.sh ships since UTIL-GETENT. Neither UTIL-GETCONF nor UTIL-GETENT built a package (both
  handoffs say NOT VERIFIED), so the check was never run on them. The packages themselves are as system.sh intends
  and every functional check passes (empty root, swap, lintian E set, proofs, smoke). Not fixed here: the brief
  forbids packaging changes in this run. Fix: add `./usr/bin/` to the directory list and `libc-bin:./usr/bin/getent`
  to the file list (check-system.sh lines 91 and 101), then rerun check-system and the two plants set aside.
  **Recommend: not blocking for the assets** (they are correct), but fix the check first and tag the head that
  carries the fix (that head's .debs get a new liella6 version and epoch, so restage with LIELLA_DEB_SERIAL=2); or
  tag 23b9dfdb5 with F1 named in the notes if the snapshot must go out now.
* **F2 (INFORMATIONAL).** lintian +3 W from the new libc-bin tools (above); same kinds as ldconfig's existing W.
* Plants: release-snapshot-cxx-plants-1 aborted at its baseline (rc 2) because of F1 (baseline-1 is
  check-system.sh); plants-2 reruns the four plants whose baselines pass; rsc-ldconfig-unstripped and
  rsc-postrm-not-packaged are commented out in handoff/RELEASE-SNAPSHOT-CXX-plants.txt until F1 is fixed.
  I declared rsc-cxx-needs-libstdcxx equivalent (thinking the root's shared libstdc++.so.6 would let the mutant
  run); that was wrong: the mutant does not link (libstdc++.so needs libgcc_s's unwinder, which the static recipe
  does not link) and the tool reported it CAUGHT. The plants file carries the correction; a rerun should use
  FINAL's `msg:` again.

## Step 3: staged assets (not published, no tag)

keke `~/release-staging/snapshot-20261116/` (`stage-1`, tools/deb/stage-release.sh: GitHub-safe names, `~` -> `.`):
8 .debs, SHA256SUMS over the staged names (`sha256sum -c` OK), MANIFEST (private) with head `23b9dfdb5...`.
SOURCE_DATE_EPOCH 1794796180 (the head commit's time), LIELLA_DEB_SERIAL=1 (nothing for 20261116 was staged).

| package | version | arch | bytes | asset |
|---|---|---|---|---|
| libc6 | 2.39+liella20261116.1-1 | amd64 | 2639336 | libc6_2.39+liella20261116.1-1_amd64.deb |
| libc-bin | 2.39+liella20261116.1-1 | amd64 | 269672 | libc-bin_2.39+liella20261116.1-1_amd64.deb |
| libc6-dev | 2.39+liella20261116.1-1 | amd64 | 2952620 | libc6-dev_2.39+liella20261116.1-1_amd64.deb |
| libc-dev-bin | 2.39+liella20261116.1-1 | amd64 | 1960 | libc-dev-bin_2.39+liella20261116.1-1_amd64.deb |
| locales | 2.39+liella20261116.1-1 | all | 1932 | locales_2.39+liella20261116.1-1_all.deb |
| libc-l10n | 2.39+liella20261116.1-1 | all | 1916 | libc-l10n_2.39+liella20261116.1-1_all.deb |
| liella6 | 0.0~snapshot20261116+23b9dfdb5880 | amd64 | 2626528 | liella6_0.0.snapshot20261116+23b9dfdb5880_amd64.deb |
| liella6-dev | 0.0~snapshot20261116+23b9dfdb5880 | amd64 | 2951120 | liella6-dev_0.0.snapshot20261116+23b9dfdb5880_amd64.deb |

Release-notes facts that changed: libc-bin now holds getconf and getent (269672 bytes); dynamically linked C++
programs against the distribution's libstdc++.so.6 and apt run with Liella as the system C library; lintian 7 E /
48 W per suite. Unchanged: the swap into trixie needs `--force-depends`; dpkg leaves `/var/cache/ldconfig` behind.

## Step 5 (documentation currency)

The brief points to a step 5 "at the END of this brief", but the brief has no such step (it ends with the standard
Developer flow, Park and resume and Hand-back sections). Not done; reported in the hand-back.

## Jobs

JOBS (rc 0, validated head): keke `release-snapshot-cxx-gate-1`, `-plants-2`, `-roots-1`, `-lint-1`,
`-smoke-keke-1`, `-stage-1`; mei `release-snapshot-cxx-smoke-mei-1`. Evidence, not in JOBS:
`release-snapshot-cxx-debs-1` (rc 1 = F1's check-system FAIL; its system and coexist builds, check-symbols and
coexist check are rc 0, and its .debs are the ones validated and staged). Superseded: `-plants-1` (rc 2, baseline
aborted by F1). Rule slip (mine): one local heredoc (`python3 - <<'X'`) to edit the plants file; no effect.

## Next

Owner/coordinator: rule on F1. If not blocking: land this branch, tag `23b9dfdb5`, publish
`~/release-staging/snapshot-20261116/`. If blocking: a small story fixes check-system.sh (two list entries), then
rerun debs/check-system and the two set-aside plants, restage with serial 2. Do not redo: the C++/apt validation.
Open from earlier: UTIL-LDCONFIG's duplicate builtin-path message.
