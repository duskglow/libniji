# LAYOUT-WINDOW-BINS handoff (after run 7, LAYOUT-WINDOW-BINS-7: slice 3, the bounds at the bins' level; chain done)

**State:** branch `task/layout-window-bins-7` (base origin/next f494ad65d, slices 1 and 2 landed), head and jobs:
`handoff/reports/LAYOUT-WINDOW-BINS-7.json`. Role keke:~/w/r5 (Zen 1). Design: `plan/layout-budget-rethink.md`
section 3, **D-X.103**; slice 1 **D-X.108**; slice 2 **D-X.111**. Ruling #5784 (three slices): all three are now
written; slice 3 changes only `crates/core/tests/hot_text_products.rs` (no product code, no new record: the bounds'
history lives in the constants' doc comments, as before).

## Slice 3 (this run): what is in force

* `TRUE_BOUND`, `LOCALE_BOUND`, `TOOL_BOUND` are per product profile (`[fast, release]`, `profile_index`) like the
  two localtime bounds, and all five go through `check_whole_count` (upper bound, and fail when the count is more
  than `WHOLE_COUNT_SLACK` = 4 under it: the ratchet). Every bound sits at its measured level, no margin.
* Host classes (#5914): the probe binaries built on the Zen 1 role, copied to the Skylake-SP host (mei, same
  launcher: `ADDR_NO_RANDOMIZE`, cwd /, argv0 `page_faults`, `relay <mode>`, `LANG=en_US.UTF-8` only, a
  fixed-length /tmp path), read exactly the counts they read on Zen 1, every mode, both profiles, twice. The
  products do not depend on the build host (LAYOUT-TRACE-HOST-CANONICAL's proof) and carry no role path, so one
  bound per profile holds for both classes. The base's "Skylake-SP one more" (CI 11-15 05:12/05:37 read fast
  localtime 41) predates slice 2; it does not show on f494ad65d.

| mode | bound before | Zen 1 fast / release | Skylake-SP fast / release | bound after (fast / release) |
|---|---|---|---|---|
| start-up only (`true`) | 11 | 11 / 10 | 11 / 10 | 11 / 10 |
| setlocale + printf (`locale`) | 47 | 43 / 43 | 43 / 43 | 43 / 43 |
| tool | 50 | 44 / 45 | 44 / 45 | 44 / 45 |
| localtime | 40 / 40 | 36 / 36 | 36 / 36 | 36 / 36 |
| localtime + mktime | 40 / 40 | 36 / 36 | 36 / 36 | 36 / 36 |
| fork child | 5 | 5 / 5 | 5 / 5 | 5 (unchanged) |
| float | 3 | 3 / 3 | 3 / 3 | 3 (unchanged) |
| strings, mktime, rebind | 0 | 0 / 0 | 0 / 0 | 0 (unchanged) |

Measurement jobs (on f494ad65d, before the change): `layout-window-bins-7-levels1` (keke: both profiles built,
hot_text_products in each, probe binaries kept, counted), `layout-window-bins-7-levels-mei` (mei: the same
binaries), `layout-window-bins-7-padsweep` (keke). Jobs on the code head 0b946a9df (keke:~/w/r5, all rc 0):
`-lint1` (full clippy, static), `-gate1` (direct tier), `-xtest1` (`cargo xtask test`: 6666 + 5247 passed,
conformance 137/137, products step), `-relhot1` (release so + hot_text_products, 26 passed), `-plants1`
(`handoff/LAYOUT-WINDOW-BINS-7-plants.txt`: 7 plants, 7 caught, no WARN).
Scripts: `~/work/LAYOUT-WINDOW-BINS-7/{launch.c,measure.sh,build_measure.sh,padsweep.sh,mei_measure.sh}` on keke
(launch.c, measure.sh, mei_measure.sh also on mei).

## Pad sweep (program-side pads in the start file, as `padded_start` builds them; both profiles, keke)

`.text` and `.data` pads of 0x800, 0x4000, 0x8000, 0xF000: every mode at its level. `pre` (`.text.unlikely`, which
the link puts ahead of the head bin): **+1 in every whole mode** (true, locale, tool, localtime, localtime + mktime)
from 0x4000 in release; in fast from 0x800 (true from 0x4000). fork, float and the 0-fault phases never move. The
head bin is unanchored (`HEAD_ANCHOR = false`, `HEAD_ROOM = 0x400`): a program with more cold code than the room
pushes the head bin's tail over a window boundary. The probe has none, so the bounds hold; real C programs built
with -O2 often have `.text.unlikely` of that size. Sizes on f494ad65d: fast libc.a 14,304,650, libc.so.6 5,938,800;
release libc.a 11,991,952, libc.so.6 5,347,896.

## Next

1. **LAYOUT-BYTE-BUDGETS-2** can start: the bins are in place and the runtime bounds sit at their level per
   profile, the same on both measured classes (its item 4 keeps them as release reports). Its byte budget per bin
   should also cover the head bin's room: the `pre` result above is the one layout input a byte budget of the
   library's own bins does not see (the program's cold text ahead of the group).
2. Head-bin variant (open): anchor the head bin (64 KiB start, costs up to one window of padding) or grow
   `HEAD_ROOM`; measure with the pad sweep above (`pre` 0x800-0xF000) before choosing. Not a bound change.
3. Still open from run 4: a phase mode's names after f1 count as its phase (overcount; needs a tracer marker).

**Not to redo:** the levels above (f494ad65d, both classes, both profiles); base counts of slice 2 (origin/next
7288d847a: true 11/10, locale 45/45, tool 49/48, localtime 40/41); the early-placement design; variant (i) of the
head bin; run 4's simulated head-bin reorderings.
