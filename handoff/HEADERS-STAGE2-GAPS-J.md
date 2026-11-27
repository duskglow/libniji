# HEADERS-STAGE2-GAPS-J: handoff (run 1, end of leg 3; REFRESH)

Branch task/headers-stage2-gaps-j, cut from origin/next dbb8c346f, no merge. HEAD c0b977b7b (code complete if fin2
is green). Role keke:~/w/n1 (not a git tree; no cargo on the agent host: every cargo on keke through jobrun; pull
generated files back with `"${VSSH[@]}" 'cd ~/w/n1 && tar cf - <paths>' | tar xf -` from the worktree).
Decision D-X.179. Job scripts on keke `~/work/headers-stage2-gaps-j/` (p1.sh, gj3.sh, gen4.sh, lint1.sh, dup1.sh,
gate1.sh, pf1.sh, fin1.sh, fin2.sh); scratch helpers in `~/w/.scratch/HEADERS-STAGE2-GAPS-J/`.
Full report so far: handoff/HEADERS-STAGE2-GAPS-J-report.md (update its "RESULTS-FINAL" line, do not rewrite).

## Done (do not redo)
* Items 1-5 coded (legs 1-2). Leg 3: nested_tag_covered generator fix (4dba9aeca), typedefs.tsv order + dup-listed
  shrink (14b1f1b1c), plants file (893568eca), rows-test data + C++ nested spelling (dd26b37f3), __kernel_old_time_t
  visibility rows (c0b977b7b).
* Proof done: probe 0 DIFF both trees C/C++ (gen4), dupval diff=0, whole header_check 0 failed of 26728 (gate1),
  inetutils builds on libniji's system debs (pf1, rc 0/0), sanitizer unit's first further stop = <netipx/ipx.h>
  SIOCAIPXITFCRT/SIOCAIPXPRISLT/SIOCIPXCFGDATA (pf1; report as next stop, follow-up story).
  gate1: niji-abi tests ok, xtask 691/692 (rows test, fixed: fin1 rows test rc 0), headers-gen --check ok.
* plants-1 (893568eca's earlier file): 7 CAUGHT; j-nested-tag WARN-NO-MSG and j-tsptype-table SURVIVED, both
  replanted in 893568eca (see the plants file comments) -> SUPERSEDED by plants-2.

## Exact next step (leg 4)
1. `~/bin/jobwait headers-stage2-gaps-j-fin2` on keke (started 10:00Z; ~15 min, whole header_check inside). Read
   `~/logs/headers-stage2-gaps-j-fin2.log`. Expect: kernel_spelled rc 0 (if kernel-spelled.tsv changes, pull it
   back, headers-gen, commit), no `fin2: changed` lines (if any: pull libc-include back and commit), rows test 0,
   clippy 0, static 0, header_check 0 failed. If static still refuses __kernel_old_time_t, fix the cause there.
2. Start plants on the final synced head: `setsid nohup ~/bin/jobrun headers-stage2-gaps-j-plants2 plants -- sh -c
   '. ~/.cargo/env; cd ~/w/n1 && cargo xtask plants handoff/HEADERS-STAGE2-GAPS-J-plants.txt --dir
   /dev/shm/claude/headers-stage2-gaps-j/plants-2 --timeout 600' </dev/null >/dev/null 2>&1 &` (~18 min). Commit
   its results.txt as the plants file (results appended), push, sync.
3. precheck on the synced final head: `cargo xtask precheck --jobs headers-stage2-gaps-j-gate1,headers-stage2-gaps-j-fin2,headers-stage2-gaps-j-plants2 --head <sha>`
   (as a jobrun job). Gate = FULL escalated (crates/abi, nested-tags.tsv): "FULL: the train's (...)" in the hand-back;
   gate1 ran on 893568eca (crates' tests), fin2 reruns the failing rows test + header_check on the final head.
4. handoff/reports/HEADERS-STAGE2-GAPS-J.json (thread_common shape + "jobs"), handback-check, hand back.
   JOBS: keke: headers-stage2-gaps-j-gen4, -pf1, -gate1, -fin2, -plants2 (+ precheck label). SUPERSEDED: -probe3
   (found the if_ppp compstat defect), -lint1 (static/dup reds, fixed), -dup1, -plants (plants-1, replanted), -fin1
   (static red, fixed), -rows1 (diagnosis).

## Informational for the hand-back
* PPPIOC* literals vs stock's _IOR forms (FORM-DIFF report mode, as MTIOCTOP): follow-up.
* header_check is compile-only: tsptype/ripcmds entries proven by the run probe, not a gate unit.
* Nested-tag generator mutant -> headers-gen stack overflow (loud, not wrong output).
* pf1's deb build ran beside gate1 (separate work tree and target; load ~3 of 60).
* IMPROVEMENT idea: header stories should run `cargo test -p xtask every_table_row...` (rows test) and the whole
  header_check in their first regeneration job: leg 2 ran neither, and both found table gaps late.
