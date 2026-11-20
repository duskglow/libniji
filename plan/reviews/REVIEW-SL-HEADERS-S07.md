# REVIEW-SL-HEADERS-S07: independent review of the SL-HEADERS series

Written 2026-11-20 by the independent reviewer (story REVIEW-SL-HEADERS-S07, a planning story: no code changes).
The rabbit-hole tripwire fired on chain SL-HEADERS-S07. The coordinator then widened the scope to the whole
SL-HEADERS series, because the tripwire fired on seven of its chains at once (S07, S08, S10, S11, S12, S16, S19).

Sources (all read-only): the ops database (`stories`, `runs`, `events`), `ops log` for every S07 member, each S07
member's `handoff/reports/*.json` on its branch, the run transcripts of the five runs killed at the wall-time limit,
`ops/tripwire.py`, `ops/reviewer.py`, `state/GOALS.md`, the tail of LOG.md and the HEADERS-GEN-S07-HOOKS brief.

## Verdict in one paragraph

The series is not a rabbit hole in its content. The header work for each slice converges in one to three runs.
For S07, the content was done by the third run (S07-3: P2/P3 closed, P1/P5/P6/P10 green, ~3 agent-h). The cost after
that is **integration**, not content. Several slices ran at the same time and all edited the same shared generator
files (`xtask/src/headers.rs`, `spec/tools/header_check.py`, `docs/differences-from-glibc.md`). Each red sent a whole
fresh run back through re-reading, merging, regenerating, gating and planting. On top of that, five runs on 11-19 hit
the 4 h wall-time limit while they waited on their own gate or plants jobs, and their hand-backs were lost.

The coordinator's ruling #6748 already targets the main cause: slices become data-only, and shared code lands first
in separate HEADERS-GEN-Sxx-HOOKS stories, enforced by the `slice-contract` precheck. **Recommendation: continue,
with named changes** (below). The most urgent one is a scheduling deadlock. Every remaining series story has priority
85-87, the HOOKS stories 86, and the feature pause admits only priority >= 93. So nothing in the series can run, and
S07-6, S08-6 and S10-5 wait on HOOKS stories that cannot be admitted.

## Premises that look false

1. **"no measured gain recorded in runs.impact"** (brief premise and facts line, written by `ops/reviewer.py`
   `submit_for_chain`). This is false. The line is hard-coded: `reviewer.py` writes "with no measured gain recorded"
   and "measured gain recorded (runs.impact): none" whatever the chain holds. `tripwire.chains()` on the live
   database counts **5 measured gains** in S07's `runs.impact`, for example:
   - headers generated and checked: 0 -> 24 (run 1947);
   - ctype `_IS*` masks declared: 2 -> 12 (`isalpha` and the other ctype macros did not compile against libniji's
     headers before; run 1974);
   - P3 rows (64-bit redirections under `_GNU_SOURCE` + `_FILE_OFFSET_BITS=64`): 32 -> 0 (run 1974);
   - header checks added: 0 -> 4 and 0 -> 56.

   Every tripwired chain in the series has gains on record: S08 9, S10 5, S11 4, S12 5, S16 3, S19 4. The true
   statement is narrower. None of these gains is in the last 24 h, and **none has been delivered**, because no member
   of these chains has landed. The gain is measured but stuck on unlanded branches.
2. **"5 reds" read as five failures.** `tripwire.chains()` counts every move into `red`. For S07 only **one** of the
   five reds is a failure of the work itself: S07's first gate was red at `stubs` because of a statx layout, fixed in
   S07-2. The other four are process outcomes:
   - S07-2: an INCOMPLETE hand-back always goes red, by design, even with every job at rc 0.
   - S07-3: handback-check went red on an unreserved compat-id placeholder in a `docs/` row.
   - S07-4 and S07-5: CI merge conflicts in shared files.

   Across the 7 tripwired chains, 9 of 32 reds are work failures (table below). Treating the rest as "failing again"
   overstates the rabbit hole.
3. **The 24 h numbers.** "last 24 h: 2 reds, 3.8 agent-h" is correct, but both reds are CI merge conflicts
   (S07-4 at 11-19 02:31, S07-5 at 11-19 20:38). The 3.8 h run (S07-5) was spent merging next into the branch.

## Cost vs gain: the seven tripwired chains

Red categories:
- **INC**: an incomplete hand-back that goes red by design.
- **CONF**: a CI merge conflict in shared generator files, including S11-5's manual fail to re-port onto the
  per-slice registry.
- **KILL**: the run hit the 4 h wall-time limit and its hand-back was lost.
- **STALE**: a job tested content other than the hand-back head.
- **PROC**: an id placeholder.
- **WORK**: a gate, plants or CI test genuinely red.

Gains are lifetime `runs.impact` entries with measured before/after. None of these chains has landed.

| Chain | Runs | Cont. | Agent-h | Reds | INC | CONF | KILL | STALE | PROC | WORK | Gains (24 h) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| S07 | 5 | 5 | 9.9 | 5 | 1 | 2 | 0 | 0 | 1 | 1 | 5 (0) |
| S08 | 5 | 5 | 9.6 | 5 | 1 | 1 | 1 | 1 | 0 | 1 | 9 (0) |
| S10 | 4 | 4 | 8.5 | 4 | 2 | 1 | 0 | 0 | 0 | 1 | 5 (0) |
| S11 | 6 | 6 | 11.2 | 6 | 1 | 2 | 0 | 1 | 0 | 2 | 4 (0) |
| S12 | 5 | 5 | 10.8 | 5 | 1 | 1 | 2 | 0 | 0 | 1 | 5 (0) |
| S16 | 4 | 4 | 8.0 | 4 | 1 | 0 | 1 | 0 | 0 | 2 | 3 (0) |
| S19 | 3 | 3 | 7.3 | 3 | 0 | 1 | 1 | 0 | 0 | 1 | 4 (0) |
| **Total** | 32 | 32 | **65.3** | **32** | 7 | 8 | 5 | 2 | 1 | **9** | 35 (0) |

The whole series has 52 red events on SL-HEADERS-* stories and about 103 agent-h on SL-HEADERS-* chains, plus about
7 h on HEADERS-* support stories. Seven slices have landed: S03, S04, S05, S06, S09, S14 and S17, at 1.1-9.2 agent-h
each. The slices that landed were mostly the ones that ran before or apart from the 11-17 wave. The wave started
about eight slices in parallel on the same shared files.

Where S07's 9.9 h went:
- 1.0 h, S07: 24 of 29 headers generated; the gate was red.
- 1.1 h, S07-2: the stubs red fixed; P8 and P10 added.
- 1.0 h, S07-3: P2/P3 closed, P6 22/0, ctype and 64-bit redirections fixed. **The content was complete here.**
- 3.0 h, S07-4: four reserved ids placed into records; then a CI conflict.
- 3.8 h, S07-5: merged next; then a CI conflict on `headers.rs`, which PLANTS-BUILD-SLOWDOWN had moved.

So **69% of S07's agent-hours came after its content was done**, and those hours produced no delivered gain.

Wall-time kills: 5 runs × 4 h = **20 agent-h with no hand-back at all** (S08-5, S12-4, S12-5, S16-4, S19-3, all on
11-19). These five are most of the "wall-time kills: 6.0/day" trend alert on the context page. Their transcripts show
almost the whole 4 h inside tool calls. The last calls were repeated `jobwait` on their own gate or plants jobs. One
run (S19-3) polled a background task's output file. Their `runs` rows hold no calls, tokens or cost (NULL), so this
spend is invisible to `ops digest` and to E3.

## Processes due for a rethink

1. **Parallel slices that edit shared code (E1, E3).** About 8 of the 32 reds are merge conflicts that this design
   causes. SL-HEADERS-REGISTRY (per-slice layout, landed 11-18) did not stop them, because shared code (`headers.rs`)
   still moved under the slices. Ruling #6748 is the right fix: data-only slices, HOOKS stories that land shared code
   on their own, and the `slice-contract` precheck. It is not finished, and it is not schedulable (next section).
2. **The feature-pause floor blocks the fix (E5).** `admit_min_priority` is 93. Every open series story is 85-87,
   including HEADERS-GEN-S07/S08/S10-HOOKS. So the series is frozen with 8 open chains holding unlanded measured
   gains. The ctype fix is one example: C programs that use `isalpha` against libniji's headers did not compile
   before it. The owner said on 11-20 that functionality is what blocks Yohane right now. The series is either
   Yohane-relevant, in which case the HOOKS stories need admitting, or it is paused, in which case the tripwire
   should not keep firing on it.
3. **An INCOMPLETE hand-back counts as a red (E4 signal quality).** That is 7 of 32 reds here, and 14 of 52 in the
   series. The red state is right for "do not land", but `tripwire_reds` should not count it as a failure. Otherwise
   the tripwire fires on chains that are progressing in planned steps.
4. **The reviewer brief hard-codes "no measured gain" (E6, E7).** `ops/reviewer.py` should print the chain's real
   gains: lifetime and 24 h counts and the top entries. It should also print the red reasons, grouped as in the table
   above. A reviewer should not have to rebuild them from `events`.
5. **Runs end blocked in jobwait at the wall-time limit (E1, E3).** The rule "start long jobs detached and hand back
   PENDING JOBS when about 45 min remain" is not followed in practice, because an agent does not see the clock while
   it is blocked in `jobwait`. Two candidate fixes:
   - a dispatcher-side warning (a hook message or a jobwait refusal) at deadline minus 45 min;
   - `~/bin/jobwait` refusing to block past a `NIJI_RUN_DEADLINE` and returning 3.

   Either one turns a lost 4 h run into a PENDING JOBS hand-back that the dispatcher verifies. Separately, a killed
   run's calls and tokens should be recovered from its transcript into `runs`.
6. **Continuations re-run the whole proof stack (E3).** Each continuation re-reads the brief chain and re-runs the
   gate and plants, with plants waits of 13-19 min. A continuation whose only change is a merge of next, or ids
   placed into records, is integration work. The train or a coordinator-side tool should do that work rather than a
   fresh four-hour agent run:
   - `merge_next` already resolves the `docs/` rows by itself;
   - with data-only slices, generated files can be regenerated by the train.
7. **Id placeholders (E3, minor).** S07-3 went red on an unreserved compat-id placeholder in `docs/`, and S07-4 then spent 3 h
   placing four ids. The rules already let an agent run `ops reserve --story <id>` itself. Briefs for header slices
   that are likely to add differences rows should say so, or should reserve the ids up front.

## Recommendation

**Continue the series, with these named changes (in this order):**

1. Owner or coordinator: decide whether the header slices are Yohane-relevant work under the feature pause.
   - If yes, admit HEADERS-GEN-S07/S08/S10-HOOKS (raise them to >= 93, or let them through as pipeline work), then
     S07-6, S08-6 and S10-5 as data-only rebases. Run the HOOKS stories **one at a time**, so that they do not
     conflict with each other on `headers.rs`.
   - If no, park the series explicitly (held, with a reason) and resolve the seven tripwire alerts as parked, so
     they stop generating reviews.
2. Do not start new slices (S13, S18, S20, S07-TAIL) until the HOOKS stories and the pending data-only slices have
   landed. Afterwards, run at most two slices in parallel. Then check, with the `slice-contract` precheck green on
   each, that the next slice lands without a CI conflict.
3. Tooling: make the reviewer brief print real gains and grouped red reasons; stop counting INCOMPLETE-by-design
   reds in `tripwire_reds`; add the deadline-aware jobwait or warning; record killed runs' cost.

Not recommended:
- **Stopping the series.** Its content is cheap and verified, and the cost is integration that #6748 addresses.
- **A RETHINK of the header design.** The generator and slice data are not where the cost goes.

## Not verified

- The red categories are mine, read from the `events` reason text. A few could fit two categories (S11-5's manual
  fail is conflict-driven; S10-4's precheck `headers-gen-check` red could be a regeneration artefact of a merge). I
  counted both as shown in the table.
- I did not check whether the S07 content still applies cleanly to next, or whether any of it reached next by
  another route.
- The 103 agent-h is a sum of the tripwire's per-chain hours for SL-HEADERS-* stems, rounded.
