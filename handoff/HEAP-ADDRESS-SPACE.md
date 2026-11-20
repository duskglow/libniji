# HEAP-ADDRESS-SPACE (GitHub issue #21): handoff

State: branch `task/heap-address-space-8` (continues -7 .. -1), origin/next `3f2a84520` merged (clean, `d7667973f`);
head = this handoff's commit. Build role mei `~/w/s4` (Skylake-SP); scripts in mei `~/work/HEAP-ADDRESS-SPACE-8/`
(`m.sh <tree>`: fast products + start-up fault tests + readelf; `c1.sh`: the final-head chain; `base/` = next
`3f2a84520` built, `nogot/` = the head without the 256-aligned `.got`). Spec: `~/state/briefs/HEAP-ADDRESS-SPACE.md`
"## The change"; rulings #6513, #6514, #6566. Decisions in force: D-3.19, D-3.14, D-X.103, ruling #6325; pending
records `records/pending/HEAP-ADDRESS-SPACE-3-1.toml` (D-3: growth with use), `-6-1.toml` (D-X: amends D-X.103,
head-bin rule); `-3-2.toml` (GOT alignment) removed in run -8 with the alignment. Code commits: -8 `10e62ddd2`
(drops the 256-aligned `.got` of entry.S and its so.rs check: entry.S and so.rs are as on the merge base); -7
`c7f02c3f6` (drops -6's gap-top placement), `6591abefd` (issue #21 RLIMIT_AS check under a fixed layout at 41296 KiB);
-6 `8afd17494`, `ce6328854`, `ad420da74`; -5 `7c6a4078b`.

## Run -8: the +1 dynamic start-up fault (run -7's red) is gone after the merge

- Step 0 on the inherited head `90fa33d84`: clippy and static clean (`heap-address-space-8-s0`).
- Merged origin/next `3f2a84520` (403 commits; DYN-START-SPEED-2 and others landed; `DYN_TRUE_BOUND` is now
  [73, 67]). The STARTUP-GOT-PAGE chain closed without landing (#6916), so nothing GOT-related to stack on.
- `init_dynamic_startup_only_program_stays_within_its_page_fault_bound`, fast profile, Skylake-SP, warm-up + 5
  runs, every run identical (`m.sh`): head **70**, next `3f2a84520` **71**, head without the 256-aligned `.got`
  (entry.S `.balign 8`, so.rs check at 8) **70**. Static probes equal in all three: true 11, locale 43.
  So the merged head is 1 below next, and the `.got` alignment does not move the dynamic count (the run -7 lead
  is ruled out). Run -7's 90 > 89 was against the old base `de4cdfc8d`; it does not reproduce on today's next.
- Coordinator note 11:53Z (rulings #6972-#6977): record the dynamic fault as a cost, do not hunt (R2: that bound
  and STARTUP-DATA-WINDOWS are report-only per story); drop the `.got` align-256 unless measured needed; cbench_malloc
  and cbench_startup before/after are required; land as a partial fix for #21 (findutils case -> HEAP-SPILL-COLD).
  The cost is -1 (70 vs 71), not +1. The variant without the alignment has STARTUP-DATA-WINDOWS 7 written pages
  (rw-budget-fast 7, ok) and the same faults, so the alignment was dropped (`10e62ddd2`).
- cbench before (next `3f2a84520`, `~/work/HEAP-ADDRESS-SPACE-8/base`) and after (the head, a copy `headb`):
  `heap-address-space-8-cbench-before` / `-after`, `--compare --counters --runs 3`; numbers in the report.
- Layout bins vs next (`target/product/fast/layout-bins.tsv`): bin 0 (start-up) 60694 -> 60732 bytes (+38),
  105 -> 104 sections: refill +32, `runtime::start` +44, `stdio::parts_first_use` +45, `new_slot` +5,
  first_cache -50, take_held -5, `KernelPages::map` (728) replaced by `map_room` (455). Other bins within +-48 B.
  `hot-text.signatures`: only the `map` -> `map_room` rename. Growth stays cold (`grow_segment`).
- `cargo xtask affected` escalates to FULL (entry.S `_start`; xtask). Gate = precheck + the changed crates'
  tests (`heap-address-space-8-tests`), plus `cargo xtask test` (`-xtest`) and the full-profile hot_text_products
  (`-fullhot`); FULL: the train's.
- Plants: `handoff/HEAP-ADDRESS-SPACE-8-plants.txt`, every plant of runs 3-7 whose code is still on the branch
  (13), on the final head (`heap-address-space-8-plants`). Results: see handoff/HEAP-ADDRESS-SPACE-8-report.md.

## Do not redo

- The +1 dynamic fault hunt (above). The fixed-layout issue #21 check (ruling #6566, run -7). Dropping the
  gap-top placement (run -7). Need bisections (run -7: fixed 41296 10/10; randomised 41296..43552).

## Exact next step

**cbench_startup is slower on the head (INVIOLABLE, blocks landing):** startup_kinstr_c +7.0k..7.9k kernel
instructions (+5.5%, 3 of 3 runs), user-mode rows +6..+28 (report: handoff/HEAP-ADDRESS-SPACE-8-report.md). Next:
1) find which child program the bench runs for startup_kinstr_c (`cargo xtask bench --help`, `bench/` sources) and
whether it calls malloc; 2) run it from `~/work/HEAP-ADDRESS-SPACE-8/base` and `headb` (release bench products
built there) under `strace -f` and a minflt count, before vs after: extra faults, or extra/larger mmap/munmap from
map_room; 3) fix the cause (cold, no hot bytes), recount cbench_startup and cbench_malloc before/after (b.sh/b2.sh,
cmp.sh in the same directory), then rerun lint, `cargo xtask test`, fullhot, plants (HEAP-ADDRESS-SPACE-8-plants.txt),
precheck. Mei role jobs on `10e62ddd2` already green: lint2, fullhot2 (plants2, xtest3: see the report).
Then the story's open follow-ups, in order (each a story of its own size, for the coordinator to cut):
1. HEAP-SPILL-COLD: the spill as a cold extension of grow_segment; tighten refill/realloc/AS_HEADROOM to 33092 /
   35144 / 35152, recheck a smaller SEG_FIRST; a placement-independent need (keep the heap's other mappings out
   of a segment's growth window under a limit: a 67104-byte start-up mapping, not NORESERVE, sat inside a later
   segment's window in -6's strace; and stop the retry of a 4 KiB plus 32 MiB plus 4.25 MiB mapping on every
   allocation once growth has failed near the limit), then the randomised limited check (ruling #6566).
2. Still open: relro by-section comparison (ruling #6325 step 1); cbench_malloc / cbench_startup counters;
   returning empty segments' address space; findutils `find` under the issue's `ulimit -v` formula.
3. The dynamic bound: head 70 on Skylake-SP (next 71); a Zen 1 measurement would show whether DYN_TRUE_BOUND
   [73, 67] can tighten (the fast bound is the higher host class's level).
