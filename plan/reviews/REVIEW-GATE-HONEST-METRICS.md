# REVIEW-GATE-HONEST-METRICS: independent review of the chain GATE-HONEST-METRICS (2026-11-24)

Reviewer: independent (no stake in the chain). Trigger: the rabbit-hole tripwire, "3 reds >= 3" (03:54Z 11-24).
Sources: the four members' ops logs and briefs, their hand-off reports on their branches, the ops database (read-only:
runs, events), owner rulings #8133-#8136, #8143, #8144, #8153, #8157, #8158 (liella-ops `state/PH14-OWNER-RULINGS.md`),
GOALS.md, the LOG tail, and two CI-host logs (the GATE-HONEST-METRICS-3 release run and train 975). Host: "the CI host"
below is the Skylake-SP build host that runs CI.

## Verdict

**Not a rabbit hole in the classic sense (each red had a real, different, explained cause), but the chain is now being
steered into work the owner paused (text layout), on a stale premise, for a red its branch alone does not produce.
Continue with named changes (section 5); do not let GATE-HONEST-METRICS-4 bin-fit.**

## 1. The chain in numbers

| Member | Run (UTC) | Agent-h | Cost | Calls | Outcome | Red cause (verified) |
|---|---|---|---|---|---|---|
| GATE-HONEST-METRICS | 11-23 17:41-20:07, 2 legs | 2.43 | $9.73 | 224 (budget 170) | INCOMPLETE "base only" (asked for a merge ruling) | verify: a named job red on next's base (4 tests failed the same on base 0495c4684) |
| -2 | 20:48-23:05 | 2.28 | $3.16 | 94 | COMPLETE, verified, queued | CI train: `init_data_clusters_hold_at_every_text_phase`; cause found by -3: regen.sh read section names with `readelf -SW` (cut at 256 bytes), so a traced 538-byte function was never grouped. The chain's new cause test caught a real, older tooling defect |
| -3 | 00:30-02:20 | 1.83 | $4.46 | 122 (extended to 255) | fixed the readelf cut; verified, queued | CI train 975 release step: WINDOW-BIN-BYTES "mode tool: 3 windows > windows-release 2" (see 3.1: attribution unproven) |
| -4 | 03:54- (live) | - | - | - | told to fit tool in 2 windows or ask a ruling | - |

Totals before -4: 6.5 agent-hours, $17.35, 440 calls, 5 session legs, 2 CI trains red (30 min/6.9 GB and 48 min/15.0 GB),
each red train also delaying its co-members (3 and 2 stories). Nothing of the chain has landed.

## 2. Premises that look false

1. **"Nothing has landed since 06:44Z on 11-23; this test is one of the blockers."** Stated in the Premise line of all
   four briefs (copied unchanged into -2, -3 and -4). Contradicted by the ops database: landings resumed at 17:43:57Z on
   11-23 (two minutes after the chain's first run started) and 55 stories have landed since. The test's blocker role
   ended when the owner restored the CI host's locale-archive at 18:10Z (LOCALE-PER-DIR-LOAD's log: "17:34 red was the
   host (no locale-archive since 06:32)"). Since then `init_data_clusters_hold_at_every_text_phase` has gone red in CI
   only in trains carrying this chain or LOCALE-PROBE-GEN-2 (20:24Z), and its three train-mates of 00:00Z (PRINTF-NO-HEAP-4,
   CTYPE-OBJECTS-SETLOCALE-3, FNMATCH-GROUP-FREE-COMPILE) landed at 01:46Z without it. The shared instruction bound
   (#8158) has not reddened a train since 16:07Z on 11-23. The chain still runs at priority 95 as an unblocker; on the
   data it is now an improvement story whose main blocking effect is on LOCALE-PROBE-GEN-3 (parked on it, owed #217).
2. **"The third tool window is your change" (GATE-HONEST-METRICS-4 brief, coordinator note on train 975).** -3's own
   release build of its head passed the same gate: `WINDOW-BIN-BYTES ok`, tool in bins 0 and 2 (the locale bin already
   included), 2 of 2 windows (its `htp-release2` log, lines 225-231). In train 975's merged head the third tool window
   is **bin 1** (modes fork, localtime, mktime, rebind, tool; the tool members listed there are numeric functions such
   as `strtol_narrow`), bin 1 grew 55,740 -> 58,748 bytes and bin 2 shrank 64,641 -> 61,355 bytes. That is the packer
   reassigning members across bins for the merged set of three branches, not the per-locale-directory trace on its own.
   The manual finishes of INTERNAL-ALLOC-ROUTING-3 and LOCALE-COLLATE-TAILORINGS-4 at 03:53Z rest on the stronger claim
   ("the only branch that changes hot-text tracing"). Their next train, without this chain, is the test. Flagged to
   the coordinator as #8321 while -4 is live.
3. **The tripwire's "3 failure reds".** The first red was an INCOMPLETE hand-back by design (it asked for a merge
   ruling); its red reason begins "verify: job ... rc=1" rather than "verify: incomplete hand-back", so
   `ops/tripwire.py` NOT_FAILURE_REDS did not excuse it. On the tripwire's own rules this chain has 2 failure reds. The
   review is still timely: under owner ruling #8157 (5th story, $20 or 8 legs with no landed measured gain) the chain
   reaches $20 during -4. But #8157 is not built: the tripwire still runs the old defaults (3 reds / 3 continuations /
   8 agent-h), owed #204, due since 17:51Z on 11-23.
4. **"6 measured gains".** Of the impact entries marked measured, the user-facing ones are: page faults on a host with
   per-locale directories, static locale 56 -> 53 and tool 58 -> 55, dynamic locale 107 -> 103 and tool 109 -> 105
   (run 1, fast profile, the CI host before its archive was restored), and 0 -> 4 name-service functions grouped
   (-3, a count, no fault number). The rest are proof or repairs of the chain's own code: plants (2, 3, 4), the
   `reference_probe` link race (the probe is new on this chain's branch; it is not on next), and a tooling entry with
   no numbers. A measured cost is reported correctly by -2: the cause test's wall time 48 -> 78 s. None is delivered.

## 3. Processes due for a rethink

1. **WINDOW-BIN-BYTES as a hard per-story, per-train gate with ~1% headroom.** -3's release build: bin 0 headroom
   1,592 of 64,512 bytes, bin 2 895, bin 3 869. With per-mode window budgets on top, a few hundred bytes anywhere can
   move a member into another bin and add a window to a mode that no branch touched. The gate is not local (a red
   names the mode, not the change), not monotone (adding bytes in one place moves another mode), and only appears on
   the merged train head, after a developer's own release run was green. The owner's frame review (#8143) found that
   "placement is the start-up lever" and "the static layout contracts protect what users run" were both false
   assumptions, and paused layout work aimed at static start-up. #8135 has already moved fault metrics to a ratio to
   the reference, with absolute counts reported but not gating. A window count is a proxy for a proxy. Release-step
   reds since 12:00Z on 11-23: 17 (not all of them window reds).
2. **Continuation briefs copy the parent's Premise line unchanged.** That is how premise 1 stayed in -2, -3 and -4 after
   it went false. The context page's "Nothing landed on next since it was submitted" is about the brief's own
   submission and does not recheck the inherited premise (E6).
3. **Bundling.** One "M" story carries six rulings (#8133, #8134, #8135, #8136, #8144, #8158). The honest-metrics
   part (fault and instruction bounds as ratios to the reference, kernel instructions reported, probe modes) does not
   depend on hot-text layout. It is held behind the part that does (#8133 with #8134, plus -3's grouping fix), and
   that part is the one that keeps meeting layout gates. Nothing ships until both fit.
4. **A host-dependent code path tested through host state.** The cause test's verdict depends on which locale form
   the CI host has (archive or per-directory). That form changed twice in 12 hours (06:32Z an install by the
   coordinator, 18:10Z the owner's restore). -2's IMPROVEMENT line already proposes the fix: pin the inputs that pick
   the path, for example a locale fixture under the test's control, so both forms are exercised on purpose on every
   host.
5. **The tripwire** (premise 3): build #8157 (owed #204), and excuse a red whose run handed back INCOMPLETE, whatever
   the red's reason text.

## 4. Cost against gain

- Cost: 6.5 agent-hours, $17.35 (+ -4 live), 2 red CI trains (78 min, 21.9 GB written), 5 co-member delays, the cause
  test +30 s per run, LOCALE-PROBE-GEN-3 parked.
- Gain, measured, not delivered: -3 to -4 page faults (about 3-5%) for locale and tool start-up, only on hosts that load
  locales from per-locale directories; a real tooling defect found and fixed (section names cut at 256 bytes, which
  silently dropped traced functions from the hot group on any host); the honest-metrics machinery (ratios, kernel
  report, modes) green on the developer gates three times. The tooling fix and the metrics are the durable value. The
  per-directory fault gain is small and conditional.
- Unknown, and it decides #8134's worth: which locale form do the target distributions' default configurations load
  (archive, or per-locale directories such as a shipped C.UTF-8 directory)? And what does grouping the per-directory
  functions cost hosts that use the archive (hot-window bytes that are mapped but never run there)? Nobody has
  measured either. Decision rule: if the per-directory form is common in the target configurations, or the archive
  form pays 0 extra faults for the grouping, keep #8134. Otherwise leave the per-directory path cold, and have the
  cause test exercise it as a declared cold path.

## 5. Recommendation: continue with named changes

1. **-4 does not bin-fit.** Its brief asks it to fit tool into 2 windows or to ask for a ruling. Premise 2 shows the
   overflow is a property of the merged train, not of its branch. Have -4 confirm it with one release `so` of its
   head (expected: tool 2/2) and hand back.
2. **Split the chain when it next goes red, or now if -4 has not started layout work.** (a) Honest metrics plus the
   section-name fix land first. These are the chain's namesake and are layout-neutral, except for the 4 functions -3's
   fix rightly groups. (b) #8133 and #8134 go into their own story, after the locale-form data in section 4, with the
   test pinning its locale form (3.4).
3. **Ruling on WINDOW-BIN-BYTES as a gate** (RULING NEEDED below).
4. **Fix the premise process:** a continuation brief re-derives its premise from the database at submission (landings,
   which reds are still live), or marks it "inherited, not rechecked".
5. **Build #8157** (owed #204) and the INCOMPLETE-red excuse in the tripwire.

## RULING NEEDED

- Should WINDOW-BIN-BYTES's per-mode window budgets (release) stay a hard per-story gate, now that #8143 found
  placement is not the start-up lever and #8135 gates faults as a ratio to the reference? Recommend: once #8135's
  fault-ratio gate lands, report the window counts and stop gating on them. Until then, a window red on a merged train
  head whose members each pass alone belongs to the train (compose, then bisect), not to one story. This is a
  loosening of an existing gate and needs the owner. The data to request: tool's measured faults with 2 and with 3
  windows.
- Does #8134 (group the per-locale-directory load path) stand after the CI host's locale-archive was restored, given
  #8143's pause on static start-up layout work? Recommend: hold #8134 until the section 4 measurement (the locale form
  of the target configurations, and the archive host's cost of the grouping) answers the decision rule. Meanwhile the
  cause test pins its locale form.
