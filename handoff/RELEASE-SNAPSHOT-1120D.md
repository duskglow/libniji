WIP (leg 1 of RELEASE-SNAPSHOT-1120D; refresh at the context checkpoint, all jobs running detached)

# RELEASE-SNAPSHOT-1120D: snapshot validation of 0e5a87e92 (GitHub #25 forced unwind)

**State.** Validated head `0e5a87e927235a019970cb2f8920562d331ae9a4` "Regenerate train-owned views" (origin/next at the
start, 20:11Z; commit time 19:33:03Z, epoch 1795203183); `git merge-base --is-ancestor 02fd586b8` holds. Branch
`task/release-snapshot-1120d` = that head + handoff/ only. Role keke `~/w/n1` (Zen 1), as in RELEASE-SNAPSHOT-1120B;
mei = Skylake-SP. Landed since 20.3 (`git log --oneline b732d9b46..0e5a87e92`: 119 lines; first-parent merges):
sl-headers-s15-3, **thread-forced-unwind-3** (#25), locale-slots-growth-7, release-snapshot-1120c (handoff),
phase14-workarounds, wl-dynamic-gate, sl-headers-s07-6, train view regenerations. LAYOUT-RO-STABLE not landed.

**Scripts.** mei and keke `~/work/RELEASE-SNAPSHOT-1120D/`, made on mei by `copy.sh` + `subst.sed` (both there) from
1120C's full set (y23/ c19/ added; 1120C's mei-role run-all.sh not copied); role back to keke n1 (smoke-keke-1 builds).
`tree/` = `git archive 0e5a87e92`. New (sequencing): `run-keke.sh` (keke: chain-role [smoke-keke-1, products.tgz,
gate-1] -> startup-keke-1 -> debs-1 -> debs.sha -> pkgdiff-1 -> chain-keke [stage-1, install-1]; log run-keke.out),
`chain-mei-pre.sh` (mei: products.sha check, unpack into tree/, smoke-mei-1, startup-mei-1, kstart-mei-1),
`chain-mei-debs.sh` (mei: debs.sha check, then roots, lint, yohane, many, dyn22, y10, y19, y26, y26t, y23, c19, y25).
New checks: `kstart.sh` (step 5 kernel side: kprobe.sh's method; user + kernel instructions, strace syscalls; glibc,
p203, p204) and `y25.sh` + `y25/` (#25: repro.cc = the issue's reproducer; fu.cc cases exit/cancel x dtor/fu/all,
exit-lock, ptask, async; built with the root's g++ 14 on the stock root, cmp vs stock after the swap).

## Done so far (leg 1)
* smoke-keke-1 rc 0 `SMOKE: RESULT bad=0` (built products in n1); smoke-mei-1 rc 0 bad=0 (libc.so.6 010ceb32...).
* products.tgz (sha c2bfa9d2...) 1286 entries vs 1205 for 20.3: the role's leftover test artefacts (`rust-test/` 14,
  `rust/` 10, `native/` 4, `niji-owner/` 3...), not products (1120B noted the same for 20.1). Copied to mei, sha OK.
* kstart-keke-1 rc 0 (results in its log; e.g. clock_gettime: user 822,341 (20.3) -> 822,385, syscalls 44 both).
* docs (step 4): local doccheck.py (1120C's, in .scratch) on this tree: the same 23 problems as 20.3 (three line
  numbers in differences-from-glibc.md moved by 6). Changed user docs since 20.3: docs/compat-flags.md (COMPAT-32
  getumask, COMPAT-33 bsd_signal, COMPAT-34 MS_* macros) and docs/differences-from-glibc.md (rows: getumask,
  bsd_signal, reserved names, MS_*, <sys/vm86.h>, struct _IO_FILE after <mntent.h>). RELEASE-NOTES.md, README.md and
  tunables.md unchanged: **no release-notes entry and no docs sentence for #25** (INFORMATIONAL, docs story).
* Plants file committed: handoff/RELEASE-SNAPSHOT-1120D-plants.txt (1120C's 6 plants, keke n1).

## Running at refresh (labels release-snapshot-1120d-*)
keke: run-keke.sh chain (gate-1 running at 20:34Z, then startup-keke-1, debs-1 (~35 min), pkgdiff-1, stage-1,
install-1). mei: chain-mei-pre.sh (startup-mei-1, kstart-mei-1, queued for slots), y25-ref203-1 (20.3's debs from
~/work/RELEASE-SNAPSHOT-1120C/debs/system; logs-y25-ref203; queued; it is the "before" side and also proves y25's
stock build).

## Exact next steps
1. keke `~/bin/jobwait release-snapshot-1120d-debs-1` and check `debs.sha` exists; rsync keke
   `~/work/RELEASE-SNAPSHOT-1120D/debs/system`, `debs/coexist`, `debs.sha` -> local scratch -> mei same paths; on mei
   start, bare: `setsid nohup sh /home/claude/work/RELEASE-SNAPSHOT-1120D/chain-mei-debs.sh >
   /home/claude/work/RELEASE-SNAPSHOT-1120D/chain-mei-debs.out 2>&1 < /dev/null &` (nothing chained before it).
2. When keke's run-keke.out says `run-keke: done` (role idle): push, sync the role, start plants on keke:
   `setsid nohup ~/bin/jobrun release-snapshot-1120d-plants-1 plants -- sh -c '. ~/.cargo/env; cd ~/w/n1 && cargo
   xtask plants handoff/RELEASE-SNAPSHOT-1120D-plants.txt --dir /dev/shm/claude/release-snapshot-1120d/plants-1
   --timeout 600' > /dev/null 2>&1 < /dev/null &`, then gate-2 (same gate.sh) on the final head, precheck.
3. mei: after the validations, `sh ~/work/RELEASE-SNAPSHOT-1120D/cmp.sh` (masked comparison with 1120C's logs);
   y25-1 vs y25-ref203-1 for the #25 table; startup tables from startup-*/kstart-* logs (20.3 numbers in 1120C's report).
4. Report (handoff, JSON, hand-back) as RELEASE-SNAPSHOT-1120C's. Note: y23.sh/c19.sh usage comments still say
   "snapshot-2026-11-20.2" (comment only).
