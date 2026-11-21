# REVIEW-SL-HEADERS-S11: chain SL-HEADERS-S11 (slice 11, math headers)

Reviewer: independent review run REVIEW-SL-HEADERS-S11, 2026-11-21. No code changes. The whole SL-HEADERS series,
S11 included, was already reviewed in [REVIEW-SL-HEADERS-S07.md](REVIEW-SL-HEADERS-S07.md) on 11-20, when S11 stood
at 6 runs and 11.2 agent-h. That verdict still holds: the cost is integration, not content. It is not repeated
here. This review covers only what has changed since then and what is specific to S11.

## Verdict in one paragraph

**Not a rabbit hole now. Continue as is: let SL-HEADERS-S11-7 land. Do not open a -8 unless CI fails on a cause
inside slice 11.** The content of slice 11 was done and verified on 11-18 (S11-4: plants 23 caught, 0 survivors,
gate green). After that, 64% of the chain's agent-hours went to three re-ports. They were caused by changes that
landed on `next` under the branch, not by the slice itself. S11-7 is the data-only re-port that the series review and
ruling #6748 called for. It handed back green and is queued for CI again.

The tripwire fired on S11-7's CI red, which is not this chain's red. The test that failed, `forced_unwind_products`,
does not belong to the header slice. The coordinator explained the red as one that never reached libniji, and
re-queued the story in the same minute (13:04:51Z) that the tripwire fired (13:04Z). So the trigger itself is the
main finding: the tripwire re-fires on a chain that has already been reviewed, because of a red that was already
explained.

## Premises that look false

1. **"6 reds".** The brief's own grouped red reasons add up to 7, not 6 (verify: job x3, verify: incomplete x1, CI
   CONFLICT x1, manual fail x1, CI RED x1). The count leaves out S11-2's INCOMPLETE-by-design red (ops fix a50b5cd,
   11-20), but the grouping still lists it. Of the 6 that are counted, the S11-7 CI red is not the chain's work: the
   failing test is outside slice 11, and the coordinator's manual finish says libniji was not reached and the
   attribution was heuristic (low confidence). S11-5's red is a "manual fail" by ruling #6597, which changed what the
   slice had to look like, not a defect in it. That leaves **4 reds attributable to the chain**: S11 (the job tested
   content that differed from the hand-back head), S11-3 and S11-6 (one surviving plant each), and S11-4 (a CI merge
   conflict in a shared file that another landing changed). Only the two plant survivors are defects in the slice's
   own proof, and each was fixed in the next run.
2. **"6 measured gains" overstates what will land.** The data-only re-port (S11-7) dropped work that earlier runs had
   reported as measured gains:
   - S11-2, correctness "floating macros tokenize in C89 and C++98": on the S11-7 head, the `values.h` hex floating
     constants (MAXDOUBLE, MAXFLOAT, MINDOUBLE, MINFLOAT) are emitted as hex again. C89 and C++98-14 cannot lex them,
     and `s11.py` asserts them only from C99 and C++17 (S11-7 report, ruling item 3).
   - S11-2/S11-3: `tgmath.h` with its P6 type check was written, then dropped together with `complex.h`.
   - S11-6, tooling "headers-gen finishes with slice 11 (dont_care index)": the generator change is gone (S11-7
     handoff: "a plant on a generator change that is gone").
   - S11-4: 23 plants; the S11-7 head has 18 (handoff) or 17 (report JSON impact). The two records disagree.
   - S11: 577 table rows; the S11-7 head reports 147 macros + 40 layouts + 6 typedefs + 160 visibility rows (353).
     The two figures are not computed the same way, so they cannot be compared directly.
   - Three "measured" entries have `before`/`after` null (S11-2 correctness, S11-3 isinf/isnan/__fpu_control, S11-6
     tooling). They are claims, not measurements.
   What lands with S11-7 (measured by that run): 5 of the slice's 8 headers (math.h, fenv.h, fpu_control.h,
   ieee754.h, values.h), 147 macro rows, 40 ieee754/ieee854 union layouts, 6 typedefs, 160 visibility rows, the
   ieee754 field check and 17-18 plants. The run reported the direct-tier gate as green.
3. **The S11-7 brief pointed its gaps at a story that had already landed.** "name HEADERS-TABLES-COMPLETE, which owns
   those gaps": HEADERS-TABLES-COMPLETE landed on 11-19 at 13:56Z, about 2 h after the brief was written and 2 days
   before the run. The coordinator handled it correctly afterwards with HEADERS-TABLES-COMPLETE-2 (priority 78).
4. **One left-out item has no owner (E5).** The S11-7 handoff lists four left-out items. Item 4 is
   `finclude/x86_64-linux-gnu/math-vector-fortran.h`. The report JSON's rulings name only three, and
   HEADERS-TABLES-COMPLETE-2 covers only those three. Nothing on record now owns the Fortran header.

## Cost vs gain

Agent-hours per run, measured from admission to hand-back in `ops log`. They add up to the brief's 14.5 h.

| Run | h | What it did | How it ended |
|---|---|---|---|
| S11 | 0.66 | tables, generator, 5 templates, Fortran header | verify: stale content (budget ran out) |
| S11-2 | 1.02 | decimal literals, iscanonical triage, tgmath.h | INCOMPLETE by design |
| S11-3 | 2.23 | ieee754.h, P2/P3, bit-field offsets | plants: 1 survivor, 3 without msg |
| S11-4 | 1.34 | close-out: survivor fixed, 23/0 plants, gate green | CI conflict (header_check.py) |
| S11-5 | 3.19 | merge only | manual fail, ruling #6597 (registry layout) |
| S11-6 | 2.73 | re-port into the per-slice registry | plants: 1 survivor (dont_care index) |
| S11-7 | 3.36 | data-only re-port onto the generator (HEADERS-GEN-COMPLETE) | green; CI red outside slice 11, re-queued |

- **Content:** S11 to S11-4, 5.25 h (36%). It converged in 4 runs on a Size M / 110-call budget, which was small for
  8 headers that include math.h (runs 1 and 2 both ended at the budget).
- **Integration and rework:** S11-5 to S11-7, 9.28 h (64%). Three foundation changes landed under a branch that was
  already finished: HANDBACK-INLOOP-CHECKS-2 (header_check.py), ruling #6597 (per-slice registry) and
  HEADERS-GEN-COMPLETE (generator first, landed 11-19 at 09:35Z while S11-6 was running). Each one redid the slice in
  a new shape. This is the integration cost the series review named, and ruling #6748 removes its cause.
- **Elapsed time:** 11-17 18:17Z to now (3.8 days). About 1.9 days of that the chain was parked or held by the series
  park (#6838) and the one-at-a-time rule (#6839).
- **Gain:** measured, not yet delivered (nothing has landed). It is smaller than runs.impact suggests (premise 2).
- **The fix is working:** since the data-only rule, S07-6, S08-6 and S10-5 have each landed as the member after their
  data-only rebase (digest, 11-20 to 11-21). S11-7 is the next one in that queue.

## Processes due for a rethink

1. **Tripwire re-fire (E3, E4).** This review was triggered by a chain that (a) was already reviewed 30 h earlier,
   (b) had one new red, which the coordinator explained as not the chain's work in the same minute, and (c) has its
   last member queued for CI. Recommended tripwire rules:
   - Do not count reds that were closed as "explained red".
   - After a review, re-fire only on new attributable reds since that review.
   - Do not fire while the chain's head member is `queued_ci` or `finished`.
   - Make the grouped reasons match the count.
2. **Continuations that re-port do not re-size.** All 7 briefs carry the same header (Size M, sonnet, 110 calls),
   although S11-5 to S11-7 did different jobs (a merge, a structural re-port, a data-only re-port). A merge-only run
   that takes 3.2 h is a signal in itself. It would be cheaper to re-plan a slice that has finished but still has to
   integrate as one data-only rebase story when it is unheld, instead of a continuation chain that inherits the
   original brief's spec.
3. **A shared check that fails on `next` makes every slice work around it.** The S11-7 handoff reports 8 failures in
   `header_check.py` on `next`'s own `sys/mount.h` (P8, `struct winsize` redefined against `linux/mount.h`). To get
   a passing baseline, its 8 header_check plants had to filter those lines. Every later slice will pay the same
   cost. The related owed item #85 waits on SL-HEADERS-S07-TAIL, which waits on S11-7 and then the rest of the queue.
   I did not verify whether CI runs this check.
4. **The CI red outside the chain (rule 3: no flakes).** The coordinator recorded the `forced_unwind_products` failure
   as its first occurrence. Under rule 3 an intermittent failure is a defect to explain, but I found no story that
   owns it. Until one does, any branch can draw this red, and the tripwire attributes it to that branch.

## Recommendation

**Continue as is for S11-7** (in CI now). Named follow-ups, none of which needs a new S11 run:

- Give `math-vector-fortran.h` an owner: add it to HEADERS-TABLES-COMPLETE-2 or to the complex.h/tgmath.h slice
  that story promises.
- HEADERS-TABLES-COMPLETE-2 already restores the decimal spelling of the `values.h` constants. Until it lands,
  `values.h` on next has 4 macros that do not lex in C89 or C++98-14. This is a labelled gap, and headers ship in no
  package today (#6701).
- Tripwire changes as in "Processes" item 1.
- A story for the `forced_unwind_products` CI failure, if the coordinator sees it again.

Not recommended: a RETHINK of the slice or the generator (the cost was integration, and #6748 has removed its
cause), or stopping (the remaining work is one CI run).

## Not verified

- Whether S11-3's bit-field offset correction (46 rows) survives on the S11-7 head. Layout rows were kept, but I did
  not read the tables.
- When ruling #6597 was made relative to S11-5's start (it would show whether S11-5's merge-only brief was already
  stale when written).
- Whether CI runs `header_check.py` (process item 3).
- How the tripwire computes "measured gains". The 6 matches the number of runs with at least one `measured: true`
  entry (S11, -2, -3, -4, -6, -7). I did not read the tripwire code.
