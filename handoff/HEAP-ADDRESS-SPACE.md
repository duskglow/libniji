# HEAP-ADDRESS-SPACE (GitHub issue #21): handoff

State: branch `task/heap-address-space-7` (continues -6 .. -1; origin/next `de4cdfc8d` merged), head = this
handoff's commit. Status: see "Run -7 results". Code commits on -7: `c7f02c3f6` (drops -6's gap-top placement
`a84abbe23`), `6591abefd` (issue #21 RLIMIT_AS check under a fixed layout at 41296 KiB, ruling #6566). Earlier runs:
-6 `8afd17494` (window-bin planner, phase-only groups out of the head bin), `ce6328854` (bounds at the measured
levels, ruling #6513; SEG_FIRST 16, ruling #6514), `ad420da74` (stale mallocdbg difference dropped); -5 `7c6a4078b`.
Build role keke `~/w/n3` (Zen 1); scripts in keke `~/work/HEAP-ADDRESS-SPACE-7/` (`m1.sh`: need bisections fixed
and randomised, with and without the placement; `c1.sh`: gate, xtask test, fullhot; `base.sh`: merge-base products).
Spec: `~/state/briefs/HEAP-ADDRESS-SPACE.md` "## The change"; rulings #6513, #6514, #6566.
Decisions in force: D-3.19, D-3.14, D-X.103, ruling #6325; pending records `records/pending/HEAP-ADDRESS-SPACE-3-1.toml`
(D-3: growth with use), `-3-2.toml` (D-X: GOT alignment), `-6-1.toml` (D-X: amends D-X.103, head-bin rule).

## Done in run -7 (do not redo)

- Step 0 on the inherited head: clippy and static clean (`heap-address-space-7-s0`).
- Merged origin/next (clean); `cargo xtask precheck` now exists.
- Issue #21 RLIMIT_AS need, 16 KiB bisections on the merged head (Zen 1 host, `heap-address-space-7-m1`):
  fixed layout (`setarch -R`) 41296 in 10/10; randomised 41296 (8x), 41312..41616 (6x), 42032..43552 (6x) of 20.
  With -6's placement disabled: fixed 41296 in 5/5; randomised 41296 (2x), 41376..42192 (8x) of 10.
- Ruling #6566 applied: the limited run sets `ADDR_NO_RANDOMIZE` in the test thread's persona (restored after),
  AS_HEADROOM 42000 -> 41296; the randomised spread is in the comment. Per-step bounds still run randomised.
- `a84abbe23` dropped: under the fixed layout it changes nothing (same need, so no deterministic test can catch a
  fault in it) and randomised it did not improve the worst case. HEAP-SPILL-COLD may bring placement back with
  its randomised check (the numbers above are its baseline).
- Plants (handoff/HEAP-ADDRESS-SPACE-7-plants.txt): run 1 `heap-address-space-7-plants` rc 1 (as-room-fallback
  CAUGHT; as-plain-fallback SURVIVED on next's own `Pages::map` fallback, not reachable by the issue test, dropped
  with the reason in the file; seg-first-17 WARN-NO-MSG: caught by a per-step bound, not "steps failed"). Run 2
  `heap-address-space-7-plants2` (the committed file, from `~/work/HEAP-ADDRESS-SPACE-7/plants2.txt`) was running
  at the refresh: expect seg-first-17 WARN-NO-MSG again (its msg could name `the heap added|steps failed`).
- Layout bins vs the merge base (`bins.sh`; base products built in `~/work/HEAP-ADDRESS-SPACE-7/base`, origin/next
  `de4cdfc8d`): bin 0 +24 bytes (refill +32, first_cache -50, take_held -5), bins 2/6 smaller, the rest equal.

## Run -7 results (head `6591abefd` + handoff commits)

- `heap-address-space-7-gate1` (direct tier): 6819 passed, 1 failed, peak 12G:
  `hot_text_products init_dynamic_startup_only_program_stays_within_its_page_fault_bound`: 90 page faults > 89.
  Merge base (same test, base products, `heap-address-space-7-dynbase`): 89 in 3/3. HS-6's gate1 (pre-merge base,
  `ce6328854`) passed this test. So the +1 comes from this story combined with the merged next: a start-up cost,
  ALWAYS FORWARD applies (the heap is start-up code; not LAYOUT-JITTER).
- `heap-address-space-7-xtest1` stopped while queued (superseded by the fix). `heap-address-space-7-fullhot1`
  (full profile so + hot_text_products, started by `c1.sh` after the gate) was queued at the refresh: its dynamic
  count in the full profile is a second data point.

- Rule slips in run -7 leg 1 (no effect on results): the plants start had a `mkdir` chained before `setsid nohup`
  (the ssh session stayed attached; the local ssh was ended by PID, the job ran on); one `sed -i` on this handoff
  whose substitution replaced a line with itself (no change; Edit only from here on).

## Run -7 leg 2 (wall-time checkpoint, 16:47Z of a 16:53Z deadline: no code change)

- `heap-address-space-7-fullhot1` was ended (kill-label) before it finished building: it was on the superseded head
  and held the role. `heap-address-space-7-plants2` was left running in its own copy (old head, superseded too once
  the fault fix lands).
- libc.so.6 head (`~/w/n3`, products 15:04) vs base: text +0x840 bytes moves .rodata and the RW segment up one page
  (0x344000 -> 0x345000, 0x42b000 -> 0x42c000); .data.rel.ro 0xac50 -> 0xab38; .got (0x9b0 bytes, both cross one
  page boundary) at page offset 0xdf0 (base) vs 0xcd8 (head); .bss 0xfb8 -> 0xfe0 (+40, heap state). RELRO 0xc000
  in both. Not yet compared: the dynamic test program itself and the faulting pages (mincore / per-page fault list).

## Exact next step

Find the dynamic start-up program's extra fault on the head vs the base (89, base products ready in
`~/work/HEAP-ADDRESS-SPACE-7/base/target/niji`). Lead: this story's `csys/src/machine/entry.S` empty `.got` aligned
to 256 (record `-3-2`, for static links) sits in every link's start object, also the dynamic program and maybe
libc.so; next has had GOT-page work since (startup-got-page-3). Compare the two builds' dynamic `page_faults` program
and libc.so (`readelf -lS`: .got/.got.plt/relro/.data placement, page crossings) and the faulting pages, then fix the
cause (no bound change). Then: sync, clippy + static, gate, `cargo xtask test`, fullhot, plants (the committed file),
`cargo xtask precheck --jobs ... --head <sha>`, report json, handback-check.

## Next steps for the story, in order

1. HEAP-SPILL-COLD: the spill as a cold extension of grow_segment; tighten refill/realloc/AS_HEADROOM to 33092 /
   35144 / 35152, recheck a smaller SEG_FIRST; a placement-independent need (keep the heap's other mappings out
   of a segment's growth window under a limit: a 67104-byte start-up mapping, not NORESERVE, sat inside a later
   segment's window in -6's strace; and stop the retry of a 4 KiB plus 32 MiB plus 4.25 MiB mapping on every
   allocation once growth has failed near the limit), then the randomised limited check (ruling #6566).
2. Still open: relro by-section comparison (ruling #6325 step 1); cbench_malloc / cbench_startup counters;
   returning empty segments' address space; findutils `find` under the issue's `ulimit -v` formula.
