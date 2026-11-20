# Review: chain STARTUP-GOT-PAGE (rabbit-hole tripwire, 2026-11-20)

Reviewer: REVIEW-STARTUP-GOT-PAGE (independent; no code changed, no jobs run). Evidence: `ops log` of all seven
members, their briefs, the report JSONs and handoff note on `task/startup-got-page-6`, `ops status / owes / digest`,
LOG.md tail, `crates/core/tests/hot_text_products.rs` and `records/decisions/D-X/D-X.103.toml` on origin/next
edec37111, and the two CI logs' fault lines on the Skylake-SP CI host. I did not read the chain's code diff (only
the file list: `entry.S` +8, `hot_text_products.rs` +62/-3, `D-X.122.toml`).

## Verdict in brief

The chain is **not a design rabbit hole**. Its design was settled in run 4 (option C, ruling #6606) after about
8.3 agent-hours: two options were measured and rejected by rulings, which is useful work. The remaining ~7 agent-hours
and the CI red are **landing friction** under a per-story gate the owner already ruled should change (D-X.103 /
`plan/layout-budget-rethink.md` R2, 11-14). The implementing story, LAYOUT-BYTE-BUDGETS-2, has sat on a manual hold
for 5 days. Its priority was cut from 96 to 40, and the window-bin slices that hold waits for (as far as I can tell)
landed by 11-16. The **user-visible gain of the chain is zero faults**. In return it costs up to 4 KiB of RELRO per
program, and its effect on libniji-built dynamic programs was never measured as a paired before/after. The protected
gain is gate robustness: one proxy check (STARTUP-DATA-WINDOWS) no longer goes red when relro length shifts. That
check went red once while measured faults stayed the same (44 = 44).

**Recommendation: stop the chain as briefed. Fix the shared cause first (R2), then either close the chain or land
it with one paired measurement.** Details in section 5.

## 1. The premise, checked

| Premise number | Finding |
|---|---|
| 6 runs, 15.4 agent-h | Correct. Durations from `ops log`: 2.15 + 3.82 + 2.34 + 4.00 + 1.02 + 2.05 = 15.38 h. Member 7 has not run. |
| 6 continuations | Correct, but -7 is `ready` at priority 91, below the feature pause's 93. It cannot run, and nothing has run for 25 h. |
| 6 reds "that were not INCOMPLETE hand-backs by design" | **Wrong by at least one.** Run 3's red is `verify: incomplete hand-back; its 5 named job(s) all rc 0`, an INCOMPLETE RULING-NEEDED hand-back by design (it produced ruling #6606). The red reasons listed add up to 7 (3+1+1+1+1) for "6 reds": run 6 has two (a refusal, then a CI red after a manual finish). |
| "10 measured gains" | **Overcounted.** Runs 5 and 6 copy run 4's impact entries word for word (4 entries carried forward 2 more times). Run 1's "true 82 -> 81" and run 2's "first %.2f 4 -> 3" belong to options A and B, both removed in run 4: they will not be delivered. The tripwire's "largest gains" include run 1's "first %.2f 3 -> 4 faults", which is a **regression** (option A's cost). The "size 28672 -> 32768 bytes" entry is a **cost** but is recorded as kind `size`. |
| What would actually be delivered if it lands | (a) the start-up `.got` page no longer moves with relro length (STARTUP-DATA-WINDOWS 8 -> 7 on LOCALE-SLOTS-GROWTH-5's merged head): gate robustness, measured; (b) the extended `init_startup_data_pages_hold_at_every_data_phase` test and 1 plant (caught); (c) fault counts equal to base everywhere it was measured (run 3 table, static probe, both profiles). The cost: +4 KiB RELRO (0x7000 -> 0x8000 release, 0xc000 -> 0xd000 fast, static probe). |

Run-by-run red causes:

| Run | Red | Class |
|---|---|---|
| 1 | verify: FLOAT bound and data1-cluster test failed (option A's own cost, which the run predicted) | design, produced ruling #6541 |
| 2 | verify: gate3 failed, release `true` 11 > 10 (option B's deliberate read) | design |
| 3 | incomplete by design (measured A/B/C, asked for ruling #6606) | not a failure |
| 4 | wall-time 14400 s: the FULL gate, escalated because `entry.S` defines `_start` | process (later fixed by ruling #6636: the CI train is the FULL gate) |
| 5 | verify: plants job rc=2 | infrastructure |
| 6 | refused: a **dry-run** `kill-label` of a finished run-5 job; then, after a manual finish, CI red 87 > 86 | process, then the systemic bound (section 2) |

## 2. Premises that look false

1. **STARTUP-GOT-PAGE-7's brief is stale.** It says "reproduce the 87 ... the bound is 86 and stays 86". On origin/next
   edec37111 the bound is `DYN_TRUE_BOUND = [73, 67]` (fast, release; `hot_text_products.rs:332`), tightened by
   DYN-START-PARITY-3 after LAYOUT-DYN-PHASE (D-X.133) changed how libc.so.6 and ld.so are placed. The branch's base
   923df67e9 predates all of it. The 87 cannot be reproduced on current next, and "find the one page" against a
   layout that no longer exists is not a useful step.
2. **"Run 5's htp-release was green, so the 87 is not this branch's" does not follow.** The test prints its count
   only with `eprintln!`, which a passing test does not show. So run 5's green job left no number to compare with
   CI's 87. On that base the release level was the bound (86, no margin, D-X.103's rule), so any +1 goes red. Also:
   the handoff calls the run-5 build host "Zen 1". It is the Skylake-SP class (Xeon Platinum 8151), the same class
   as the CI gate, and the bounds are per host class.
3. **The chain's dynamic effect was never paired.** `entry.S` builds `niji-crt1.o` (`xtask/src/so.rs:1387`), the
   start object of libniji-linked programs. The A/B/C table (run 3) covers the static probe only. The dynamic
   start-up-only program, the one CI failed on, was only checked against the bound as pass/fail, never paired base
   vs branch. The brief asked for "paired pre-merge and merged measurements". An extra page-aligned `.got` in a PIE
   adds a page boundary inside RELRO, which start-up writes during relocation. That is a plausible +1 fault,
   **not verified**: it is the first thing to measure, and it is cheaper than the -7 brief's interpreter-trace
   hypothesis.
4. **The chain's own premise:** "the contract counts pages, so any change of relro size can turn an unrelated story
   red" while "measured faults were unchanged (tool 44 = 44)". This is the D-X.103 failure mode exactly: a
   page-quantized proxy at zero headroom. The owner ruled the cure on 11-14 (R2: per-story gate on byte budgets
   per bin; fault counts become a per-class release metric). Pinning objects one at a time (D-X.101 relro,
   D-X.122 `.got`, D-X.133 libc/ld.so phase, HEAP-ADDRESS-SPACE-8's own `.got` alignment) treats each symptom of
   that one cause.

## 3. Processes due for a rethink

1. **Exact-count, zero-margin start-up fault bounds as a per-story gate (E1, E3).** The bound comment documents
   several things the code under test does not control: the kernel's 2 MiB placement of the first libc mapping on
   ext4 but not on tmpfs ("the cause was not isolated"), 64 KiB fault-around windows, and the traced ld.so group
   growing with unrelated loader code. On top of that the bound is set "no margin". Result: SYSTEMIC alert,
   `init_dynamic_startup_only_program_stays_within_its_page_fault_bound` red in 4 stories in 3 days (DYN-START-SPEED,
   HEAP-ADDRESS-SPACE, ICONV-NAME-MATCH, STARTUP-GOT-PAGE). Both CI reds I checked are exactly bound + 1 (87 > 86 on
   11-19, 68 > 67 on 11-20). The owner already ruled the replacement (R2). Its story, LAYOUT-BYTE-BUDGETS-2, has been
   on the manual hold "after all three LAYOUT-WINDOW-BINS slices land; coordinator unholds" since 11-14 21:31Z.
   LAYOUT-WINDOW-BINS-2, -6 and -7 landed by 11-16 01:23Z, so the hold looks 4 days overdue (E5) unless a slice I did
   not identify is still open. Its priority was cut 96 -> 40 on 11-14 22:50Z (the brief file still says 96). So the
   feature pause now refuses it, while the gate it would replace blocks the owner's priority start-up work
   (DYN-START-SPEED went red on it at 01:08Z today; DYN-START-SPEED-2, priority 95, is running).
2. **Continuation briefs carry numbers forward unchecked (E6, E7).** -7's brief, written 11-19 02:31Z, fixes
   "bound 86" and "reproduce the 87". Both were outdated before -7 could run. A cheap guard: when the brief generator
   continues a red on a named test, re-read that test's bound constant on current next and say "re-measure" if it
   changed.
3. **The tripwire's arithmetic (E4).** (a) It counts carried-forward impact entries once per run: dedupe by
   (kind, what). (b) It treats every impact entry as a gain, including a regression and a cost: count only entries
   whose direction is an improvement, and show kind `cost` and size growth separately. (c) It counts an incomplete
   hand-back that was incomplete by design as a red. (d) It fires on lifetime totals for a chain that has been idle
   for 25 h and cannot be admitted. "Cost growing" should need some recent cost. Theme grouping (OWED #26) would
   have caught the real pattern: the start-up layout theme (STARTUP-FAULTS-RESTORE, LAYOUT-STARTUP-RODATA 8 runs
   closed without landing, LAYOUT-DYN-PHASE 13.9 agent-h in the last 24 h, DYN-START-PARITY 7.2 h, this chain).
4. **A refused dry-run counted as a red (run 6).** `kill-label <label>` without `--kill` is read-only. Either allow it
   or do not count its refusal as a red. It cost a manual finish.
5. **Passing start-up tests leave no number.** Write the measured count to the job log on pass too (for example
   `--nocapture`, or a one-line result file). Then a green developer run and a red CI run can be compared without a
   rerun. Run 6 and the -7 brief both had to guess.

## 4. Cost vs gain

| | Value |
|---|---|
| Runs / continuations / agent-hours | 6 / 6 / 15.4 h (design 1-3: 8.3 h; landing friction 4-6: 7.1 h) |
| Reds | 7 red events: 2 design (rulings #6541, #6606 came out of them), 1 by-design incomplete, 3 process or infrastructure (wall time under a FULL gate escalation, plants rc=2, dry-run refusal), 1 CI red on the systemic zero-margin bound |
| Measured user-visible gain | **None.** Faults equal to base in every measured row (static probe); dynamic not paired |
| Measured pipeline gain | One false red class removed (STARTUP-DATA-WINDOWS 8 > 7 when relro shifts). It happened once (LOCALE-SLOTS-GROWTH-5) |
| Coverage | Data-phase test extended (7 relro-tail pads, `.got` page checks), 1 plant caught |
| Cost carried by the change | +4 KiB RELRO per program (measured, static); dynamic effect unmeasured |
| Cost to others | LOCALE-SLOTS-GROWTH-6 and HEAP-ADDRESS-SPACE-8 (both priority 86) held on this chain since 11-18; under the feature pause they could not run anyway |

## 5. Recommendation

**Stop the chain as briefed; rethink at the theme level; a one-step decision on the chain afterwards.**

1. Do not run STARTUP-GOT-PAGE-7 as written: its numbers are obsolete (section 2.1).
2. Fix the shared cause first. Release LAYOUT-BYTE-BUDGETS-2's hold and set it to at least 93 (owner: it is pipeline
   work under E1 and blocks the P2 start-up story), so per-story gates stop counting zero-margin, layout-dependent
   fault steps (ruling R2). This is the owner's call on priority (below).
3. Then decide the chain in one small step:
   * If R2's byte-budget gate replaces STARTUP-DATA-WINDOWS as a per-story check, **close the chain**. Its only
     delivered effect is robustness for that check, and it costs 4 KiB RELRO per program for no fault gain.
   * Otherwise brief one small continuation (Size S, about 40 calls): merge next. Run one paired count, base vs branch,
     static **and dynamic** start-up-only program, both profiles, on the Skylake-SP class where CI runs. If equal,
     land. If the dynamic count is +1, the page alignment costs a fault in every libniji-linked dynamic program:
     drop option C. Settle the single `.got` alignment with HEAP-ADDRESS-SPACE-8 (256 vs 4096) in the same step.
4. Re-point the holds of LOCALE-SLOTS-GROWTH-6 and HEAP-ADDRESS-SPACE-8 to whichever of those outcomes happens,
   not to a -7 that may never run.

## Rulings needed (owner)

* Unhold LAYOUT-BYTE-BUDGETS-2 and raise it to at least 93 under the feature pause: it implements the owner's own
  R2 and removes the cause of the SYSTEMIC start-up bound red (recommend yes).
* STARTUP-GOT-PAGE: close once R2 lands, or one paired static + dynamic measurement before landing (recommend:
  decide after R2; never land without the dynamic pair).
