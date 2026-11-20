# REVIEW-LOCALE-SLOTS-GROWTH: independent review of chain LOCALE-SLOTS-GROWTH

Written 2026-11-20 by the independent reviewer (story REVIEW-LOCALE-SLOTS-GROWTH, a planning story: no code changes).
The rabbit-hole tripwire fired on chain LOCALE-SLOTS-GROWTH at 09:59Z (4 reds >= 3; 5 continuations >= 3).

Sources (all read-only): `ops log` for every member (LOCALE-SLOTS-GROWTH .. -6), the first brief and the latest
brief (-6) in full, the other briefs by header, each member's `handoff/reports/*.json` on its branch, `ops/tripwire.py`,
`ops status` / `owes` / `digest`, `state/GOALS.md`, the tail of LOG.md and the STARTUP-DATA-WINDOWS code on next
(`xtask/src/layout.rs`, `xtask/src/binbudget.rs`). No other branch's code or history was read.

## Verdict in one paragraph

The chain is **not a rabbit hole in its content**. The product work (audit row HL-10, owner ruling 11-17: remove the
locale, name, file and mapping caps with fallible growth) was done and measured in the first two runs, about 2.1
agent-hours. Those runs loaded 1100 locales where next stops at 1024, and they report `ENOMEM` instead of `ENOENT`.
The run-1 lookups were linear scans; hash indexes past 64 entries replaced them in run 2. The common case stayed
within 60 instructions. The next three runs (about 2.9 agent-hours, of which 0.4 h was a run killed by a login
expiry) and a 50-hour hold were spent on **start-up layout side effects at zero margin**: page and text-fault counts
moved by one when locale code grew or `.data.rel.ro` changed size. That is the systemic cause that REVIEW-STARTUP-GOT-PAGE
already named, and R2 (LAYOUT-BYTE-BUDGETS-2/-3, landed 06:13Z and 09:10Z today) and LAYOUT-GOT-ORDER (running) now
address it. Also, the tripwire fired one minute after the chain's sixth run was admitted, on lifetime counts. In
the last 24 h the chain had 0 reds and 0 continuations, because it spent that time on a coordinator hold.
**Recommendation: continue with one named change**: decide now how a residual `.got` straddle is treated, the same
way for this chain and THREAD-FORCED-UNWIND, so that run 6 lands instead of producing a run 7 (details below).

## The chain, run by run

| Run | Agent-h | Hand-back | Red reason | What caused it | Its own product? |
|---|---|---|---|---|---|
| 1 | 0.7 | INCOMPLETE (context checkpoint): caps removed, spill chunks, ENOMEM, re-map reuse; 1100-locale test 205 s (linear scans) | verify: incomplete (by design) | scope: three caps, reload rule, archive measurement and two stress tests in a Size M brief | yes (scope) |
| 2 | 1.4 | hash indexes past 64 entries; 1100 loaded 42.6M -> 135k instr | verify: job gate rc=1 (`init_coreutils_shaped_program_stays_within_its_page_fault_bound`, 45 > 44) | ~3.7 KB of new locale-tier text in hot window bin 2 | **yes**, fixed in run 3 (growth paths cold) |
| 3 | 0.7 | fault traced and fixed; jobs queued | verify: job plants rc=1 | two plant gaps on the new out-of-line paths | **yes**, fixed in run 4 (20/20 caught) |
| 4 | 1.25 | INCOMPLETE: plants 20/20; gate red, strings mode text fault 0 -> 1 | verify: job gate rc=1 (`init_first_string_calls_after_printf_take_no_text_fault`, ...) | run 3's layout change moved the strings group; trace not deterministic | partly (layout); the trace needed TRACE-TIME-PIN |
| 5a | 0.4 | none | run failed | the Claude login expired (coordinator requeued it: "not on the story") | **no** (infrastructure) |
| 5b | 0.6 | INCOMPLETE: every trace criterion holds (tool 44 = base, strings 0), but STARTUP-DATA-WINDOWS 8 > 7 | verify: incomplete (by design) | `.got` straddles a page because LTO shortened `.data.rel.ro` by 400 B | no (zero-margin page count, position of `.got`) |
| 6 | running | admitted 09:58Z after a 50 h hold (ruling #6526: wait for STARTUP-GOT-PAGE, which closed without landing at 07:21Z) | - | - | - |

Total 5.05 agent-hours (matches the tripwire's 5.0). Calendar: submitted 11-17 08:47Z; about 50 of the 73 hours since
were holds (TRACE-TIME-PIN for 3 h, STARTUP-GOT-PAGE for 50 h).

## Premises that look false

1. **"4 failure reds" (tripwire).** Of the 4 counted reds, one is not this chain's work. Run 5a's red is
   `run failed: run ended with success`, and the coordinator requeued it at 04:06Z 11-18 as "the run died on ...
   Claude login expiry ..., not on the story". Run 4's red belongs to a hand-back that already said INCOMPLETE
   ("attributed, not fixed"). It counts only because its reason starts `verify: job` and not
   `verify: incomplete`. So the chain has **2 work failures** (run 2 page fault, run 3 plant gaps), each fixed in
   the next run, plus 1 layout red (run 4).
2. **"cost growing" in the last 24 h.** The tripwire's own line says "last 24 h 1 runs, 0 reds, 0.2 agent-h". The
   only run in the window is run 6, which was admitted at 09:58:41Z, and the tripwire fired at 09:59Z. The chain had
   been held by the coordinator from 11-18 04:44Z to 11-20 07:16Z. Nothing grew. The tripwire fires on lifetime
   counts when no gain is recorded in the window, so any chain that waits out a hold trips it as soon as its next run starts.
3. **"10 measured gains ... largest: cost: start-up strings mode text faults ... 0 -> 1 page faults".** The tripwire's
   `is_gain` (`ops/tripwire.py`) counts any measured entry with numeric before != after, whatever its kind or
   direction. Of the 10:
   - 2 are **regressions** reported as costs: run 4's text fault 0 -> 1 and run 5's written pages 7 -> 8. The
     "largest gain" printed in this brief is one of them.
   - 4 perf entries (newlocale or setlocale with 1100 loaded, 42.6M -> 135k and similar) are measured against run
     1's own linear-scan intermediate, not against next. Next cannot load 1100 locales at all.
   - 2 are common-case rows: newlocale of a new locale -1.2%, and setlocale(LC_ALL) of a loaded locale **+56
     instructions** (103637 -> 103693, +0.05%).
   - 2 are coverage entries: tests 1 -> 3, and plants caught 18 -> 20. The second only closes the chain's own plant gaps.

   The chain's **real deliverable** is not counted at all, because those entries have `before: null`: 1100
   directory-form locales load where next fails at 1024, and running out reports `ENOMEM` (newlocale and setlocale)
   where next reports `ENOENT`. Owed #32 (due since 02:42Z) already covers "a regression is not a gain". This chain
   is a live example, and it adds a second defect: correctness gains with a null before are invisible.
4. **Run 6's brief, "Why it keeps going red": "none is its own product cost".** Run 2's red (coreutils-shaped start-up
   45 > 44) was the chain's own product cost (3.7 KB of locale text in hot bin 2), found by the gate and fixed in run 3.
   Run 3's red (plant gaps) was its own as well. The conclusion holds for runs 4 and 5 only. This does not change
   the plan, but the "why" section is a premise and should be accurate.
5. **Run 6's brief has two notes that disagree.** The 11-18 note says "rerun cargo xtask so (STARTUP-DATA-WINDOWS
   must pass)". The 11-20 note says "the R2 byte budgets ... are the per-story gate now". On next,
   STARTUP-DATA-WINDOWS page counts block only with `NIJI_FAULT_BOUNDS=enforce` (`xtask/src/layout.rs`,
   `COUNT_BOUNDS_ENV`, the release ratchet); otherwise an over-budget count prints `over its budget` and does not
   fail. If run 6 still sees 8 > 7 pages (the cause in run 5 was `.got` position, which next did not change), it
   may read the 11-18 note literally and hand back INCOMPLETE again.

## Inconsistent treatment of one cause (the item that decides whether run 6 lands)

The `.got` straddle has one cause: `.got`'s position is `.data.rel.ro` size mod 4096, with 383 B headroom on next
(#6925). Two stories have hit it:
- THREAD-FORCED-UNWIND-3 is **held until LAYOUT-GOT-ORDER lands** (ruling #6925, 08:23Z, "the 8th written page is real").
- LOCALE-SLOTS-GROWTH-6 was **unheld at 07:16Z** onto next as it is. That was before #6925 diagnosed the page, and it
  was not re-held afterwards.

Run 5 measured the dynamic coreutils-shaped probe unchanged (44 = 44). The 8th page is in the static contract
probe, and it moves whenever any change shrinks or grows `.data.rel.ro`, in either direction. LAYOUT-GOT-ORDER (ruling
#6935: relax byte-checked binop relocs so `.got` leaves the non-PIE static image) removes the cause for every branch.

**Recommended rule for both stories:** if the only residue on a verified head is the `.got` straddle, the branch is
verified and finished, and CI holds the *landing* (not a new agent run) until LAYOUT-GOT-ORDER lands. That way no
static program gains a written page, which agrees with #6925 and the INVIOLABLE rule. If LAYOUT-GOT-ORDER closes
without landing, treat the straddle as layout jitter under ruling #4347 (its position depends on `.data.rel.ro` size
mod 4096, which any merge moves), and land both stories with the RC's `NIJI_FAULT_BOUNDS=enforce` as the backstop.
This needs the coordinator or owner to decide, because it sets how R2 and the RC ratchet divide the work.

## Processes due for a rethink

- **The tripwire's firing window (E4).** It fires on lifetime reds and continuations when no gain is recorded in
  24 h. A chain that was held, rather than one that is spinning, trips the moment it is readmitted. Count cost since
  the last unhold, or since the last review, or require at least one red or continuation inside the window. Here that
  would have saved this review (about 25 tool calls).
- **The tripwire's red count.** Exclude reds the coordinator later requeued as infrastructure (event `red -> ready`,
  "requeued by the coordinator"). Treat an INCOMPLETE hand-back whose named job failed like `verify: incomplete`, when
  the hand-back already attributed the failure.
- **Impact entries (E3/E4, owed #32).** They need a direction (gain or cost), so that `is_gain` can skip `kind: cost`,
  and `is_gain` should take a non-null after with a null before as a gain for `correctness` and `coverage`.
  Continuation runs should state their perf baseline as next, not the chain's own earlier head. Otherwise a chain's
  first draft becomes its "before".
- **Holds on a story that may not land.** Run 6 waited 50 h on STARTUP-GOT-PAGE (ruling #6526), and that chain then
  closed without landing because the fix measured worse. The coordinator caught it at 07:16Z. A hold on an
  experimental fix needs a fallback: "unhold if it closes". The dispatcher already re-points holds on continuation,
  and it could also unhold on close.
- **Zero-margin start-up counts as per-story gates (E1).** These counts cost this chain 3 of its 5 continuations.
  R2 (byte budgets per bin, counts as a release metric) is now on next. No new process change is needed beyond
  applying R2 consistently (the next section).

## Cost vs gain

- **Cost:** 6 runs, 5.05 agent-hours, about 73 calendar hours (about 50 of them held), 4 counted reds (2 work
  failures, 1 layout, 1 infrastructure), 2 by-design INCOMPLETE reds.
- **Gain (measured, not delivered: nothing has landed):**
  - correctness: 1100 locales load (next: `ENOENT` at 1024; the reference is ok at 1100);
  - correctness: `ENOMEM` instead of `ENOENT` under RLIMIT_AS;
  - correctness: an identical re-map reuses its mapping;
  - coverage: 3 reload tests and 20 of 20 plants caught;
  - common case: newlocale -1.2%, setlocale +56 instructions (+0.05%).
- **Not measured:** the cost of lookups on next at, say, 1000 loaded locales (inside the old cap) against the
  branch. That is the only comparison that gives the hash index a gain against next and not against run 1.
- About 2.1 agent-hours of product work and about 2.9 of layout friction. The remaining value (an owner-ruled audit
  row: PostgreSQL's `pg_import_system_collations` and test suites that open every locale) costs one verification run
  if run 6 is allowed to land. Stopping now would waste the 5 hours already spent and leave HL-10 open.

## Recommendation

**Continue, with one named change.**
1. Before run 6 hands back, the coordinator tells it, through its inbox, how a residual STARTUP-DATA-WINDOWS 8 > 7
   (`.got` straddle, report-only under R2) is treated, and replaces the 11-18 note "STARTUP-DATA-WINDOWS must pass"
   with that answer. Treat it the same for THREAD-FORCED-UNWIND-3.
2. Run 6 records the common-case rows (newlocale and setlocale with 1 loaded) against next on its final head. The +56
   instructions on setlocale must be within `bench/instr_limits.tsv`, or be reported as a cost.
3. No run 7 for layout. If run 6's only residue is the `.got` straddle, the story lands or waits for
   LAYOUT-GOT-ORDER as ruled, without another agent run.
4. Tripwire fixes: owed #32 plus the three items under "Processes due for a rethink".
