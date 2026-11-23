# Chain cost against measured gain (CHAIN-COST-AUDIT, goal E4)

Audit of every continuation chain and theme with runs from 2026-10-24 to 2026-11-23 (as of 17:30Z), ranked by
cost per measured gain, the worst 10 explained, and a proposed E4 tripwire with thresholds taken from this data.
Read-only: no story, decision, product or test changed.

## How to reproduce

```
python3 tools/chain_cost_audit.py --db ~/state/ops.db --since 2026-10-24 \
    --now 2026-11-23T17:30:00Z --out results/chain-cost-audit --worst 10
```

The database is opened with the sqlite `mode=ro` URI. The tool writes `chains.tsv` (all 689 chains),
`themes.tsv`, `ranked.md` (summary, evidence counts, the top 40 ranked rows), `tripwire.md` (the threshold
tables below) and `evidence.md` (each run's one-line result and each red of the worst 10, host names and home
paths replaced). Definitions are in the tool's docstring; in short:

* **Chain**: stories joined by a continuation id (`X-N`, N 2..30, to `X` or a sibling), a "continued as" or
  "now waits for" event, or a Stacks-on parent other than `next` with at most 3 followers (the 21 parents with
  more are planning batches and are not chain edges). A chain is named by its most frequent stem.
* **Cost**: `runs.cost_usd`. Runs before 2026-11-10 have no cost recorded; their cost is tokens x the per-model
  $/token ratio of the runs that have both ($953 of the $4,665 total is estimated this way).
* **Gain**: a hand-back impact row with `measured: true` of kind perf, faults, size, accuracy, correctness or
  tooling (a numeric perf/faults/size row only when after < before); the same `kind`+`what` re-measured by a
  continuation counts once. Coverage, unblock, none and cost rows are not gains. **Realized** = the gain is in
  the impact of a story that landed. Impact is read from `runs.impact` (recorded from 2026-11-12) or else
  `handoff/reports/<ID>.json`. `n/r` = no impact record at all for the chain (mostly before 2026-11-12): gain
  unknown, not zero.
* **Reds** (transitions into `red`): *own* (the story's verify, hand-back check or precheck failed; CI red
  attributed to it alone), *incomplete* (an INCOMPLETE hand-back sent on to a `-N`), *ci-conflict* (merge
  conflict with next), *ci-shared* (CI red attributed to several stories or none), *env* (wall-time kill, API or
  spawn failure, no final result, refused command), *coordinator* (manual fail).

## Overall

689 chains, $4,665, 1,757 runs, 516 realized gain items: $9 per realized gain overall.

| stories in chain | chains | cost $ | $/chain | landed % | realized gains/chain | $ per realized gain |
|---|---|---|---|---|---|---|
| 1 | 362 | 1,108 | 3.1 | 83 | 0.67 | 5 |
| 2 | 130 | 671 | 5.2 | 89 | 0.59 | 9 |
| 3 | 68 | 724 | 10.6 | 90 | 1.25 | 9 |
| 4-5 | 74 | 846 | 11.4 | 95 | 0.91 | 13 |
| 6+ | 55 | 1,317 | 23.9 | 95 | 0.82 | 29 |

Long chains almost always land (95%) but deliver no more realized gain than single stories while costing 8x as
much: 55 chains of 6+ stories took 28% of all cost for $29 per realized gain, 6x a single story's $5. In chains
of 3+ stories, 10-11% of runs (121 runs, $215) were mainly "merged next" maintenance, against 2% in chains of 1-2.
Reds: own 537, incomplete 254, ci-conflict 155, ci-shared 105, coordinator 74, env 48. Cost per run rose from
$3.03 (11-10..11-16) to $4.51 (11-17..11-23), so a chain of a given length now costs about 1.5x what it did.

Themes (first id token; full list in `themes.tsv`): LIBM $642 (67 chains, 19 of 4+ stories), DL $567 (15 of 4+),
LAYOUT $346, SL $202, TIMING $150 and PERF $111 (no realized gain recorded: mostly before impact recording),
HEAP $140, LOCALE $87 (5 chains, 2 landed).

## Ranked: worst chains by cost per realized gain (cost >= $25)

Reds = own/incomplete/env/ci-shared/ci-conflict/coordinator. Legs = session legs (refreshes); 0 before the leg
table existed (11-10). Full table with every column: `ranked.md`.

| # | chain | stories | depth | legs | cost $ | agent-h | reds | landed | impact recs | realized | $/gain |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | LAYOUT-TEXT-PIN (+DATA-CLUSTERS, FAULT-CONTRACTS) | 31 | 15 | 31 | 88 | 30.6 | 13/11/0/2/2/2 | 5 | 2 | 0 | inf |
| 2 | TB-FOREIGN-WORDS-LOW (+TB-FOREIGN-SAFETY) | 27 | 14 | 27 | 80 | 28.2 | 15/5/2/0/3/2 | 2 | 15 | 0 | inf |
| 3 | TIMING-COV-MEMCMP-NONAVX2 (timing stack) | 27 | 4 | 0 | 57 | 24.6 | 3/7/0/2/3/0 | 14 | 0 | 0 | n/r |
| 4 | LIBM-F64-TRIG-QUICK (+EXP/LOG/POW/INVHYP quick) | 18 | 3 | 1 | 50 | 21.4 | 8/5/0/3/5/0 | 8 | 0 | 0 | n/r |
| 5 | LIBM-SPECIAL-FAST-TIER (+BESSEL/GAMMA, SINF-SHRINK) | 19 | 6 | 0 | 44 | 19.9 | 4/4/1/0/11/0 | 6 | 0 | 0 | n/r |
| 6 | DL-SEARCH-CACHE (+DL-MAP, RELOC, IMAGE-READER) | 14 | 4 | 14 | 40 | 15.4 | 6/1/0/0/2/0 | 5 | 0 | 0 | n/r |
| 7 | STARTUP-FAULTS-RESTORE (+LAYOUT-MANIFEST, ...) | 15 | 5 | 12 | 39 | 15.7 | 10/0/0/0/0/0 | 7 | 0 | 0 | n/r |
| 8 | CLASSED-SLOT-INFRA (+NULLFOLD-MEMSET) | 13 | 7 | 0 | 34 | 12.6 | 5/4/0/0/0/0 | 5 | 0 | 0 | n/r |
| 9 | PRINTF-NO-HEAP | 3 | 3 | 9 | 34 | 9.4 | 2/1/0/0/0/0 | 0 | 3 | 0 | inf |
| 10 | INTERNAL-ALLOC-ROUTING | 3 | 4 | 8 | 29 | 7.8 | 1/1/0/0/0/0 | 0 | 3 | 0 | inf |
| 11 | SHSTK-OPT-IN | 3 | 2 | 7 | 29 | 5.7 | 0/0/0/2/0/0 | 0 | 3 | 0 | inf |
| 12 | LOCALE-PROBE-GEN | 4 | 2 | 5 | 29 | 6.8 | 1/0/1/4/0/0 | 0 | 4 | 0 | inf |
| 13 | LIBM-F32-ONEPASS-REST | 10 | 5 | 10 | 27 | 9.6 | 4/2/1/1/0/0 | 3 | 0 | 0 | n/r |
| 14 | HEAP-ADDRESS-SPACE | 10 | 10 | 14 | 50 | 20.0 | 6/4/1/0/0/0 | 1 | 10 | 1 | 50 |
| 15 | UTIL (UTIL-N) | 9 | 4 | 14 | 49 | 16.3 | 1/2/0/1/2/0 | 3 | 9 | 1 | 49 |
| 16 | WCTYPE-TRIE-FILE | 13 | 11 | 11 | 40 | 17.6 | 10/1/0/0/1/0 | 2 | 5 | 1 | 40 |

By absolute cost the largest chain is DL-DLSYM (34 stories, $112, 34 agent-hours, 30 reds), ranked 20th because
it realized 9 gains ($12 each). LOCALE-SLOTS-GROWTH went to -7 ($28, 8 runs) before landing.

Premise check: by story id PRINTF-NO-HEAP is at -3 (not a 4th continuation) but its three runs used 9 session
legs; INTERNAL-ALLOC-ROUTING has a -4 that was closed without running; LD-RELR-LOOP landed at -4 for $16 and
does not rank. Refresh legs are continuations the id does not show, so the tripwire counts legs too.

## Worst 10: what kept them going

Evidence: each run's hand-back first line and red reasons in `evidence.md` (quoted fragments below).

1. **LAYOUT-TEXT-PIN, LAYOUT-DATA-CLUSTERS (31 stories in 4 days, depth 15): moving target and brittle bounds.**
   The goal (start-up text and data windows, fault counts) was measured against contracts and hot-text key lists
   that next kept changing: 7 of 31 hand-backs (the tool's regex count) report "merged next" and regenerated lists,
   and the reds are
   start-up fault and window bounds moving by one ("start-up +1 found (`__libc_single_threaded` read ... outside
   the .data...)", "fork child 7->6 cause found", "DO NOT LAND: ... fast true 15->13 but tool 58->59, locale
   55->56"). 11 INCOMPLETE reds: each leg ended a step short. Its two impact records hold no realized gain. The
   start-up layout-jitter ruling (#4347, 11-10, amended 11-13) came after this chain was mostly spent.
2. **TB-FOREIGN-SAFETY, TB-FOREIGN-WORDS-LOW (27 stories, depth 14): brittle bounds and a moving target.** The same
   layout contracts: "next's new STARTUP-DATA-WINDOWS contract fails on this branch (fast 4>3)", "setlocale +
   printf fault count back within its bound (52 -> 51, bound 51, next 50)", "CI's 8th written start-up page is
   the 1640-byte GLIBC_TUNABLES name table", ending with an owner ruling raising a bound ("errno_location limit
   11 -> 12"). 15 gains were recorded on hand-backs, none on the two stories that landed: the realized gain is
   not visible to the record.
3. **TIMING-COV / TIMING-N100 / TIMING-IVB (27 stories): a theme, not one chain.** Joined by Stacks-on edges of
   small timing stories (pre-impact, n/r). Several ended "DO NOT LAND" on time results ("N100 time memchr_64k
   -8.2%, ... (parity"). Cost per story was low ($2); the waste is 7 INCOMPLETE reds, not depth.
4. **LIBM-F64 quick phases (18 stories): planned decomposition plus conflicts on shared files.** One story per
   function family (exp, log, trig, pow, invhyp), each wired and then re-merged; 5 CI conflicts and lint/static
   reds ("Fixed the two static findings that turned -2's lint red"). Hand-backs report measured instruction cuts
   ("pow 177.5 vs 141.3 instr FMA") but before impact recording (n/r). Not a rabbit hole; a planned series whose
   cost is inflated by serial landing on the same files.
5. **LIBM-SPECIAL fast tiers (19 stories): the environment of shared files.** 11 of 20 reds are CI conflicts;
   five runs are only "merged origin/next" onto the per-family table split ("-5 merged with next (hot.rs/fast64.rs
   both sides kept"). Gains were measured but not recorded (n/r). Cause: concurrent stories editing hot.rs and
   the generated tables, not the design.
6. **DL-MAP, DL-RELOC, DL-SEARCH-CACHE, DL-IMAGE-READER (14 stories): scope by design.** The loader plan's steps;
   reds are the story's own gate and plants ("corpus had no IRELATIVE"), fixed at the cause next run. Delivered
   and landed (5); n/r only because impact was not recorded. A tripwire must not stop this kind of chain.
7. **STARTUP-FAULTS-RESTORE, LAYOUT-MANIFEST, LAYOUT-STARTUP-* (15 stories): brittle bounds built in parallel.**
   All 10 reds are own gate failures; several stories added layout contracts (STARTUP-TEXT-WINDOWS, -DATA-WINDOWS,
   HOT-GROUP-ANCHOR) that the sibling stories then had to satisfy. "64 KiB hot-group anchor measured worse
   (start-up faults +1..+4 in every row)" was the measured stop of one branch.
8. **CLASSED-SLOT-INFRA, NULLFOLD-MEMSET (13 stories): a design that could not reach its target.** "DO NOT LAND:
   swap meets the 4-syscall/2-VMA bound, but every start ... ", then "swap off by default", and for
   NULLFOLD "Ruling #3621 branch 3: no memset code change (D-4.60 REJECTED)". Seven CLASSED-SLOT continuations
   ran after the first DO NOT LAND measured the fixed start-up cost; the stop came from a ruling, not a trigger.
9. **PRINTF-NO-HEAP (3 stories, 9 legs, $34, not landed): scope growth and a brittle product bound.** Each run is
   INCOMPLETE and the scope widened per leg ("printf allocates nowhere ... and swprintf truncat..." -> "long %f
   one pass (18,444 -> 17,813 instr/call)" -> "printf_no_heap.rs folded into s12_stdio_symbols"). Reds: the
   hot-text job and `niji::catalogs_products`. Cost per run ($11) is 2-3x the October chains'.
10. **INTERNAL-ALLOC-ROUTING (3 stories, 8 legs, $29, not landed): design iteration against a brittle bound.**
    "step 1 measured (+2 instr per allocation...)" -> "option 1b measured: -16 instr per allocation". The red is
    `niji::hot_text_products init_dynamic_empty_main_stays_within_its_instruction_bound`, the test named in 11
    reds since 11-16 across many unrelated stories (CI train 953 went red on it today).

Shared causes, by cost: (a) start-up layout bounds and contracts that move under the branch (chains 1, 2, 7,
and the `init_dynamic_empty_main` bound in 10, 11 and today's CI reds); (b) serial landing of stories that
edit the same files (4, 5: 16 of 41 reds are CI conflicts); (c) INCOMPLETE legs that end one step short (254
incomplete reds over the window, 11 in chain 1); (d) a design kept going after a measurement said it could not
reach its target (8); (e) scope growth inside a continuation (9). Wrong premise was not the main driver of any
of the ten. The record also hides gain: 3 of the top 10 (DL, LIBM) delivered measured work before impact
recording, and chain 2 recorded 15 gains only on stories that did not land.

## Proposed E4 tripwire

Data (chains first run on or after 2026-11-13, when every hand-back records impact; `tripwire.md`):

| fire when | chains fired | per day | landed a gain after firing | median hours to fire | cost after firing |
|---|---|---|---|---|---|
| 4th story or $15 without a landed gain | 52 | 4.8 | 33 | 6.0 | $390 |
| 4th story or $20 | 46 | 4.3 | 30 | 7.6 | $328 |
| **5th story or $20** | **27** | **2.5** | **15** | **15.9** | **$230** |
| 5th story or $25 | 23 | 2.1 | 11 | 17.0 | $181 |
| 6th story or $25 | 17 | 1.6 | 9 | 18.1 | $142 |

Cost alone: 17 chains crossed $20 before any landed gain; 12 later landed a gain, $163 was spent after crossing,
$40 of it in chains that never landed. Cost before any *recorded* gain never fires above $10 (hand-backs record
gains easily), so the trigger must count only gains on stories that **landed**.

Proposal (the coordinator builds it in liella-ops; numbers from the table above):

1. **Fire a chain review** when, with no story of the chain landed with a measured gain since the chain began
   (or since its last review), any of:
   * the chain reaches its **5th story** (the `-5` continuation, Stacks-on followers included), or
   * its **cumulative cost passes $20** (`runs.cost_usd` summed over the chain), or
   * its **session legs reach 8** (PRINTF-NO-HEAP and INTERNAL-ALLOC-ROUTING reached 8-9 legs in three stories,
     invisible to an id count).
   At today's volume this fires about 2.5 times a day, a median 16 hours after the chain starts (inside a day:
   "hours, not days"). Applied to the worst 10 it fires on all of them: chains 1-8 at their 5th story,
   PRINTF-NO-HEAP and INTERNAL-ALLOC-ROUTING on cost ($20) and legs (8) in their third story. On the DL and
   LIBM series (4-6) it would stay quiet only if their landed steps record a gain, which they now must.
2. **The review is a decision, not a stop.** 15 of the 27 chains this rule fires on went on to land a gain, so
   the review asks: which of the five causes above holds (moving target, brittle bound, environment, scope
   growth, unreachable design), and is the next continuation the right fix? Default outcome: the chain is held
   until the review's answer; a "continue" re-arms the trigger at the next $20 or 3 more stories.
3. **Second-order triggers** (each also fires a review): two consecutive INCOMPLETE hand-backs whose results
   both mention merging next (maintenance legs; 7 of 31 runs in chain 1); one named test failing in reds of
   3 or more different stories within 7 days (the `init_dynamic_empty_main` bound: 11 reds since 11-16; this is
   a bound to review, not a story).
4. **Do not count** chains whose stories are a planned series that land one by one (each continuation lands
   with a gain: DL loader steps, LIBM per-family tiers); a landed gain re-arms the trigger, so they pass.

## Limits of this data

* Before 2026-11-10 cost is estimated from tokens; before 2026-11-12 impact is not in `runs.impact` and only some
  report JSONs carry it, so 2026-10-24..11-12 chains are often `n/r`. No runs are recorded 2026-10-25..11-03.
* Stacks-on edges join neighbouring work into one chain (chains 1, 2, 3, 7); the stem view in `chains.tsv` column
  `depth` gives the longest single-stem run.
* Gains are counted, not weighted: a 1-instruction cut and a 40% cut count the same. Ranking by realized-gain
  count is a floor on value, not a measure of it.
* Runs live at 17:30 record their cost when they end, so a re-run with the same `--now` gives slightly higher
  costs (the committed tsv/md files, regenerated at 17:40, total $4,679 against the $4,665 quoted above; the
  ranking and the tripwire tables do not change beyond one chain at the $8 row).
* Chains still running (PRINTF-NO-HEAP, INTERNAL-ALLOC-ROUTING, SHSTK-OPT-IN, LOCALE-PROBE-GEN) show `inf`
  partly because they have not landed yet.
