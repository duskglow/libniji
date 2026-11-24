# LOCALE-ADDRESS-SPACE: handoff (run 1, leg 3: one red needs a ruling)

State: branch `task/locale-address-space` (base origin/next `fb12f4d3e`), code head `f9abb46dd` (later commits:
this note and the report only), build role keke `~/w/r1` (Zen 1, 60 vCPU). Brief: `~/state/briefs/LOCALE-ADDRESS-SPACE.md`.
Decision: pending record `records/pending/LOCALE-ADDRESS-SPACE-1.toml` (D-NEW, series D-7; CI assigns the id).
Plants file: `handoff/LOCALE-ADDRESS-SPACE-plants.txt`. Scratch scripts: keke `~/work/locale-address-space/`
(`measure.sh`, `trace.sh`, `trace2.sh` = strace of the 200000-entry walk with and without a limit; outputs of leg 3 in
`/dev/shm/claude/las/t4096`).

## What was done (legs 1 and 2; do not redo)

* `niji::mem::Permanent`, `LocaleView::new_in`, `global_locale::Objects: Permanent`; in `boundary/src/locale.rs` the
  products' page arena (`perm_place`, 16 KiB first chunk doubling to 1 MiB, own lock inside the interning lock; heap
  path with a replacement malloc, D-X.156); unsafe helpers in `tb.rs`.
* Test `heap_meta_products::a_utf8_locale_costs_no_more_address_space_than_the_references`; plants: 2 of 2 CAUGHT.
* find over 3 files, VmSize at exit (KiB): en_US.UTF-8 18512 -> 16480 (+5616 -> +3584 over C; reference +2988);
  C.UTF-8 13472 -> 13488. Left over +596 KiB = dlopen loader state (528 KiB table map): its own story (record).

## Leg 3

* Fix for gate red 1 (`startup-state.list` declares PERM_LOCK/NEXT/END/CHUNK): `init_startup_data_accesses_are_in_the_data1_cluster`
  ok on `f9abb46dd` (job `locale-address-space-t2`).
* Gate red 2, `find_over_200000_entries_bounds_its_address_space` (`FIND_PLACEMENT_SLACK` = 548): still red. All 8
  phases (KiB; need = least `ulimit -v` of the 200000-entry walk, maps = VmSize at exit without a limit):

| phase | base need | base maps | base over | branch need | branch maps | branch over |
|---|---|---|---|---|---|---|
| 0 | 46844 | 46836 | 8 | 45360 | 44840 | 520 |
| 2048 | 46844 | 46836 | 8 | 45360 | 44840 | 520 |
| 4096 | 46844 | 46836 | 8 | 45616 | 44840 | 776 |
| 6144 | 46844 | 46836 | 8 | 45616 | 44840 | 776 |
| 8192 | 45052 | 46836 | 0 | 45360 | 44840 | 520 |
| 10240 | 45052 | 45044 | 8 | 45360 | 44840 | 520 |
| 12288 | 45052 | 45044 | 8 | 45360 | 45352 | 8 |
| 14336 | 45324 | 46836 | 0 | 45360 | 44840 | 520 |

  Base = release-snapshot-1124c's gate on the same host today. Issue #21's own bound (over 3 entries <= 35000) holds:
  33008 to 33264.
* Cause (strace at phase 4096, `trace2.sh`, limit 45616): under the limit the process ENDS with 45616 mapped (not a
  transient peak). The loader's 64 MiB reservation is refused under the limit, so the libraries sit higher and the
  kernel puts the 2.9 MiB locale archive at the top of the gap above the first heap segment: that segment's 5th 2 MiB
  growth step fails (EEXIST). The first segment's 68 KiB head below its aligned address then stops the next
  segment's last 1.75 MiB step, and the walk places a third segment (12.3 + 14.25 + 6.25 MiB against 16 + 16 without
  a limit: 768 KiB more, plus two 4 KiB per-request maps). Without a limit the same walk fits in two segments.
  Phase 8192 (limit 45360, `/dev/shm/claude/las/t8192`): the same mechanism. The first segment fills its 16 MiB slot,
  the second (0x...6000000) stops after 3 growth steps below the archive (mapped at 0x...6a00000), and a third
  segment follows; ends with 45360 mapped against 44840 without a limit.
  Inferred, NOT VERIFIED (no base products on the host to trace): base hit the same placement under the limit, but
  its at-exit mapping without a limit was also three segments (its locale objects took a 2 MiB heap step), so
  the difference hid it. The branch makes the run without a limit more compact; need fell 1228 to 1484 KiB at 6
  phases and rose 308 at 8192/10240 and 36 at 14336 (not explained yet).
* Not changed: the slack (ALWAYS FORWARD). RULING NEEDED (below).

## RULING NEEDED

The test's limit-vs-no-limit allowance (548 = 2 x 256 + 9 x 4) against this branch's 776 at 2 of 8 phases, caused
by placement under the limit (locale archive above the first segment, the first segment's head in the slot below)
that the base hid by mapping one more segment without a limit. Recommend: accept 3 x 256 + 9 x 4 = 804 for this
change with the trace evidence, and brief a heap story to stop a segment's head from taking the last growth step of
the aligned slot below it (worth 256 to 768 KiB under a limit in this walk), after which the slack tightens again.

## Jobs (keke, code head f9abb46dd)

* `locale-address-space-t2` (so + the two reds), `locale-address-space-clippy3` (xtask clippy + static: ok),
  `locale-address-space-gate2`, `-xtest`, `-fullprofile` (chain `~/work/locale-address-space/chain2.sh`, started
  17:41 UTC). The gate is expected red on find_over_200000 only; any other red is new.
* Plants `locale-address-space-plants` (head 1f3dddf4b, arena code unchanged since): 2 of 2 CAUGHT.
* Superseded: `locale-address-space-gate` (head 3b97f3f2b, stopped at 17:28 after the two reds).

## Next step

On the ruling: if accepted, change `FIND_PLACEMENT_SLACK` and its comment (cause above), re-run that test, then
`cargo xtask precheck --jobs <labels> --head <sha>` and handback-check. Also explain the +308 KiB need at phases
8192/10240 (trace2.sh at phase 8192 with limits 45052 and 45360).
