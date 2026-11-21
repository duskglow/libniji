# REVIEW-HEAP-ADDRESS-SPACE: independent review of chain HEAP-ADDRESS-SPACE

Reviewer: REVIEW-HEAP-ADDRESS-SPACE (2026-11-20, opus). No stake in the work reviewed. Sources: `ops log` of the eight
members, the ops `runs` and `escalations` rows (read-only), the first brief and the latest brief in full, each member's
`handoff/reports/<id>.json`, run 7's `handoff/HEAP-ADDRESS-SPACE.md`, and `origin/next` at `d9c350f22`
(`crates/core/tests/hot_text_products.rs`, `xtask/src/layout.rs`). No code was changed or run.

Chain: GitHub issue #21 (Yohane Linux: malloc reserves far more address space than the reference, VmSize +20 MiB at
start, freed space not reused; findutils' `many-dir-entries-vs-OOM` fails under `ulimit -v`).

## Verdict in one paragraph

Not a rabbit hole in the heap work itself: the heap content converged by run 6 (growth with use, cold
`grow_segment`, issue probe peak address space 67652 -> 41296 KiB, first malloc 16452 -> 4420 KiB). Most of the cost
since run 2 went into start-up layout counts at zero margin (the same cause the SYSTEMIC alert named and that owner
ruling R2 / D-X.136 has since made report-only for a story), plus a layout change the chain took on that is not heap
work. Two things are wrong now. First, the running run 8 is following a brief that tells it to chase a count that no
longer gates. Second, the chain as it stands would not close #21: by its own numbers the findutils case still fails,
and the part that would close it (the spill, HEAP-SPILL-COLD) was removed in run 5 to satisfy those same counts.
**Recommendation: continue with named changes** (below). This is not a rethink.

## The chain, run by run

| run | agent-h | calls | ended as | what the run spent itself on |
|---|---|---|---|---|
| 1 | 0.80 | 100 | INCOMPLETE (context) | heap content: lazy segments, nearby-class reuse; probe 67652 -> 34888 KiB; 4 rt tests red |
| 2 | 1.61 | 82 | INCOMPLETE (context) | fixed the 4 tests; issue test written; **blocked by STARTUP-DATA-WINDOWS 8 > 7** (.got straddle) |
| 3 | 0.54 | 103 | red: plants2 rc=1 | **fixed the 8 > 7 by aligning `.got` to 256 in `entry.S`** (pending record D-X "GOT alignment"); own plant survivor |
| 4 | 0.30 | 95 | red: gate | own reds fixed; **coreutils fault bound** red (spill kept off short programs for +1 fault) |
| 5 | 1.10 | 97 | red: fullhot rc=101 | **reshaped the heap for start-up**: cold `grow_segment`, spill removed, SEG_FIRST 16; full-profile phase fault red |
| 6a | 0.01 | 0 | run failed | login expiry on the coordinator host, requeued; not the story's cost |
| 6b | 2.93 | 172 | INCOMPLETE | rulings #6513/#6514 applied; fullhot phase fault fixed; RLIMIT_AS need placement-dependent (ruling #6566) |
| 7 | 3.95 | 133 | red: gate (wall time) | #6566 applied; then **hunted a +1 dynamic start-up fault (90 > 89)** to the wall-time limit |
| 8 | 1.34+ | live | running | brief: find the +1 fault, "fix the cause, never the bound" |

Totals (ended runs plus run 8 so far): 12.6 agent-h, 782 calls, 60.3 M tokens, USD 32.3 for runs 1-7. Held 38 h
(11-18 17:35Z to 11-20 07:16Z) on STARTUP-GOT-PAGE-4, which closed without landing; queued 3 h after that.

Estimate, not measured per call: of the 11.2 agent-h of ended runs, at least about 5 h (run 3, most of run 4, about
half of runs 5 and 6b, most of run 7) went on start-up page or fault counts, not on the address space.

## Premises that look false

1. **Run 8's task is a count that no longer gates.** HEAP-ADDRESS-SPACE-8's brief: "start with run 7's 'Exact next
   step' ... (the +1 dynamic start-up fault, 90 > 89; fix the cause, never the bound)". On `origin/next` since
   LAYOUT-BYTE-BUDGETS-2 landed (11-20 06:13Z), `init_dynamic_startup_only_program_stays_within_its_page_fault_bound`
   goes through `fault_bound()` (`hot_text_products.rs:715`): "a runtime fault bound is a release metric, not the
   per-story gate ... prints a `FAULT-COUNT REPORT` line and the test goes on; with `NIJI_FAULT_BOUNDS=enforce` it
   fails" (D-X.136). The same holds for the STARTUP-DATA-WINDOWS page count (`xtask/src/layout.rs:896`). Run 8 merged
   that next at 10:23Z (`d7667973f`), so the test only prints on its branch. The coordinator told the sibling run
   LOCALE-SLOTS-GROWTH-6 exactly this (#6949/#6950, 10:20Z), 48 s after run 8 was admitted (10:19:55Z). Run 8 has
   had no inbox note. Its only coordinator note (07:16Z) says to measure on next and that "Yohane #21 is the goal".
   Not all fault checks became report-only: the zero-fault phase checks (e.g. `hot_text_products.rs:588`, `:604`)
   still `assert!`. Run 5's full-profile red was one of those, so run 8 must still keep them green.
2. **Run 8's "Why it keeps going red" names the wrong cause.** It says run 7's red is a "different cause from runs
   1-6 (placement)". The shared cause is start-up layout counts at zero margin: run 2's blocker (STARTUP-DATA-WINDOWS 8
   > 7), run 4's red (coreutils fault bound), run 5's red (full-profile phase fault) and run 7's red (dynamic 90 > 89).
   That is 3 of the 4 failure reds and the reason runs 2 and 3 checkpointed. The SYSTEMIC alert counted this chain
   among four stories red on the dynamic bound. Only run 3's plant survivor and run 6's placement spread were heap
   problems.
3. **"13 measured gains" overstates what was delivered.** Nothing has landed, so the delivered gain is zero. The
   distinct measured gains against next are about five: issue probe peak and RLIMIT_AS need 67652 -> 41296 KiB; VmSize
   at the first malloc 16452 -> 4420 KiB; 6 -> 3 mmap calls in three start-up modes; heap bin 0 -296 B; coverage
   (plants and tests). The others are double counts or not gains:
   * run 2 repeats run 1's 67652 -> 34888 twice;
   * run 3's "8 -> 7 pages" undoes the branch's own +1 page, which is net zero against next;
   * run 3 repeats the probe number unmeasured;
   * run 1's 34888 was given back in run 5, when the spill was removed for start-up reasons (now 41296).

   This is the same tripwire item REVIEW-LOCALE-SLOTS-GROWTH raised (owed #32). It is not raised again here.
4. **The original brief's proof was never attempted.** HEAP-ADDRESS-SPACE's "## Proof" asks for "findutils' case
   reproduced: `find` over 200000 entries under the issue's `ulimit -v` formula succeeds" and for "Malloc benches and
   start-up counters before and after". Run 7's handoff still lists "cbench_malloc / cbench_startup counters" and
   "findutils `find` under the issue's `ulimit -v` formula" as open after eight runs. By the chain's own numbers the
   find case still fails. Ruling #6513 measured the RLIMIT_AS need at 42000 KiB against the formula's 35152 KiB
   headroom, and it is now 41296 with a fixed layout or up to 43088 with ASLR. **Estimate, not measured:** landing
   this chain improves #21 but does not close it for Yohane. The malloc hot path (P2) has no before/after count on
   the branch, although refill and growth changed. Under INVIOLABLE that count is needed before landing.

## Scope that crept in (an item that decides how run 8 should finish)

The heap branch carries two layout-system changes that are not about address space:
* the start object's `.got` aligned to 256 in `csys/src/machine/entry.S`, with pending record
  `records/pending/HEAP-ADDRESS-SPACE-3-2.toml` (D-X: GOT alignment);
* run 6's window-bin planner change, with pending record `-6-1.toml`, which amends D-X.103.

The `.got` alignment was added in run 3 only to turn STARTUP-DATA-WINDOWS 8 back into 7. That count is now report-only.
The straddle it worked around has a rule (#6949: a head whose only residue is the straddle is finished, and CI holds
it until LAYOUT-GOT-ORDER lands) and a story addressing it (LAYOUT-GOT-ORDER, running). Run 7's handoff names this
alignment as the lead for its +1 dynamic fault. STARTUP-GOT-PAGE showed that `.got` alignment can cost dynamic
programs faults (#6916: +1/+2/+2). Under D-X.138 `.got` is now a byte-budgeted data bin, so alignment padding is
budget, not free. **Not verified:** whether removing it keeps `cargo xtask so` green on run 8's head.

## Processes due for a rethink

* **A rule given to one live run reaches only that run.** #6949/#6950 was worded as one rule for both stories it
  named, and it went to one inbox. Run 8, which carries the same straddle and the same fault-bound frame, was
  admitted 48 s earlier and was missed. A ruling that changes a gate's meaning (D-X.136) should go to every live and
  queued story whose brief cites the old gate. A grep of the briefs for "page_fault_bound", "STARTUP-DATA-WINDOWS"
  and "never the bound" would find them.
* **Continuation briefs carry the old frame forward.** Run 8's brief was drafted on 11-18 (before R2) and run 8 was
  admitted on 11-20 with only a short note added. The 11-18 instruction stayed in force. Owed #34 (mark carried
  numbers as to-recheck) covers numbers but not rules. Re-deriving "how the last run ended" against current next at
  admission would cover both.
* **Context checkpoints, not budget, ended runs 1-3.** Those runs stopped at 82-103 of 150 calls, on 40-44 KB briefs
  and 6.6-10 M tokens each. Every checkpoint became a new story, branch and brief to re-read. The later refresh legs
  (runs 6 and 7) are the better shape. Nothing more to add here.
* **The follow-up is held behind the chain.** HEAP-SPILL-COLD (the part of #21 that closes the refill and realloc
  gap) has been held since 11-17 "until HEAP-ADDRESS-SPACE-5 lands". It was re-pointed three times and now waits on
  run 8, so Yohane's acceptance case is two landings away.

## Cost vs gain

* Cost: 12.6 agent-h, 9 runs (1 an infrastructure failure), 4 failure reds, 7 continuations, USD 32.3 and 60 M tokens
  for runs 1-7, and 38 h held. In the last 24 h the chain did nothing but start run 8, so the tripwire's "0 gains in
  the last 24 h" reflects the hold, not churn.
* Measured gain on the branch, not delivered: issue #21 probe peak address-space growth 67652 -> 41296 KiB (the
  reference grows by about 29800 KiB), VmSize at the first malloc 16452 -> 4420 KiB, start-up mmaps 6 -> 3 in three
  modes, plants and regression tests. All are measured by the runs. None has landed.
* Gain still absent: the findutils case and the malloc bench counts, both in the original proof.
* Reading: the heap part is worth landing, and its cost is reasonable for a start-up-path change (runs 1, 5 and 6, about
  5 agent-h). The rest was the layout frame, which R2 has since changed for every story.

## Recommendation: continue, with these changes

1. **Now, through run 8's inbox** (it is live until 14:19Z): the R2 rule. The whole-process fault bound and
   STARTUP-DATA-WINDOWS print a FAULT-COUNT REPORT and do not gate. Run 8 should measure the +1 dynamic fault and record
   it as a cost (impact `cost`), not hunt it. The zero-fault phase checks still gate.
2. In the same note: drop the `entry.S` `.got` 256-alignment and its pending record `-3-2` from the heap branch,
   unless a measurement on run 8's head shows the heap change needs it. If removing it leaves only the straddle, #6949
   applies (finished; CI holds it for LAYOUT-GOT-ORDER). Run 6's planner change (`-6-1`) stays only if its own plants
   and records stand on their own; otherwise it gets its own story.
3. Before run 8 hands back COMPLETE: malloc bench and start-up instruction counts before and after (INVIOLABLE), and
   the findutils `find` run under the issue's formula, measured and stated whether it passes or fails.
4. Land run 8 as a partial fix for #21. Keep the issue open and tell Yohane the measured numbers and the remaining gap.
5. Unhold HEAP-SPILL-COLD with `Stacks-on: HEAP-ADDRESS-SPACE-8` as soon as run 8 hands back verified, not when it
   lands. Its acceptance is the findutils case under the issue's formula (the issue's real test). Its brief should
   state that page counts are report-only (D-X.136), so the spill is designed for the address space, with cold code
   and byte budgets. Its handoff already plans a recheck of SEG_FIRST 16 (ruling #6514: 4 MiB at the first malloc,
   chosen to keep growth out of start-up traces); that recheck stays.
