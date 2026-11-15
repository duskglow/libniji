# LAYOUT-WINDOW-BINS handoff (after run 4, LAYOUT-WINDOW-BINS-4: slice 2 still open, NOT landable yet)

**State:** branch `task/layout-window-bins-4` (LAYOUT-WINDOW-BINS-3's branch continued; base origin/next 7288d847a), head:
see `handoff/reports/LAYOUT-WINDOW-BINS-4.json`. Role keke:~/w/r5 (Zen 1). Design: `plan/layout-budget-rethink.md`
section 3, **D-X.103**; slice 1 **D-X.108**; slice 2 record pending: `records/pending/LAYOUT-WINDOW-BINS-3-1.toml`
(its body still says NOT YET IN FORCE; rewrite it with the final numbers). Ruling #5784: three slices, one per run.
Run 4 stopped at the ~150k context checkpoint (calls 88/160).

## Slice 2: done (runs 3 and 4)

* Run 3 (unchanged): names `.text.sorted.h9<bin><0|1><place>`, fillers, 64 KiB bin starts, `HEAD_ANCHOR = false`,
  `HEAD_ROOM = 0x400`; clusters and first-window retired; BIN-WINDOW / FIRST-WINDOW tests.
* Run 4, warm-up in the signatures: `tools/hottext/regen.sh` finds the phase modes of page_faults.c (blocks that
  print `f1 - f0`: float mktime rebind strings) and writes each one's trace up to its first `getrusage` as the
  signature mode `<mode>.pre`. `xtask/src/bins.rs`: `WARM_UP`, `counted(sig)` (drops `.pre` modes and a phase mode
  whose warm-up runs the section), groups keyed by the counted signature, start-up (`true`) groups first, a bin's
  modes keep the warm-ups (`join`), `windows()` charges a phase mode 0 in a bin its warm-up runs in, `bound()` 0 for
  phase modes. Unit tests: `a_measured_phase_shares_its_warm_up_windows`, `a_phase_split_by_its_warm_up_is_one_group`,
  `the_start_up_group_takes_the_head_bin_before_wider_groups` (11 + 2 bins:: tests green at a7ca1d604).
* Plants file (not run yet): `handoff/LAYOUT-WINDOW-BINS-4-plants.txt` (run 3's 7 + 6 unit + 1 product plant).

## Measured (keke, `~/work/LAYOUT-WINDOW-BINS-4/counts.sh`, so + hot_text_products --nocapture, both profiles)

counts1 (a7ca1d604, with an "early" step that placed phase groups into a warm bin ahead of their turn): planned
strings/mktime/rebind 0, float 1 (fast) / 2 (release), but float's phase took the head bin before the pure start-up
group: startup-only 13 > 11 (fast), fork 6 > 5 (release); localtime whole 38 / 36 (release 36 fails the bound's
slack: 42 - 4). Dropped the early step (57f28976b). counts2 did not compile (test tuple, fixed in 8190fc757).
**counts3** (label `layout-window-bins-4-counts3`, started 06:13Z on 8190fc757, ~27 min) measures the current design:
read it first (`grep -E "window bins \(applied|page faults|test result|want 0" ~/logs/layout-window-bins-4-counts3.log`).

Offline planner (`~/work/LAYOUT-WINDOW-BINS-4/sim.py <layout-bins.tsv> none [align]`, re-plans from a plan's
sections and full signatures; it reproduces the Rust plan's windows exactly for fast counts1): the current design
plans, both profiles, float 1, fork 2, locale 2, localtime 2, localtime_mktime 2, mktime 0, rebind 0, strings 0,
tool 2, true 1 (sum 12; run 3's plan: 26/27 with phase modes counted whole).

## Open (next run, still slice 2)

1. **Expected red in counts3: `init_zero_fault_calls_hold_at_every_text_phase` at `pre` pads** (counts1 read
   "pre 0x800: rebind 1", "pre 0x2000: mktime 1"). The head bin is unanchored; a pad ahead of the group pushes its
   tail into the next window, which only the tail's own sections touch. The plan puts rebind's (catalog, unlink,
   rmdir) and mktime's/strings' phase sections at the head bin's tail (sim: offsets 0xde99..0xf56b of 0xfc00).
   Candidate (simulated, `sim3.py`): a group whose counted signature holds a phase mode and not `true` prefers a
   non-head bin at equal cost: same windows (sum 11-13 over aligns 1/16/64), rebind leaves the head; three wide
   sections (resolve_fold_slow, liella_sys_close, liella_hot_equal_rest: locale,tool + mktime/strings) stay at
   offset >= 0xde99, which still spills at pad 0x2000 unless the start-up sections reach that window. Rejected by
   simulation: phase groups barred from the head bin (`sim2`: locale/tool 3, release strings 1) and a start-up-only
   head bin (`sim4.py`: locale/tool 3, float 2). Likely needed: order the head bin so start-up sections sit at its
   tail (any window the tail lands in is then faulted by start-up for every mode); `place` is the list position, so
   this needs a head-bin place rule in `placements()` and `position_of` (hottext.rs) kept readable: RULING-free, a
   D-row detail. Check pre 0x800/0x2000/0x8000/0xF000 in the sim before a product build.
2. The overcount: a phase mode's names after f1 (fprintf to stderr, exit path, rebind's unlink/rmdir) count as its
   phase. Harmless so far; a marker after f1 would need a tracer change (trace_gdb.py records first touches only).
3. Then: tighten bounds to the measured levels (LOCALTIME and LOCALTIME_MKTIME: release must drop, 36 < 42 - 4;
   LOCALE 47 and TOOL 50 if they drop, all host classes: only Zen 1 measured so far; the Skylake-SP release level
   ran 1 over Zen 1 in the base), run the plants file (`cargo xtask plants`), the direct gate, `cargo xtask test`,
   the release `hot_text_products` (brief: start-up checks), rewrite the pending record's body.

**Not to redo:** base counts (origin/next 7288d847a, run 3's table: true 11/10, locale 45/45, tool 49/48, fork 5,
localtime 40/41, float 3, strings/mktime/rebind 0); the early-placement design (counts1); variant (i) of the head bin.
**Slice 3 is LAYOUT-WINDOW-BINS-5 (PROOF):** pad sweep (shiki `~/work/LAYOUT-BUDGET-RETHINK/sweep.{sh,py}`), file
sizes (libc.so.6 too), head variant numbers.
