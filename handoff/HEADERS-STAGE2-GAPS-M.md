# HEADERS-STAGE2-GAPS-M: handoff (run 1, leg 2 done: header edits committed; leg 3 next)

Branch task/headers-stage2-gaps-m, cut from origin/next af6f8acaf, no merge. HEAD 70d74b185 (WIP commit of the header
edits). Role shiki:~/w/sh1 (stock glibc 2.39, Ubuntu 2.39-0ubuntu8.9: black-box probes of the reference run there).
No local cargo: run `cargo xtask headers-gen` on the role, then pull the tracked `libc-include/` back with
`rsync -rc --include='*/' --include='*.h' --exclude='*' claude@<shiki addr>:w/sh1/libc-include/ ./libc-include/`
(list first with `rsync -rcn --itemize-changes --exclude target --exclude .git ...:w/sh1/ ./`).
Scratch (local) ~/w/.scratch/HEADERS-STAGE2-GAPS-M/, the same scripts on shiki in ~/work/headers-stage2-gaps-m/,
outputs in /dev/shm/claude/headers-stage2-gaps-m/. `sh ~/work/headers-stage2-gaps-m/cmp.sh <tag>` reruns probe.py and
probe2.py on both sides (stock outputs p-stock2.txt, q-stock2.txt, q-stock2.txt.use cached) and prints per-item DIFF
counts. probe3.py = the bits/types.h typedef probe (r-stock.txt). Hand-back-only notes: notes.txt there (read it).

## Coordinator notes in force
* 22:52Z CORRECTION to #8823 (owner): **B1 is back in.** abort() reached again from a SIGABRT handler ends the
  process with SIGABRT, as #8813 said; a test beside the existing abort tests; a differences row against 2.39
  (docs/differences-from-glibc.md; compat option per CLAUDE.md rule 2 or the reason none). Variants (raise then
  abort, longjmp out then abort): probe 2.41 in a trixie chroot as CONFIGURE-DIFF did (its chroots worked on the
  Skylake-SP host); if one cannot be probed, apply the same rule (a second abort in one process ends it) and say so.
  Leg 1's own mmdebstrap attempt on mei was refused by the classifier (see Refused); do not retry that exact form.
  libniji's abort: crates/rt/src/fatal.rs `abort` (unblocks and re-raises).
* Every row: confirm stock 2.39 differs from libniji before changing it; a row where 2.39 agrees is 2.41-only: list it.

## Done (commit 70d74b185; probe results vs stock 2.39 after it: L3 L4 L6 L8 L9 L13 0 DIFF)
L5 features.h includes gnu/stubs.h; L12 features.h (a) C90 C99 names when MISC or POSIX >= 200112, (b) _REENTRANT/
_THREAD_SAFE raise POSIX < 199506 to 199506 + _POSIX_C_SOURCE 199506L + _POSIX_SOURCE 1 (`__NIJI_POSIX_RT`), (c) GNU mode
with no POSIX/XOPEN macro implies POSIX 2008 (also with _ISOC99_SOURCE/_ISOC11_SOURCE); L13 `#warning`; L3 cdefs;
L4 gnu-versions.h (4 versions; exports checked present in docs/codemap/symbols.tsv); L6 `#define d_fileno d_ino`;
L8 utmpx `__UT_*` via @@value(UT_*); L9 3 rows each in visibility.tsv + enumerators.tsv (FAST_NP gnu+C++ only; not
macros, probed); L11 8 providers.tsv `none` rows; L15 signal.h `@@if SIGTSTP` includes new bits/niji-sigtstp.h (slice
s08 row, p1 0).

## Next steps (in order)
1. L11 still 10 DIFF: math.h did not change after the providers rows (generated math.h identical): find why the
   generator still skips `__exp10*` (prototypes.tsv header is bits/mathcalls.h; check how math.h's emit reads it).
2. L12 remnants (cmp.sh n1): with -std=gnu17/gnu89 + _ISOC99_SOURCE or _ISOC11_SOURCE stock defines
   _POSIX_C_SOURCE 200809L, _POSIX_SOURCE 1, _ATFILE_SOURCE 1, libniji not (the `@@if _POSIX_C_SOURCE` condition
   is level-derived: add the case beside the __NIJI_POSIX_RT block). The c89/gnu89 + _XOPEN_SOURCE=500 NAMES rows
   (snprintf etc. per-name XSI conditions; no C90+X/Open config in ftm-configs.tsv) are pre-existing: REPORT them,
   do not chase. du-*.txt (USE diff) differs on every line by __USE_TIME_BITS64 (accepted, #8776): filter it
   (`sed 's/ __USE_TIME_BITS64=1//'`) before counting.
3. L2: probe.py now compiles L2 rows with -c (12 DIFF lines). Plan from leg 1 still holds: attr_probe.py `r` tests
   in config `gnu` (fortify off) -> wur `1` or `fortify`; generator xtask/src/headers.rs ~line 598 and functions.rs
   redirect_line: wur `fortify` appends ` __NIJI_FWUR` (features.h.in: warn_unused_result when __NIJI_FORTIFY > 0);
   regenerate diagattrs.tsv's wur column on shiki; and match stock's `__wur` on a program's own function (probe
   "L2 own-wur"; libniji's cdefs defines __wur unconditionally).
4. L7: stock's <bits/types.h> shows the same 63 `__` typedefs in every configuration (C89..gnu17, _GNU_SOURCE,
   FOB64, TIME64; C++98, gnu++17): list in scratch bt-names.txt, sizes/signedness in r-stock.txt, all in
   typedefs.tsv. Generator: `__x` names are omitted unless headers/reserved.tsv says emit/emit-once
   (headers.rs `omitted`); typedef emitter xtask/src/headers/typedefs.rs. Needs a way to emit them in bits/types.h
   only (e.g. visibility rows for bits/types.h + a per-header emit), guarded against redefinition beside the
   headers that already emit some (emit-once, `__NIJI_D_` guard, headers.rs `guard`). Report the bits/ list: diff.tsv
   has only bits/types.h; grep the cdiff rerun's config.logs for `#include <bits/`.
5. spec/abi/provenance.d/HEADERS-STAGE2-GAPS-M.txt (order: highest + 10) for the visibility/enumerators rows.
6. header_check fragment spec/tools/header_checks/s13.d/80-stage2-gaps-m.py (units per item; L13 warning check in
   `problems(ctx)` with own compiles); header_check.py base flags must not fail the bsd and svid configurations on
   the new #warning (they compile with -Werror): add `-Wno-cpp` there only for flags defining _BSD_SOURCE or
   _SVID_SOURCE without _DEFAULT_SOURCE (features.py line 139 does the same for the reference).
7. B1 (above). 8. header_check whole, clippy, static, gate, plants (handoff/HEADERS-STAGE2-GAPS-M-plants.txt).
9. Debs: tools/deb/ (build.sh, system.sh); cdiff.sh over the 45 packages (diff.tsv first column) on the Skylake-SP
   host; classify libsigsegv as 2.41-vs-2.39.

## Refused (report in the hand-back)
Leg 1: `mmdebstrap` trixie throw-away chroot on mei to probe 2.41's nested-abort rule (the Bash call that wrote
~/work/headers-stage2-gaps-m/t241.sh and abrt.c to mei and started `~/bin/jobrun headers-stage2-gaps-m-t241 test --
sh ~/work/headers-stage2-gaps-m/t241.sh`): denied by the auto-mode classifier. Not retried.
