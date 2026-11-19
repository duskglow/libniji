WIP (leg 1 of RELEASE-SNAPSHOT-1119B; refresh): snapshot validation of d496d4c25, jobs mostly done, collect and report

# RELEASE-SNAPSHOT-1119B: snapshot validation of d496d4c25

**State.** Validated head `d496d4c25cb16f57cc4ea3390a551b6c3b74a58c` "Merge task/plants-build-slowdown into next"
(origin/next at the start, 2026-11-19 18:34 UTC; commit time 17:20:15 UTC, epoch 1795108815). Branch
`task/release-snapshot-1119b` = that head + handoff/ only. Role mei `~/w/s2` (Skylake-SP); keke = Zen 1. Job scripts:
mei and keke `~/work/RELEASE-SNAPSHOT-1119B/`, made by `copy.sh` + `subst.sed` from `~/work/RELEASE-SNAPSHOT-1119/`
(head, epoch, serial 20261119.2, role s2, labels, staging dir, pkgdiff against snapshot-20261119.1, stage note;
y10/ copied too). tree/ = local `git archive` of d496d4c25. New: `y19.sh` + `y19/` (y10.sh's recipe; #19).
Local scratch: `$S/work/RELEASE-SNAPSHOT-1119B` (doccheck.txt, y19 sources, subst.sed, copy.sh).

**Landed since 19.1** (`git log --oneline fb63eeec8..d496d4c25`: 92 lines; first-parent merges):
release-snapshot-1119 (handoff), headers-gen-complete, gate-dev-no-full, test-scratch-base, headers-tables-complete,
dl-23-2 (includes dl-23), train views, plants-build-slowdown. `merge-base --is-ancestor` origin/task/{dl-23,
dl-23-2,plants-build-slowdown,headers-gen-complete,headers-tables-complete}: all yes.

## Results so far (mei unless noted)

| check | job | result | vs 19.1 |
|---|---|---|---|
| smoke | -smoke-mei-1 (builds), keke -smoke-keke-1 | `SMOKE: RESULT bad=0` both; libc.so.6 896e4a46..., ld.so 2db5ffdb... | same |
| gate | -gate-1 rc 0 | clippy -D warnings, xtask clippy, static, records check, affected: all RC 0 | same |
| system debs | -debs-1 rc 1 | `check: PASS (7 packages, 2.39+niji20261119.2-1)`; DBG 17 of 19 (iconv, locale; ruling #6321) | same |
| check-symbols | debs-1 | PASS 4440 lines (19.1: 4438): ld.so block + `_r_debug@GLIBC_2.2.5 2.2.5` + `(symver)NIJI_PRIVATE` (ld.so exports `_dl_debug_state@@NIJI_PRIVATE`, FUNC 1) | +2, explained |
| coexist | debs-1 | builds; `check: PASS (2 packages)` | same |
| empty root | -roots-1 | `root: RESULT pass=19 fail=0` | same |
| swapped trixie | roots-1 | `verdict: commands=65 same=63 known=2 bad=0` | same |
| dbg trixie/excalibur | roots-1 (rc 1 only from this) | `dbg: RESULT pass=8 fail=1` both (19.1: pass=5 fail=0 pending=2; DL-23 rewrote dbg-system.sh) | FINDING below |
| fresh | roots-1 | `fresh: RESULT fail=0` | same |
| lintian + proofs | -lint-1 rc 0 | proofs trixie/excalibur pass=21 fail=0; E/W counts and normalised tag diff NOT YET compared | todo |
| #16 | -yohane-1 | 16 PASS / 2 FAIL, masked FAIL/RESULT lines identical to 19.1's | same |
| #9 | -many-1 rc 0 | pass=9 fail=0 | same |
| #22 | -dyn22-1 rc 1 | `niji fail=8` (19.1: 9): FINDING below | improved |
| #10/#18 | -y10-1 rc 1 | 19 PASS / 1 FAIL (bare-name dlopen), identical masked lines to 19.1's y10-2 | same |
| #19 | -y19-1 rc 0 | all PASS (read the log for the report table) | fixed (19.1: -y19-ref191-3 fails) |
| docs | local doccheck.py | 23 problems, the same 23 as 19.1 (differences-from-glibc.md lines shift) | same |

Superseded: -y19-ref191-1 / -2 (probe script fixes: gnat1 prints nothing for --version; info sharedlibrary
before the dlopen; gnatmake log prefix; -2 killed). -y19-ref191-3 is the valid 19.1 reference (fails: `undefined
symbol: _r_debug` for gnat1, r-nopie, r-pie; gdb at main lists only ld.so, no breakpoint in libg19.so; gnatmake rc 127).

FINDINGS so far:
* dbg `FAIL gdb in a running /usr/bin/true`: gdb DOES stop (`Breakpoint 1.1, setlocale () at
  boundary/src/posix/s07_locale.rs:18`, `#0 setlocale () at ...:18`, bt to __libc_start_main, info sharedlibrary
  ld.so + libc.so.6); dbg-system.sh line 90 wants `^Breakpoint 1, `; gdb numbered the stop 1.1 (a breakpoint with
  more than one location: `break setlocale` before run = true's PLT 0x2240 plus libc's). Not yet known whether
  stock gives one location. Check-script regex vs product: route to a DL story; not blocking (behaviour shown).
* dyn22: the four `nofile` lines are now classed `start` (19.1: `other`) and `dl_iterate_phdr reports start not
  at all` is gone: the start-time `$ORIGIN` library now has a dlpi_name (19.1: ''). Verify with a grep of
  `dlpi_name`/`start` lines in both logs before stating it.
* docs/RELEASE-NOTES.md:74 Known limitations still says "no debugger rendezvous (`_r_debug`) ... yet"; notes end
  at 2026-11-17. INFORMATIONAL, docs story.
* Headers: libc-include differs from 19.1's tree in 14 files (+added/-removed lines): argp.h +8-8, fmtmsg.h +4-4,
  getopt.h +1-1, mcheck.h +7-7, printf.h +1-1, stdio.h +4-1, stdlib.h +83-3, sys/auxv.h +12-3, sys/epoll.h +4-4,
  sys/pidfd.h +63-1, sys/ptrace.h +37-8, sys/types.h +83-3, time.h +2-2, utmpx.h +8-1 (diff of the two tree/
  archives on mei). Packages ship no headers (check pkgdiff file counts: libc6-dev 38, niji6-dev 48 in 19.1).

## Running at refresh
mei: `release-snapshot-1119b-plants-1` (role s2, dir /mnt/zram/claude/release-snapshot-1119b/plants-1).
keke: `release-snapshot-1119b-pkgdiff-1`, chain-keke.sh (`-stage-1` then `-install-1`). keke debs verified
identical to mei's (sha256, keke ~/work/RELEASE-SNAPSHOT-1119B/debs.sha).

## Exact next step
1. `jobwait` keke pkgdiff-1 stage-1 install-1 and mei plants-1; read summaries (pkgdiff: counts, sizes,
   control fields; stage: `sha256sum -c` OK lines + asset list; install: `inst: RESULT fails=0` and 7 of 7 identical).
2. lintian: compare E/W counts and normalised tag lists (mask build-ids/versions) of logs-lint/lint-*.log with
   ~/work/RELEASE-SNAPSHOT-1119/logs-lint/.
3. Read logs-y19/out/post.txt lines for the #19 table (ldso-r_debug, gnat1, r-nopie, gdb, gnatmake).
4. Optional: dbg 1.1 probe (stock vs libniji `info breakpoints` after `break setlocale` + run in /usr/bin/true).
5. Write the final report here (first line COMPLETE or blocking), handoff/reports/RELEASE-SNAPSHOT-1119B.json,
   commit, push, sync role, `cargo xtask precheck --jobs <labels> --head <sha>` on mei s2, then
   `ops handback-check`. JOBS: mei: smoke-mei-1, gate-1, debs-1, roots-1, lint-1, yohane-1, many-1, dyn22-1,
   y10-1, y19-1, y19-ref191-3, plants-1; keke: smoke-keke-1, pkgdiff-1, stage-1, install-1 (all prefixed
   release-snapshot-1119b-).
Plants file: handoff/RELEASE-SNAPSHOT-1119B-plants.txt (pushed, 8c50b77ee).
