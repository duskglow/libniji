# HEADERS-STAGE2-GAPS-M: handoff (run 1, leg 3 done: every item coded; leg 4 = proof and hand-back)

Branch task/headers-stage2-gaps-m, cut from origin/next af6f8acaf, no merge. HEAD acddbdf5c. Role shiki:~/w/sh1
(stock glibc 2.39, Ubuntu 2.39-0ubuntu8.9: black-box probes of the reference run there). No local cargo: run
`cargo xtask headers-gen` on the role, then pull the tracked headers back with
`rsync -rc --include='*/' --include='*.h' --exclude='*' claude@<shiki addr>:w/sh1/libc-include/ ./libc-include/`
(run it again if `git status libc-include` shows nothing: once it raced a running headers-gen).
Scratch (local) ~/w/.scratch/HEADERS-STAGE2-GAPS-M/, the same scripts on shiki in ~/work/headers-stage2-gaps-m/,
outputs in /dev/shm/claude/headers-stage2-gaps-m/. `sh ~/work/headers-stage2-gaps-m/cmp.sh <tag>` reruns probe.py
and probe2.py on both sides and prints per-item DIFF counts; filter `sed 's/ __USE_TIME_BITS64=1//'` from the
.use files before counting USE lines (accepted, #8776). visprobe.py = per-configuration visibility of names on stock.
Hand-back-only notes: notes.txt there (read it: SECURITY-PRIVATE item; B1 now fixes it on libniji's side).

## Coordinator notes in force
* 22:52Z correction to #8823 (owner): B1 is in (done below). Variants raise-then-abort and longjmp-then-abort were
  NOT probed on 2.41 (leg 1's mmdebstrap on mei was refused; not retried): the rule "a second abort in one process
  ends it" applies, said in D-1.15 and in the differences row. Say so in the hand-back.
* Every row: stock 2.39 differs from libniji before changing it; a 2.41-only row is listed, not changed.

## Done (commits 70d74b185, ed0a97156, acddbdf5c)
Leg 2: L3 L4 L5 L6 L8 L9 L13 L15, L11 providers rows, L12 (a)(b)(c). Leg 3:
* L11: reserved.tsv `emit` for __exp10 and 7 siblings (visibility rows existed; probe = same configs as exp10).
* L12 remnant: features.h `__NIJI_POSIX_ISOGNU` (GNU mode, only ISO C macros: _POSIX_C_SOURCE, _POSIX_SOURCE,
  _ATFILE_SOURCE defined) and `__USE_POSIX_IMPLICITLY` widened (ISOGNU, or _REENTRANT raising in GNU mode).
* L2: attr_probe.py `table` reruns the `r` tests in config gnu (wur_off); wur `fortify` (147) / `1` (7);
  headers.rs attr_text + fortify_wur, functions.rs redirect_line append ` __NIJI_FWUR`; features.h.in defines it
  (warn_unused_result when __NIJI_FORTIFY > 0); cdefs `__wur` = __NIJI_FWUR, `__attribute_warn_unused_result__`
  unconditional (stock probe: same). diagattrs.tsv regenerated on shiki (`--columns wur`).
* L7: headers/bits/types.h.in (@@emit), 63 typedef rows + include-error row in visibility.tsv, s05 slice row (p1 0),
  headers-inventory.tsv row now public. Stock's bits/types.h also defines ~57 internal `__*_TYPE` / `_BITS_*_H`
  macros: NOT shipped (reserved names, no package use seen): report. bits/ list for the report: diff.tsv over 438
  packages has only bits/types.h (openldap header check, psmisc struct test) differing; libniji had no bits/ dir,
  so every other bits/ include a probe makes answers alike (stock lacks it too, or not reached). Grep the cdiff
  rerun's config.logs for `#include <bits/` to list the 45 packages' direct includes.
* B1: crates/rt/src/fatal.rs `ABORTED` flag (first abort runs the handler; later ones go to the default action),
  boundary selftest `with_aborting_handler`, test `s1_2_c_4_1_again` (core s01_startup_clauses), record
  records/decisions/D-1/D-1.15.toml (reserved id), differences row (no compat option, reason in the row).
* Probes vs stock after leg 3: L2 L3 L4 L6 L7 L8 L9 L11 L13 0 DIFF; L12 only the pre-existing c89/gnu89 +
  _XOPEN_SOURCE=500 per-name rows (snprintf etc.; no C90+X/Open config in ftm-configs.tsv): REPORT, do not chase.
  USE lines: recheck after leg 3 with the time64 filter (expect 0 besides those).

## Next steps (in order)
1. Read ~/logs/headers-stage2-gaps-m-lint5.log on shiki (records check, clippy, static on acddbdf5c's tree); fix.
2. spec/abi/provenance.d/HEADERS-STAGE2-GAPS-M.txt (order: highest existing + 10) for the visibility/enumerators
   rows added (L9 3+3, L7 64) and diagattrs wur; see an existing fragment for the form.
3. header_check fragment spec/tools/header_checks/s13.d/80-stage2-gaps-m.py (units per item; L13 warning check in
   `problems(ctx)`); header_check.py base flags: `-Wno-cpp` only for flags defining _BSD_SOURCE or _SVID_SOURCE
   without _DEFAULT_SOURCE (features.py line 139 does the same for the reference). Then header_check whole: 0 failed.
4. Plants (docs/plants.md form, handoff/HEADERS-STAGE2-GAPS-M-plants.txt): fatal.rs swap guard (msg
   s1_2_c_4_1_again), features.h.in ISOGNU block, cdefs __wur, attr_text fortify branch (header_check msg).
5. Gate (`cargo xtask affected --run`, label headers-stage2-gaps-m-gate) + plants job; `cargo xtask test` is not
   needed unless a start-up path changed (abort is not start-up).
6. Debs: tools/deb/ (build.sh, system.sh); cdiff.sh over the 45 packages (diff.tsv first column) on the Skylake-SP
   host (mei), RAM run dir /mnt/zram/claude/headers-stage2-gaps-m/; classify libsigsegv as 2.41-vs-2.39.
7. precheck, report json, handback-check, hand-back (with SECURITY-PRIVATE from notes.txt, updated: libniji's
   B1 change makes the re-entering handler end by SIGABRT).

## Refused (report in the hand-back)
Leg 1: `mmdebstrap` trixie throw-away chroot on mei to probe 2.41's nested-abort rule (the Bash call that wrote
~/work/headers-stage2-gaps-m/t241.sh and abrt.c to mei and started `~/bin/jobrun headers-stage2-gaps-m-t241 test --
sh ~/work/headers-stage2-gaps-m/t241.sh`): denied by the auto-mode classifier. Not retried.

## Jobs so far (all on shiki; none is a gate)
headers-stage2-gaps-m-gen3a, -wur3b, -gen3c (probes), -lint3 (red: duplicate test name, fixed), -lint4 (red:
inventory, fixed), -lint5 (running at hand-over).
