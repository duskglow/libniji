# COMPAT19-LDSO-COST handoff (COMPAT19-LDSO-COST-2: merged with next)

State: branch `task/compat19-ldso-cost-2` (task/compat19-ldso-cost 167e63d3e + a merge of origin/next cd1bbabdb).
Role keke:~/w/n2 (Zen 1 host). Decision **D-27.71** (records/decisions/D-27/D-27.71.toml), unchanged.
Report: handoff/reports/COMPAT19-LDSO-COST-2.json (this run), handoff/reports/COMPAT19-LDSO-COST.json (the first run).

## The change (unchanged since the first run; see D-27.71)
A start dependency found by a search is mapped by the plan its candidate check made (`dlopen::check_into` writes
`Checked { elf, plan, dynm }` into a slot in `link`'s frame; `map::map_planned` maps by it; the link-map object is
built from the same parse). Every unsearched path (DT_NEEDED with `/`, an executable stack the COMPAT-19 tunable
accepted, dlopen, the program) keeps `map::load`. Plants: handoff/COMPAT19-LDSO-COST-plants.txt (4).

## This run: the merge with next (DYN-START-RING-2's view arena and StartMapper, CRT-START-FILES-3)
Conflicts, both sides kept:
* image.rs `open_candidate`: a refused candidate's view goes back through next's `unmap_view` (arena-aware).
* link.rs: the secure-mode preload rejected by file mode uses `unmap_view` and still clears the slot; a checked
  dependency is mapped by `map::map_planned(&mut StartMapper { fd }, ...)` (next's start mapper, which keeps an
  aligned reservation's slack: `map_planned` calls the same `reserve`, so it does too); unchecked paths use
  `map::load` with `StartMapper` as next does.
* hot_text_products.rs `DYN_START_INSTR`: both comments kept; the bounds take the tighter side (this branch's)
  and were re-measured on the merged head (below).
No check next added was dropped: `check_into`'s verdict is `check`'s (the same function), and next's other changes
to these files (the view arena) are kept as they are.

## Re-measured on the merged head
Zen 1 host, `compat19-ldso-cost-2-counts` (`~/work/compat19-ldso-cost-2/kc3.sh`: the first run's kc.sh method, median
of 101, ASLR off; both sets the shipped release products built by `validation/wl_prep.sh` from the two commits; the
three test libraries rebuilt with the host compiler, the same files for every set, so the 3-library row is not
comparable with the first run's table). User instructions / user faults / syscalls:
| program | glibc 2.39 | origin/next cd1bbabdb | merged branch b8387b18b | change |
|---|---|---|---|---|
| empty main | 101,045/48/42 | 740,758/51/28 | 723,608/51/28 | -17,150 |
| 3 libraries | 136,281/64/68 | 983,495/65/58 | 962,974/65/58 | -20,521 |
| clock_gettime | 103,184/50/42 | 762,879/55/28 | 745,750/55/28 | -17,129 |
Faults and syscalls unchanged. Kernel instruction medians (one run of 101 each): 775,454 -> 728,196, 1,133,097 ->
1,134,848, 763,849 -> 764,695 (they move by tens of thousands run to run; no change claimed). What remains of the
first run's gain after the merge: -17.2k per empty main (was -18.8k against 32261597b), -20.5k with three libraries
(was -22.4k; different test libraries). Skylake-SP host: not re-measured (this run's role is on the Zen 1 host; the
first run measured both classes and the change moved them by the same amount, -18,811 / -18,817).
`DYN_START_INSTR`: the tighter side kept (fast 788,100, release 758,100, this branch's). Estimated levels on the merged
head (first run's in-test levels plus next's +340 at the base): about 2% under each, so not tightened further;
`compat19-ldso-cost-2-hot` measures the release level (pending at hand-back; a red there, either side of the bound, is this story's to fix).

## Jobs (keke; labels of the final head)
All on the merged tree b8387b18b (later commits: handoff/ only). `compat19-ldso-cost-2-lint1` (clippy -D warnings
+ `cargo xtask static`) green; `compat19-ldso-cost-2-gate` (direct tier) 927 passed / 0 failed, 12 suites;
`compat19-ldso-cost-2-plants` 4 CAUGHT (handoff/COMPAT19-LDSO-COST-plants.txt); `compat19-ldso-cost-2-counts` (above);
`compat19-ldso-cost-2-nextprod` (next's products for the counts). Sequenced by `~/work/compat19-ldso-cost-2/seq.sh`
(lint1 -> gate -> xtest -> hot -> count; log `~/work/compat19-ldso-cost-2/seq.log`): `compat19-ldso-cost-2-xtest`
(`cargo xtask test`), `compat19-ldso-cost-2-hot` (`NIJI_FULL_PROFILE=1` so + hot_text_products) and
`compat19-ldso-cost-2-count` (kc2.sh: the counts again from the role's own release build) were still running or
queued at hand-back (PENDING JOBS); collect with `ssh keke '~/bin/jobwait <label>'`.
SUPERSEDED: `compat19-ldso-cost-2-lint0` (ended unstarted, below).

## Step 0
The inherited head 167e63d3e differs from 47b89c8bf (precheck GREEN in the first run, clippy and static included) only
in handoff text. Its clippy/static job (`compat19-ldso-cost-2-lint0`) waited 30 min for a slot on a full host and
held the role against the sync; it was ended unstarted and the same checks ran on the merged head
(`compat19-ldso-cost-2-lint1`, green).

## Next step
Land through the train. Open after landing: the c19 16/16 snapshot check on debs of this head (train/RC).
