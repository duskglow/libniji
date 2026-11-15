# LAYOUT-WINDOW-BINS handoff (after run 6, LAYOUT-WINDOW-BINS-6: slice 2's last plant fixed, handed back for landing)

**State:** branch `task/layout-window-bins-6` (runs 3-5's branch continued; base origin/next 7288d847a), head and
jobs: `handoff/reports/LAYOUT-WINDOW-BINS-6.json`. Role mei:~/w/s1 (run 5: keke:~/w/r5, Zen 1). Design: `plan/layout-budget-rethink.md`
section 3, **D-X.103**; slice 1 **D-X.108**; slice 2's record is pending, `records/pending/LAYOUT-WINDOW-BINS-3-1.toml`
(D-NEW, series D-X; its body now describes what is in force, with the numbers below). Ruling #5784: three slices.

## Slice 2 (runs 3-5): what is in force

* Run 3: bin names `.text.sorted.h9<bin><0|1><place>`, fillers, 64 KiB bin starts, `HEAD_ANCHOR = false`,
  `HEAD_ROOM = 0x400`; clusters and first-window retired; BIN-WINDOW / FIRST-WINDOW product checks.
* Run 4: phase modes' warm-up in the signatures (`regen.sh` writes `<mode>.pre`; `bins::counted`, start-up groups
  first, a phase mode charged 0 in a bin its warm-up runs in).
* Run 5: **group end.** counts3 (run 4's last job) read fast start-up 12 > 11 and the data-phase test red (cold pads
  11/44, others 12/45). A perf page-fault map (`~/work/LAYOUT-WINDOW-BINS-5/diag.sh`, faultmap.py) showed the probe's
  `main` (plain `.text`, right after the last bin) at 0x49feee..0x4a0c38, across a 64 KiB boundary: two faults.
  Fix: every bin has a gap filled to its boundary (the last and the unmeasured bins too), and liella-crt1.o carries
  an empty 64 KiB-aligned `.text.sorted.h9z.liella_group_end` (`bins::GROUP_END`, csys/src/machine/entry.S) that
  sorts after every bin, so the program's text starts on a boundary. `so` refuses a crt1 without it
  (`so.rs check_group_end`); BIN-WINDOW now also checks GROUP-END at every text phase.
* Run 4's open item 1 (phase sections at the unanchored head bin's tail at `pre` pads) did not show: counts3 and
  run 5's counts read 0 for strings/mktime/rebind at all 8 pads, both profiles. Not pursued further.
* Bounds: release `LOCALTIME_BOUND` / `LOCALTIME_MKTIME_BOUND` 42 -> 40, only because `WHOLE_COUNT_SLACK` refuses
  36 under 42 (coordinator: no tightening this run; this is the least that passes). Nothing else changed.

* Run 6: run 5's plants job was red on one survivor, hottext-bin-position (`position_of` reads a bin member's place
  from `&d[3..]` instead of `&d[4..]`). That mutant is equivalent: the guard requires `d[3] == '0'` and a leading 0
  parses the same. The unit test `grouped_names_give_back_their_list_position_and_function` only used place 42, whose
  leading zeros would also hide a dropped digit (a member placed at the wrong offset of its bin); it now also reads
  back place 12345 of bin 12. `handoff/LAYOUT-WINDOW-BINS-6-plants.txt` marks the old plant equivalent and adds
  hottext-bin-digit (`&d[5..]`). No product code changed; the plants, gate and start-up checks reran on the final head.
  Jobs (mei:~/w/s1, base 7288d847a): `layout-window-bins-6-lint1` (full clippy, static, hottext unit tests),
  `-plants` (17: 15 caught, 2 equivalent, 0 survived, no WARN), `-gate` (escalated to FULL, so it ran `cargo xtask
  test` with its products step: 32 suites, 11874 passed, 0 failed, conformance 137/137), `-relhot` (release-profile
  so and hot_text_products, 26 passed). `-xtest` was not run: the escalated gate already runs `cargo xtask test`.

## Measured (keke, Zen 1, `layout-window-bins-5-counts1` on 3137aa64b, every run identical), fast / release

start-up 11 / 10, locale 43 / 43 (base 45 / 45), tool 44 / 44 (base 49 / 48), localtime and localtime + mktime
36 / 36 (base 40 / 41), fork 5, float 3, strings/mktime/rebind 0 at every pad. Planned windows unchanged from run 4.

## Next (slice 3, LAYOUT-WINDOW-BINS-7, PROOF; #5914)

1. Levels of every host class (Skylake-SP read one more than Zen 1 in the base), then tighten TRUE/LOCALE/TOOL and
   both LOCALTIME bounds to them (ALWAYS FORWARD), with the levels CI records (bound tightening with every host
   class's level, #5914).
2. Pad sweep (shiki `~/work/LAYOUT-BUDGET-RETHINK/sweep.{sh,py}`), file sizes (libc.so.6 too; the group end adds
   at most the last bin's residual), head variant numbers.
3. Still open from run 4: a phase mode's names after f1 count as its phase (overcount; needs a tracer marker).

**Not to redo:** base counts (origin/next 7288d847a: true 11/10, locale 45/45, tool 49/48, fork 5, localtime
40/41, float 3, strings/mktime/rebind 0); the early-placement design (run 4 counts1); variant (i) of the head bin;
run 4's simulated head-bin reorderings (not needed: the phases read 0 at every pad).
