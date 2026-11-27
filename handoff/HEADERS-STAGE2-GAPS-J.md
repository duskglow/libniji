# HEADERS-STAGE2-GAPS-J: handoff (run 1, end of leg 4: hand-back)

Branch task/headers-stage2-gaps-j, cut from origin/next dbb8c346f, no merge. Last code commit c0b977b7b; later
commits are the plants results, this note, the report and its json. Role keke:~/w/n1. Decision D-X.179.
Full report: handoff/HEADERS-STAGE2-GAPS-J-report.md; json: handoff/reports/HEADERS-STAGE2-GAPS-J.json.
Job scripts on keke `~/work/headers-stage2-gaps-j/`.

## State
* Items 1-5 done: <protocols/talkd.h> values, <protocols/rwhod.h> whod::whoent (C++), <scsi/scsi.h> ABORT/NOP,
  ANYADDR (coordinator #8789: as stock), tsptype/ripcmds tables, ax25/netrom/rose/if_ppp/ppp_defs gaps.
* Proof: probe 0 DIFF C/C++ (gen4), dupval 0, inetutils builds on libniji's system debs (pf1); both on 4dba9aeca's
  tree, libc-include unchanged since. Final tree (c0b977b7b): fin2 green (kernel_spelled, headers-gen, rows test,
  clippy, static, header_check 0 failed of 26728); plants-2 9/9 CAUGHT; gate2 (cargo test -p niji-abi, -p xtask)
  and precheck on the final synced head: see the hand-back.

## Open / next
* Train: FULL gate (crates/abi and headers/nested-tags.tsv escalate).
* Follow-up story: <netipx/ipx.h> SIOCAIPXITFCRT, SIOCAIPXPRISLT, SIOCIPXCFGDATA (llvm-19 sanitizer next stop).
* Follow-up: PPPIOC* as _IOR/_IOW forms (FORM-DIFF report mode).

## Do not redo
Everything above. SUPERSEDED jobs: -probe3, -lint1, -dup1, -plants (plants-1), -fin1, -rows1, -gate1 (crates'
tests on 893568eca; rerun as gate2 on the final head).
