# COORD-PROCESS-REVIEW: an independent review of the coordinator's decisions and the pipeline process

Reviewer: a task agent (Opus), read-only, 2026-11-23. Sources: the liella-ops `LOG.md` entries of 11-20 to 11-23
(cited as `LOG <date time>`), `state/GOALS.md`, `state/PH14-OWNER-RULINGS.md` (cited as `#<ruling>`),
`backlog/TEMPLATE.md`, and the ops database (cited as `ops.db <table>`, read-only queries). Host names are given as
CPU class ("the Skylake-SP CI host", "the Zen 1 host"); "the coordinator's container" is the ops machine.

Scope: this review covers process and roles. The 11-23 audits (CI-ENV-AUDIT, TEST-BOUNDS-AUDIT,
PERF-GAP-STRUCTURE-AUDIT, DECISION-EVIDENCE-AUDIT, CHAIN-COST-AUDIT, RELEASE-STEP-EXEC-FAIL) cover technical causes,
and nothing here repeats them.

## Summary

The coordinator works hard and documents honestly: every slip below is in its own log, most within the hour. The
problem is the shape of the system, not effort or candour. In four days one coordinator session did the following:
submitted 237 stories, made 566 to 1,169 manual ops changes a day, overrode 59 red verdicts by hand, and changed
CI, hosts and the dispatcher about 15 times. It also held 100 open owed items, 78 of them overdue. Safeguards exist,
but most of them are things the coordinator must remember to do, and at this volume it does not always remember.
The structural gains came when the owner asked a structural question. The routine machinery mostly produced patches
and continuations.

The five incidents in the brief are not five separate failures. They come from four mechanisms:

1. Changes to shared infrastructure have no change control (no canary, no blast-radius check, no rollback).
2. Next stays red and nothing stops the line. Work keeps flowing into a broken train.
3. The coordinator keeps dependencies and sequencing in its own head and fixes them by hand. Briefs are written
   against what next is expected to look like, not what it is.
4. Decisions and premises go out without evidence. Agents or the owner catch them later.

## Data at a glance (ops.db, 11-17 to 11-23)

| day | trains | red trains | landed verdicts | red verdicts | carried | runs | continuations | coordinator red->finished |
|---|---|---|---|---|---|---|---|---|
| 11-20 | 37 | 4 | 44 | 4 | 1 | 70 | 27 | 1 |
| 11-21 | 35 | 6 | 50 | 5 | 5 | 68 | 20 | 15 |
| 11-22 | 40 | 11 | 48 | 10 | 17 | 68 | 30 | 9 |
| 11-23 | 20 | 10 | 10 | 55 | 93 | 80 | 24 | 34 |

Sources: `ops.db ci_trains`, `ci_verdicts`, `runs` (continuation = story id ending `-N`) and `events`
(`actor like 'cli:claude%'`, `red>finished`).

- Owed items: 199 opened from 11-20 to 11-23 (91 on 11-20 alone), 99 paid. 100 open now, 78 past due (`ops.db owed`).
- Coordinator manual events (`cli:claude`): 1,169 / 626 / 566 / 742 per day. Submissions: 67 / 61 / 61 / 48 per day.
- Trains 946-955 (11-23 06:49Z-16:43Z) were all red, and none of the reds came from the stories in them. Failures:
  `catalogs_products`, `deb_manifest`, the release start-up check (951, 952: `RELEASE STEP FAILED: release-profile
  start-up check`) and a hot_text bound. 93 story verdicts were carried. Nothing landed after 06:44Z
  (`LOG 11-23T15:57Z`).
- Search counts over the 4-day log: "Slip" 5, "CORRECTION" 5, "Lesson" 5, "MISS/LATE" 5, "by hand" 13, "explained
  red" 16, "CI conflict" 14, held/re-held 41, "dispatcher restarted/deployed" 15, zram near-full 9.

## Patterns

### P1. Infrastructure changes go live with no change control (host packages, CI, the dispatcher, ops tools)

The evidence:
- `LOG 11-23T07:30Z`: on the coordinator's recommendation, locales-all went onto the Skylake-SP CI host for
  LOCALE-PROBE-GEN. "My dry run checked package changes only, not what the install does to /usr/lib/locale." Every
  gate on that host failed 7 locale tests from then on. An agent found the cause (#7846), not the coordinator or a
  monitor. Trains 946-955 were red and CI stopped (`ops.db ci_trains`). The 12:16Z state line costs it at
  "~$30-40", plus CI stopped since 07:30Z.
- `LOG 11-23T15:57Z`: a "CI cleanup" (ci_train_max 8, CI restarted between trains) went in the same afternoon. Trains
  951-952 then failed the release step, and RELEASE-STEP-EXEC-FAIL was briefed at priority 95. I did not verify the
  cause; the story owns it.
- `LOG 11-23T05:12Z`: `sudo` in `~/bin/gate` silently dropped `CARGO_INCREMENTAL=0`, which filled 31 GB of CI
  incremental cache. This was an earlier tooling change with an unchecked side effect.
- `LOG 11-21 ~10:20Z`: the coordinator patched `watch.sh` in place while it was running, and the watch died. The
  rule "atomic edits (write new + mv)" already existed.
- `LOG 11-22T02:59Z`: an agent's experiment bind-mounted over the CI instr role. CI could not start and lost a train
  slot. The guard is owed (#131).
- `LOG 11-20 ~18:40Z`: "pushed before running the ops tests". `LOG 11-23T12:29Z`: the coordinator's container had
  21 unpushed liella-ops commits.
- The dispatcher or CI service was restarted about 15 times in 4 days to load ops changes. Each change was small
  and tested. None was canaried on the live pipeline before it carried real work.

Safeguards that exist only on paper: TOOL INSTALLS staging and atomic edits (thread_common), "root changes need the
owner" (the owner did approve the package, but on a recommendation that left out its effect), and "pipeline first"
(GOALS.md). No tool checks a host or CI change against a known-good train of next before story trains run on it.

### P2. Next can be red, and nothing stops the line; red verdicts are overridden by hand

- `ops.db events`: the coordinator moved a story from red to finished (to CI) by hand 1, 15, 9 and 34 times on
  11-20..23. The log calls this "explained red" 16 times. At least one explanation was wrong when made:
  `LOG 11-21 ~01:20Z` CORRECTION. HEAP-ADDRESS-SPACE-10's red check was the contract CI also enforces: "it would have
  gone red in the train". Another was a real test bug, treated as known noise across four held stories until an
  agent challenged it (#7249, `LOG 11-21 ~12:35Z`: "HEAP-10 never ran pass 2 before landing").
- Next itself was red for long stretches while feature trains kept running: dns_products, "every train that runs
  dns_products fails until then" (`LOG 11-21T19:29Z`); the heap_meta NUMA test, "~1 train in 3 may go red on it"
  (`LOG 11-22T03:55Z`); locales from 11-23 07:30Z. CI was stopped by hand, and restarting it was an owed item. It
  stayed stopped 52 minutes after the fix was ready because the item came due as "info" (`LOG 11-23T12:29Z`).
- On 11-23, after the CI restart, trains of 13 to 28 stories ran into a next that was still red (948-953) and
  carried 93 verdicts (`ops.db ci_trains`, `ci_verdicts`).

Pipeline first is written down (GOALS.md trade-off rule). In practice a red next is worked around one story at a
time ("finished as explained red", "held until") rather than stopping the line.

### P3. Dependencies and sequencing live in the coordinator's head and are fixed by hand

- 41 log lines hold, re-hold or unhold stories, and many of the fixes are by hand: "re-pointed ... holds onto -3 by
  hand (fresh-brief continuation does not move holds; owed)" (`LOG 11-22T23:07Z`); "Holds re-pointed by hand ...
  (owed #153 again)" (`11-23T02:57Z`); "Removing the auto-submitted UTIL-12-2 earlier had stranded three holds"
  (`11-22T11:17Z`); "the first S12-7 submit was refused ... I paid owed #113 before checking" (`11-21T14:56Z`); the
  DL-RUNPATH-4 hold failed the same way (`11-21T18:14Z`); "auto-closed my separate follow-up ... by name"
  (`11-22T08:26Z`); auto-submits from a DO-NOT-LAND draft overwrote a hold twice (`11-23T04:09Z`, `04:23Z`).
- Stories get briefed in batches against a next that has not happened yet. STARTUP-STRUCTURE-REVIEW's seven briefs
  were submitted at once (`11-21T21:42Z`), then re-ranked, merged and removed 12 hours later
  (`11-22T09:54Z`). LD-OBJECT-ONCE-2 and LD-RELR-LOOP-2 were based before DL-PROCESS-OBJECTS-3 landed. Trains
  929/932 went red, and the cause was first misdiagnosed as a merge interaction (`11-22T22:17Z`, corrected
  `11-23T00:41Z`).
- 14 "CI conflict" lines produced merge-only continuations: LD-SPLIT-3, COMPAT19-LDSO-COST-2, DL-RUNPATH-2/3/4,
  TEST-SCRATCH-CLEANUP-2/3, UTIL-12-3, LD-PHDR-ONCE-2, LD-RELR-LOOP-4, ROOT-HARNESS-CONTAIN-3, DL-SLOT-REUSE-2,
  CRT-START-FILES-3, VALGRIND-LDSO-SYMBOLS-2. Continuations are 29-44% of runs every day (`ops.db runs`). The loader
  serialisation (`LOG 11-21T18:14Z`) came after DL-RUNPATH's third conflict, not at admission.

I could not tell which two stories the owner meant in "briefed two stories before the stories they depended on had
landed". The mechanism above produces such cases routinely.

### P4. Decisions and premises go out on supposition; agents or the owner catch them later

The coordinator's premise was false in mid-run challenges and rulings at least 11 times in 4 days:
- #6824: the RAM class of the role.
- #6857: a byte check that could not fail.
- #6866: an audit's estimate, never measured.
- #6902: a snapshot step pointed at a check that had never run.
- #7249: the NUMA lead.
- #7378: getaddrinfo_a "loss". A false statement was published in the 21.6 release notes and corrected later
  (`11-22T00:29Z`).
- #7390: the start-up multiplier in our own gate.
- #7407: AFFECTED-COVERAGE premise 2.
- #7707: what ld.so knows.
- #7740: "merge interaction".
- `11-21 ~06:35Z`: answers sent to runs that had already ended ("check the story state before answering").

On defaults, the owner caught both cases, not the coordinator:
- HEAP-CLASS-272, "an arbitrary number just to satisfy find" (`11-21 ~07:25Z`).
- The 64 KiB static TLS surplus. The coordinator endorsed it provisionally; one survey found nothing above 136
  bytes (#8009).

The challenge channel works and saves runs (the flag log, `LOG 11-20 ~03:20Z`). But it moves the coordinator's
missing evidence check onto agents at run time, after admission, a role and a run start have already been spent.
E6's indicator ("premises found false late") is high, and nothing tracks it.

### P5. Patches instead of structure; structural work starts when the owner asks

- The layout and fault chain: STARTUP-GOT-PAGE (padding, then dynamic faults got worse, `11-20 ~08:05Z`),
  LAYOUT-GOT-ORDER, LAYOUT-RO-STABLE (the .rodata1 64 KiB gap), STATIC-LAYOUT-FLIP, WINDOW-GATE-STABLE, zero-margin
  fault bounds red as "87 > 86" (`11-20 ~02:45Z`), and the LAYOUT-JITTER rule. That is days of single-page moves.
  The owner's verdict (`11-23T15:57Z`): "This whole 'moving page faults' around is a real smell".
- STARTUP-STRUCTURE-REVIEW, briefed on the owner's request (`11-21T20:48Z`), took the empty main from 742k to 323k
  user instructions (-56%) through LIBC-SELF-BIND (`11-22T08:26Z`). One structural story outweighed the whole
  layout chain. The kernel-cost probe (`11-20 ~18:00Z`) and the heap-class generality rule (`11-21 ~07:55Z`) also
  started from owner questions.
- 5 "Lesson:" lines (for example "close a chain whole", "read the submit output before paying", "check the story
  state before answering", "never TaskStop a watch without reading its output") stay as text. GOALS.md says each
  item must become "an `ops owe` item or a built check, never a line in LOG.md". Some did become checks: the watch
  flock, the cont_submit fix and the autosubmit park. Others did not.
- Disk space handled by hand 9 times (zram 91-93%) before the structural cleanup, each time with "sweep by hand".

### P6. The coordinator role is overloaded, and the owner is the early warning (E8)

- Volume: about 60 submissions and 600 to 1,100 manual ops events a day, plus the rulings the log records (I did not
  count them). In addition the coordinator triages, publishes snapshots, handles Yohane, builds ops tools and
  restarts services. Missed events
  follow: a snapshot published 3 h late (`11-20 ~10:25Z`, found when the owner asked), another 3.5 h late from a
  duplicate watch (`11-21 ~10:00Z`), log times "estimated" up to 2.5 h ahead of the clock (`11-21T12:33Z`), and a
  stray `ops paid` reverted by editing ops.db by hand (`11-22T03:11Z`).
- Owed items as the to-do list: 100 open, 78 overdue. E5 ("nothing dropped") is red by its own indicator, and the
  digest does not show it as an alert.
- Problems the owner found first in these 4 days: the late snapshot (11-20), the arbitrary heap class (11-21), the
  orphaned root processes (11-21 21:44Z, the owner said "kill them"), the 64 KiB TLS default (11-23) and the forward
  stop itself (11-23). By E8's own indicator, that is five owner-found problems in four days.

## Checks against the owner's principles

| principle | where practice departs | evidence |
|---|---|---|
| Pipeline first (GOALS trade-off rule) | A red next is routed around one story at a time; feature trains keep running; stopping CI is manual and restarting it is an owed item | P2; 11-23 trains 946-955; 52 min stop |
| When we don't know, more data (#8009 principles) | Premises and estimates go out unmeasured; agents measure them mid-run | P4, 11 false premises |
| Defaults need evidence | The coordinator endorsed 64 KiB TLS and an arbitrary heap class without a named case | #8009, `11-21 ~07:25Z` |
| Ratchet / accuracy over the ratchet (#7365 ruling) | The rule is kept, but zero-margin bounds turn layout noise into reds, and explained-red overrides go around the ratchet instead of fixing the bound | `11-20 ~02:45Z`, P2 |
| Coordinate, not micromanage | 600-1,100 manual ops events a day; holds, continuations, priorities and CI state are set by hand | `ops.db events` |
| No dedicated lanes (11-16) | Respected; the coordinator caught itself (`11-20 ~02:35Z`). Loader serialisation by chained holds was a hand-made lane-like fix after the third conflict | P3 |
| Safeguards are checks, not log lines (GOALS) | "Lesson:" lines and owed items stand in for checks | P5, P6 |

## Recommendations, ranked by expected effect

1. **Stop the line automatically when next is red (tooling; the biggest effect).** CI keeps a canary: a train of next
   alone after every infra change, and whenever a train is red with no story blamed (`basis none/global`) twice
   running. While the canary is red, CI composes no story trains and the dispatcher admits only stories tagged as
   fixing it. When the canary goes green, both resume on their own. This replaces manual CI stops, "restart CI" owed
   items, and runs carried 93 times in a day. P1 and P2 both point here.
2. **Change control for shared infrastructure (process and tooling).** Every change to a host package, CI script,
   dispatcher, `~/bin` tool or ops setting goes through one command (`ops infra-change`). It records what the change
   touches (for packages: the file list and directories it replaces, not only dependencies), the rollback command and
   the canary that proves it. The canary from recommendation 1 runs before the next story train. Host packages go on a
   non-CI host first. If a train goes red within two trains of an infra change, the change is the first suspect, and
   the alert names it.
3. **Explained-red overrides need a machine check and a budget (tooling).** `ops finish` on a red must show either
   (a) that CI's selection for that branch does not include the failing test, or (b) the same failure on a canary of
   next. Otherwise it refuses. More than 5 overrides a day raises a normal alert to the coordinator and the reviewer.
   11-23 had 34.
4. **Brief just in time, with dependencies as data (process and tooling).** A brief that stacks on an unlanded story
   stays a draft (not submitted) and is re-read against next when its dependency lands. `Stacks-on` and holds belong
   to the stem, not to one link, so continuations, removals and auto-submits move them without hand edits. At
   admission, a story whose touched paths (from the brief or the stem's diff) overlap a story in queued_ci or running
   waits. That is a file-overlap guard, not a lane. It targets the 14 CI-conflict continuations.
5. **Evidence fields at submit (process and tooling).** `ops submit` refuses a brief whose Premise quotes a cost,
   gain or size without `Evidence:` (measurement, file and line, or run) or `UNKNOWN: <decision rule>`. Rulings the
   coordinator gives are held to the same rule: a provisional ruling names its data or its decision rule (#8009
   principles). Track "premise false at run time" per week as E6's indicator.
6. **Cut the coordinator's work in progress (roles).**
   - Stop: admitting at A_max 9 while next is red; writing more than one link ahead of a chain; editing ops.db or
     live scripts by hand.
   - Hand to automation: hold re-pointing, mechanical continuations (already partly done with sonnet), snapshot
     publishing on landing (the alert exists; make it a story the dispatcher submits), and disk cleanup.
   - Hand to agents: infra diagnosis (as LD-LOOKUP-RETHINK-2 already did for locales) and a weekly owed-backlog
     triage story (each of the 78 overdue items becomes a story, a check, or is dropped with a reason).
   - Start: a daily E1-E8 line in the digest with each indicator's value. E5's 78 overdue would have been red today.
7. **Schedule structural reviews before patching starts (process).** When a primary-goal gap (start-up, faults,
   per-call rows) has two stories in a row that moved a number by less than its noise or moved cost between metrics,
   a structural review story (Effort max, deliverable: path to parity, as PERF-GAP-STRUCTURE-AUDIT) is briefed
   automatically, before a third tweak. The RUT RULE exists for agents; apply it to the coordinator's own chains, and
   count gap-to-parity progress, not only reds (the tripwire counts reds and agent-hours).
8. **Lessons become checks or they expire (process).** Each "Lesson:" or "Slip:" line gets an owed item with a
   trigger ("built as a check" or "dropped: why"). The reviewer counts the unbuilt ones.

## What works and should be kept

- The challenge channel (`ops flag`, about 11 accepted in 4 days, each saving a run or more).
- The tripwire with its fixes, and the SECURITY-PRIVATE alert.
- Ops fixes that come with a test that fails on the old code (for example the slots reserve fix, `11-22T07:10Z`,
  and the submit continuation fix, `11-22T11:17Z`). This should be the rule for every ops change.
- Honest logging: every incident in this review came from the coordinator's own log.

## Limits of this review

- I read the log of 11-20 to 11-23 only. Earlier patterns were not checked.
- Escalation texts, alert history and brief texts were not read.
- I did not verify the causes of the release-step failure or of the trains 954-955 reds.
- The pattern counts are text searches over the log and may include a few non-matching lines.
- "Coordinator manual events" counts every `cli:claude` event, including routine state moves the CLI makes on the
  coordinator's behalf. The number shows volume; it does not count decisions.
