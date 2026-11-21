WIP (leg 1 of RELEASE-SNAPSHOT-1120F; refreshed at ~147k context): snapshot validation of 15a2d6482 running

# RELEASE-SNAPSHOT-1120F: snapshot validation of 15a2d6482 (GitHub #28, #6, #25 static, #23)

**State.** Validated head `15a2d64827445b4f39b3c5dd5d5bbcc9d2a852f5` (origin/next at the start, 01:04 UTC; "Merge
task/release-snapshot-1120e into next"; commit time 00:49:33 UTC, epoch 1795222173); `git merge-base --is-ancestor
1e377e145` holds. Branch `task/release-snapshot-1120f` = that head + handoff/ only. Role **mei `~/w/s3`
(Skylake-SP)**, synced rc 0 (tree 9868681d, clean); keke = Zen 1. 20.5 = 305065b07 (1120E).

**Landed since snapshot-2026-11-20.5** (`git log --oneline 305065b07..15a2d6482`: 64 lines; first-parent merges):
thread-forced-unwind-static-2 (#25 static), libm-so-mvec (#28), iconv-name-match-4 (#6 names), deb-coexist-serial,
headers-gen-s08-hooks-2, stdio-wide-byte-reads-3 (#23), release-snapshot-1120e (handoff only), two train view
regenerations (1e377e145, b77a096b3).

**Scripts** (mei and keke `~/work/RELEASE-SNAPSHOT-1120F/`; local scratch `~/w/.scratch/RELEASE-SNAPSHOT-1120F/`):
`copy.sh` + `subst.sed` (on mei) made the set from 1120E's full set (every input dir incl. y24/); head, epoch,
serial 6 (`20261120.6`), p205/p206, role s3, labels changed; only leftover old value: cmp.sh's mask (expected).
Hosts swapped vs 1120E (role on mei, as 1119B): **run-mei.sh** (started 01:07 detached, output `run-mei.out`):
smoke-mei-1 (builds products in s3) -> products.tgz + products.sha -> gate-1 -> startup-mei-1 -> kstart-mei-1 ->
debs-1 -> debs.sha -> plants-1 (handoff/RELEASE-SNAPSHOT-1120E-plants.txt, dir /mnt/zram/claude/
release-snapshot-1120f/plants-1) + y24-1 + chain-mei-debs.sh (roots, lint, yohane, many, dyn22, y10, y19, y26, y26t,
y23, c19, y25), all labels `release-snapshot-1120f-<x>`. keke (copied, tree.tgz = git archive of the head):
**chain-keke-pre.sh** (needs products.tgz + products.sha copied from mei; smoke-keke-1 TREE 0, startup-keke-1,
kstart-keke-1) and **chain-keke-debs.sh** (needs mei's `debs/` dir + debs.sha copied; pkgdiff-1, then chain-keke.sh:
stage-1 -> install-1 into `~/release-staging/snapshot-20261120.6`). Neither keke chain is started yet.

**New checks** (made by `mk.sh` from y24.sh's recipe; probe dirs y28/ y6/ y25s/):
* `y28.sh` (#28): pre compiles tools/deb/smoke/mvec.c (-O3 -ffast-math) and nomvec.c on the stock root, links with
  plain `gcc ... -lm` stock and swapped; judges link, 0.8415, NEEDED (libmvec.so.1 for mvec, none for nomvec);
  issue one-command form shown (no headers in system libc6-dev: known).
* `y6.sh` (#6): iconvp over y6/names.txt (69 spellings: judged) and y6/cjk.txt (30 CJK names: shown), the iconv
  program for 11 spellings (judged), `iconv -l` (shown). Root now includes `locales` (Debian's charmaps, which
  libniji reads: D-6.8). The first version without locales is `y6.sh.nolocales`.
* `y25s.sh` (#25 static): forced_unwind.cc/.c compiled on stock with forced_unwind_products.rs's flags; linked
  -static (s-cxx, s-plain, s-fexceptions, s-buffers) and dynamic -static-libgcc C++ (judged); the three
  -static-libgcc no-libgcc_s cases shown only (ruling #7118 recorded difference; do not write the reference's text).
* 20.5 reference runs (DEBDIR ~/work/RELEASE-SNAPSHOT-1120E/debs/system, logs `logs-y<n>-ref205`): y28-ref205-1
  rc 1, y6-ref205-1 rc 1 (that one WITHOUT locales: rerun as y6-ref205-2 with the new y6.sh), y25s-ref205-1 running.
  rc 1 is expected for the "before" side, but **read the logs first: the scripts are new, a script bug is possible**.

**Docs so far:** doccheck.py (1120E's) on the head: same 23 problems as 20.5, lines identical. Changed user docs
since 20.5: docs/install-packages.md (libm.so script, coexist serial), docs/differences-from-glibc.md (2 lines),
spec/encoding/iconv-names.md (new). Finding candidate (non-blocking): differences-from-glibc.md "Character sets"
intro still gives snapshot-2026-11-17's count (58 of 1180 open; ISO-8859-15 does not open) - qualified by date, so
stale, not false; no release-notes sentence yet for #28, #6, #25 static, #23, coexist serial (check RELEASE-NOTES).

## Exact next step
1. `~/bin/jobwait` on mei: smoke-mei-1, then copy products.tgz + products.sha mei -> keke (through local scratch),
   start `setsid nohup sh ~/work/RELEASE-SNAPSHOT-1120F/chain-keke-pre.sh > .../chain-keke-pre.out 2>&1 </dev/null &`.
2. Read logs-y28-ref205 / logs-y6-ref205 / logs-y25s-ref205 (`out/post.txt`, `out/pre.txt`); fix a script bug if
   any; start y6-ref205-2 (new y6.sh, 20.5 debs).
3. After debs-1: copy `debs/system`, `debs/coexist` + debs.sha to keke, start chain-keke-debs.sh; start y28-1, y6-1,
   y25s-1 on mei (this head's debs, default args); `dpkg --compare-versions` niji6 20.6 vs 20.5 and 20.4 (step 6).
4. y23 must be 13 of 13 (20.5: 11/2). Compare every masked log with 1120E's (cmp.sh). Start-up tables vs 20.5.
5. precheck on the synced final head with the JOBS list; report json; handback-check; hand back.
