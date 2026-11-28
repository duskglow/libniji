# PIPELINE-TEST-TIME-1127: where the test time went since 10-19

Branch `task/pipeline-test-time-1127` from origin/next af6f8acaf. Data: the coordination host's ops.db (read only:
`jobs`, `run_legs`, `runs`, `events`, `settings`), the build hosts' `~/logs/jobs.tsv` and job logs, the CI hosts'
train logs, and the session logs of agent legs. Weeks start on Monday (10-19 = the anchor week). All distributions
are p25 / median / p75 over jobs (or legs); minutes unless marked. Scripts (not committed; scratch):
`weekly.py`, `plants_parse.py`, `plants_agg.py`, `plants_split.py`, `ci_split.py`, `ci_parse.py`, `gate_agg.py`,
`affected_parse.py`, `tests_agg.py`, `instr_parse.py`, `instr_agg.py`, `legs.py`, job scripts `cold.sh`, `ab.sh`
(in `~/work/pipeline-test-time-1127/` on the Zen 1 48-CPU build host). Host classes: "Zen 1-48" = the 48-CPU Zen 1
build host (CI gate host), "Zen 1-60" = the 60-vCPU Zen 1 host (4-core job slots; ci-instr host), "SKX" = Skylake-SP.

## Summary

| what | 10-19 week | 11-23 week | cause |
|---|---|---|---|
| story plants job, wall | 2.5 / 4.4 / 8.9 | 15.3 / 23.0 / 33.2 | product builds: a cold `cargo xtask so` per copy and one per mutant (11-05 on) |
| plants per job | 3 / 6 / 10 | 3 / 6 / 9 | unchanged: not more mutants |
| minutes per plant | 0.6 / 1.0 / 1.7 | 2.9 / 4.9 / 7.7 | x4.9 |
| CI gate (Zen 1-48 only) | 14.9 / 16.7 / 23.4 | 20.1 / 25.7 / 33.6 | +5.6 min inside the full test run, +7 min release start-up check after it (11-13) |
| ci-instr (Zen 1-60) | 11.0 / 13.7 / 15.1 | 25.2 / 29.1 / 33.4 | release product build in the job (10 min), bench +2.3 min, 12 min not split by the logs |
| agent leg (20 sampled) | 13.8 / 26.0 / 40.9 | 27.7 / 63.2 / 91.5 | job waits 7.4 -> 46.5 min median; model time 5.3 -> 8.5 |

Not test bloat in the sense of more or longer tests: the full gate's test-seconds rose 7% and the plants per job not at
all. The growth is builds: products rebuilt for correctness (plants must test the mutant's products, D-X.177's
predecessor XTASK-PLANTS-PRODUCTS-REBUILD on 11-05), every plants copy built from cold (a new directory per run), and
a second release build per train in the CI gate.

## 1. Plants

Source: every plants job on the three build hosts (`jobs.tsv` kind `plants`, or a label ending in `plants[-n]`; 2051
jobs over 60 s), its log's verdict lines (`plants: <id>: CAUGHT (... build: the products, rebuilt with the mutant ...
in <n>s`) and the baseline `so ok (... in <n>s)` lines. The per-plant logs (`<dir>/logs`) were pruned, so the
baselines' and the commands' own time is the remainder ("rest") of each job.

Weekly (all hosts; `xt%` = jobs run by `cargo xtask plants`, the rest by the old script):

| week | jobs | xt% | wall | plants/job | min/plant | cold baseline build (min) | one mutant build (s) | build share of copy-time | baseline share |
|---|---|---|---|---|---|---|---|---|---|
| 10-19 | 416 | 41 | 2.5 / 4.4 / 8.9 | 3 / 6 / 10 | 0.6 / 1.0 / 1.7 | none | none | 0% | 0% |
| 11-02 | 347 | 76 | 3.7 / 6.2 / 12.3 | 4 / 5 / 7 | 1.0 / 1.6 / 2.7 | none | 55 / 71 / 122 | 21% | 0% |
| 11-09 | 368 | 100 | 9.9 / 17.9 / 31.2 | 3 / 5 / 7 | 2.8 / 4.5 / 7.5 | 3.6 / 4.9 / 5.2 | 153 / 191 / 329 | 37% | 5% |
| 11-16 | 560 | 100 | 13.1 / 21.0 / 32.8 | 4 / 6 / 9 | 2.6 / 4.1 / 7.1 | 5.4 / 6.7 / 10.3 | 79 / 181 / 284 | 28% | 29% |
| 11-23 | 360 | 99 | 15.3 / 23.0 / 33.2 | 3 / 6 / 9 | 2.9 / 4.9 / 7.7 | 8.6 / 12.3 / 15.1 | 101 / 168 / 320 | 29% | 40% |

The step, by day (ops.db, story jobs naming plants): 11-04 median 3.9 min, 11-05 6.7, 11-06 6.0, 11-07 11.8,
11-10 27.1 (no jobs 11-08/09). Commits on next (subjects only): 11-05 `Make cargo xtask plants the required plants
path`, `Rebuild the products in cargo xtask plants for commands that use them`; 11-07 the catcher; 11-13 the build's
own timeout; 11-14 ungrouped mutants; 11-19 one tag per run; 11-20 `--copies`; 11-26 products only where read. The
11-05 rebuild rule is the change: before it, a plant on a products-reading test ran against the unmutated
products (blind), so most of the growth bought plants that test what they claim. 11-09 to 11-16 then added the cold
baseline per copy (the copy is a new RAM directory with no `target/`, and `--copies 2` from 11-20 builds it twice).

Today's jobs (11-21 on), medians per host class and copies, `prod` = the job built products:

| class | copies | jobs | wall | cold baseline | mutant builds (per copy) | rest (baselines, commands, copy) | plants | rest per plant |
|---|---|---|---|---|---|---|---|---|
| Zen 1-60 | 1 | 33 | 24.7 | 10.2 | 6.8 | 8.4 | 3 | 1.86 |
| Zen 1-60 | 2 | 206 | 31.1 | 14.5 | 7.0 | 8.8 | 6 | 1.61 |
| Zen 1-48 | 2 | 186 | 16.9 | 6.8 | 4.2 | 4.6 | 5 | 1.00 |
| SKX | 2 | 51 | 22.5 | 8.8 | 5.7 | 5.0 | 8 | 0.80 |
| any | 2, no products | 11 | 3.6-16.3 | 0 | 0 | all | 9-12 | 0.42-1.81 |

So about 70% of a plants job is product builds; 45% is the copy's cold baseline alone. On the Zen 1-60 host two copies
in one 4-core slot make each cold baseline 14.5 min against 10.2 with one.

One cold baseline, measured (job `pipeline-test-time-1127-cold`, Zen 1-48, idle, the way the tool builds a copy):
`cargo xtask so` cold 297 s (xtask itself 47 s, then early-builds-wait 92 s: the interpreter's and ldconfig's
one-codegen-unit builds are the critical path); the same copy again with only `target/niji` removed (what a warm
cargo cache at the same path would pay) 79 s; a no-op 112 s. Every `cargo xtask so` re-assembles all products
(`begin_products` clears them), so 80-110 s is paid by every mutant build however small the change: libc-archive
27-39 s, charmaps-placement 15-29 s, layout 14-19 s.

Which tests each mutant runs: the plant's own command (the plants file names it); the tool adds no tests. Since
11-26 (PLANTS-PRODUCTS-ONCE) a mutant outside the product inputs no longer rebuilds products. No mutant was found
running tests that cannot reach its code beyond what the plants file asks.

## 2. CI steps

ci-gate (train gate jobs, `ci_trains.gate_job`; the 10-19 week also ran 49 on the Skylake-SP host, left out):
Zen 1-48 medians 16.7 -> 29.2 -> 23.3 -> 32.8 -> 25.7 min (weeks 10-19 .. 11-23). Split of one full-tier gate job
(`--- summary (Ns)` of `cargo xtask affected --tier full --run` vs the job's wall):

| week | jobs | wall (s) | affected run (s) | outside it, p25 / med / p75 (s) | tests run | test-seconds | test binaries |
|---|---|---|---|---|---|---|---|
| 10-19 | 178 | 1000 | 996 | 3 / 7 / 24 | 10490 | 9833 | 192 |
| 11-02 | 122 | 1755 | 1724 | 27 / 32 / 39 | 10903 | 22461 | 240 |
| 11-09 | 201 | 1401 | 1344 | 43 / 58 / 129 | 11628 | 15340 | 267 |
| 11-16 | 159 | 2057 | 1591 | 258 / 460 / 551 | 12025 | 15804 | 312 |
| 11-23 | 89 | 1704 | 1332 | 221 / 425 / 512 | 12365 | 10552 | 338 |

Steps that grew more than 20%:

1. **After the affected run, +418 s median (7 s -> 425 s):** the train's gate command gained
   `NIJI_FULL_PROFILE=1 cargo xtask so` and `cargo test -p niji --test hot_text_products` (the release-profile
   start-up check) on 11-13 21:37Z (`set ci_gate_cmd` event). Cause: an added step (a second, release-profile product
   build per train). The ci-instr job on the other Zen 1 host builds the release products too, for the same tip.
2. **The affected run, 996 -> 1332 s (+34%):** tests run +18% (10490 -> 12365), test binaries 192 -> 338, but
   test-seconds +7% only; the run's own steps (logged since 11-09): products 179-307 s (the gate's `cargo xtask so`,
   with `target/niji` cleared by the CI command first), tests-nextest 632-711 s, native pass 128-274 s, interposed
   54-68 s, test-sets 117 s (new in 11-23), rust-dylib 65-69 s, native-examples 54-57 s, conformance 24-28 s. Cause:
   more test binaries (per-binary setup and builds) and the products step, not longer tests. The 11-02 week's peak
   (22461 test-s) fell back after the TEST-REDUNDANCY-AUDIT follow-ups and the per-section binaries' fixes.
   Largest single binaries today: niji-locale::reference 1332 test-s for 24 tests; xtask's unit tests 994 -> 1526
   test-s (230 -> 660 tests); hot_text_products 477; niji-math 450 (crate names changed on 11-16, so a per-binary
   10-19 comparison only holds for unchanged names).

ci-instr (Zen 1-60; `cargo xtask bench --counters --runs 1 --only cbench_memref,cbench_hot,cbench_startup`):

| week | jobs | wall | release products (`so/total`) | of it libc-archive | bench rounds (`took`) | rest |
|---|---|---|---|---|---|---|
| 10-19 | 162 | 13.7 | not logged | | 4.5 | 9.4 (holds the build) |
| 11-02 | 187 | 15.4 | not logged | | 4.7 | 10.8 |
| 11-09 | 167 | 25.2 | not logged | | 6.6 | 18.6 |
| 11-16 | 133 | 33.4 | 14.4 | 10.9 | 6.8 | 12.4 |
| 11-23 | 68 | 29.1 | 10.0 | 6.5 | 6.8 | 12.4 |

Grew more than 20%: the bench rounds (+51%: more rows in the three benches) and the remainder (product build plus
bench builds and counter runs, which the log does not time separately). CI-INSTR-REBUILDS (landed) found the release
libraries rebuilt on most tips because the build tag hashed every file; its fix (product inputs only) shows in the
11-23 week's lower `so/total`. Host contention: ci-instr has its own role on its host; not measured here.

## 3. Agent legs

20 legs per period, chosen by a fixed seed from legs with a result record (`legs.py`). Per leg: wall and model time
from the result record (exact); each tool call's time from its result's timestamp minus the previous one, less the
leg's mean model time per turn; a Bash call naming `jobwait`, `vm_wait`, `vm_affected`, `finish.sh`, `plant.sh` or
`sleep` is a job wait, one running cargo/xtask/rsync/sync outside `jobrun` is a build outside jobs.

| | 10-19 week (20) | 11-20 on (20) | 11-20 on (60) |
|---|---|---|---|
| wall | 13.8 / 26.0 / 40.9 | 27.7 / 63.2 / 91.5 | 25.6 / 58.5 / 107.7 |
| model time | 3.5 / 5.3 / 8.0 (19%) | 5.6 / 8.5 / 10.7 (12%) | 5.5 / 8.8 / 10.9 (12%) |
| job waits | 2.3 / 7.4 / 25.2 (57%) | 10.9 / 46.5 / 76.4 (76%) | 10.5 / 41.5 / 90.2 (78%) |
| of them naming a plants job | 0 / 1.2 / 7.1 (22%) | 0 / 18.9 / 42.2 (35%) | 0 / 11.6 / 33.2 (30%) |
| builds/tests outside jobs | 0.2 / 1.2 / 4.6 (13%) | 0.4 / 2.3 / 5.6 (5%) | 0.3 / 1.5 / 3.1 (4%) |
| other tools | 1.2 / 2.4 / 5.0 (11%) | 2.6 / 4.3 / 5.5 (7%) | 2.3 / 4.0 / 5.6 (7%) |
| tool calls | 33 / 46 / 48 | 67 / 78 / 108 | 66 / 81 / 102 |

Of the median leg's growth (+37 min), job waits are +39 min, model time +3 min, builds outside jobs +1 min (other
tools +2). So nearly all of it is waiting on jobs (gate, plants, counts), and a third of a recent leg is plants
waits. A wait naming several jobs counts as plants when one of them is a plants job, so the plants share is an upper
bound.

## 4. Fix (this branch)

`xtask/src/so.rs` `EarlyData`: the charmaps, locale charmaps and locale sources (no built product is an input) are
written by a thread from the start of `cargo xtask so`, beside the library builds, and joined before the placement
check, which then sees the same files; their log lines print at the join in the old order; `step: so/early-data`
gives the thread's own time. It shortens every product build's critical path: the cold copy build (where the thread
also overlaps the 92 s early-builds wait), every mutant rebuild, the CI products step and the release build.

Before / after (job `pipeline-test-time-1127-ab`, Zen 1-48, nothing else of mine running; fresh RAM copies built
the plants way: source only, `NIJI_LAYOUT_TRACE=ungrouped`, one constant plants tag; before = origin/next af6f8acaf,
after = this branch; alternated, two rounds; seconds of the whole command, `so/total` in brackets):

| build | before r1 | before r2 | after r1 | after r2 | change |
|---|---|---|---|---|---|
| cold copy | 273 (225.5) | 274 (226.9) | 254 (206.8) | 255 (207.0) | -19 s (-7%) |
| no-op, same copy | 64 (63.6) | 63 (62.4) | 52 (51.3) | 51 (50.6) | -12 s (-19%) |
| `charmaps-placement` phase, cold / no-op | 23.3 / 10.7 | | 0.4 / 0.4 | 0.4 / 0.3 | off the critical path |
| `early-data` (the thread, beside the builds), cold / no-op | | | 29.9 / 10.4 | | |

No row got slower; `layout` (11.4-11.6 s) and `early-builds-wait` (77.0-77.1 s cold) are unchanged. Products: the
1686 files of `target/niji` of round 1's before and after copies hashed (sha256): the 43 charmaps, 10 locale
charmaps and 357 locale sources are identical, and so is every other file except `.products-manifest` (it records
what the set was built from, which includes the changed xtask). Per plants job that is ~19 s on the cold baseline
plus ~12 s per mutant build (a mutant build is the same post-cargo assembly as the no-op plus its compile): about
1 min of a median 17-31 min job; per train ~25 s (the products step and the release build). Small, measured, and no
coverage change; the large items are in section 5.

Coverage: the change selects no tests. Plants (`handoff/PIPELINE-TEST-TIME-1127-plants.txt`, job
`pipeline-test-time-1127-plants-2`, 3 of 3 CAUGHT, no WARN): each of the three outputs dropped from the thread stops
the mutant's own `cargo xtask so` at the D-X.164 check of every shipped file (`43 / 10 / 357 shipped objects lack the
SHSTK property: charmaps/... No such file`), before `deb_manifest` runs (the tool says "caught by no test the log
names": the product build caught it). The first run (`pipeline-test-time-1127-plants`) caught the same three; one
WARN-NO-MSG there was my charmaps regex naming only the test's message, fixed in plants-2.

Gate (direct tier; `cargo xtask affected` says the change is xtask's bounded set, FULL is the train's):
`pipeline-test-time-1127-gate` rc 0, 8 suites, 773 passed, 0 failed, peak 4.9G. Lint: workspace clippy
(`--all-features -D warnings`) and `cargo xtask static` clean (`pipeline-test-time-1127-lint`).

## 5. Recommended, not done (they change what runs, where, or a rule; the owner decides)

| item | saved | what it could stop catching | recommendation |
|---|---|---|---|
| Plants: keep one warm copy per role (fixed path, cargo cache kept; the role's source synced in by content with a fresh mtime on every changed file; products dir cleared; one run at a time per copy, else a new cold copy) | the cold baseline: median 14.5 (Zen 1-60), 6.8 (Zen 1-48), 8.8 (SKX) min per job, ~45% of a plants job, replaced by ~1.3 min (79 s measured); also the cold compile of each baseline command | a stale build if cargo's mtime check is fooled (mitigated by the content sync); changes the "new RAM dir per run" rule and holds RAM per role | RULING NEEDED: yes, as its own story with a plant that leaves a mutant in the warm copy |
| Plants `--copies auto`: count the slot's CPUs, not only RAM | on a 4-core slot two cold copies take 14.5 min each against 10.2 for one; the split pays only past ~6 plants | nothing (same plants, same commands) | measure on the Zen 1-60 slots, then pick by CPUs and plants |
| CI gate: run the release-profile start-up check (`NIJI_FULL_PROFILE=1 cargo xtask so` + `hot_text_products`) in the ci-instr job, which builds the release products for the same tip already, or beside the full gate | ~7 min per train on the gate's critical path (425 s median) | the check would run on the 60-vCPU Zen 1 host instead of the 48-CPU one (same CPU class); a red would come from the instr job | coordinator/owner: yes |
| `cargo xtask so`: skip re-assembling products whose inputs (cargo outputs, tables, the xtask binary) are unchanged | 80-110 s of every product build (no-op 112 s; every mutant, CI products step, agent builds) | a stale product if the key misses an input (a cache: SLOW AND WORKING WINS) | RULING NEEDED before a story; key on content hashes only |
| ci-instr: step lines for `cargo xtask bench` (build, counters, rounds) | nothing directly; 12.4 min of the job are not split today | nothing | yes (tooling) |
| niji-locale::reference (1332 test-s, 24 tests) and xtask unit tests (660 tests, 1526 test-s) | up to ~1.5 min of the full gate's wall if sharded or trimmed | depends on the cut | TEST-REDUNDANCY-AUDIT follow-up; owner decides any cut |

## Probe-settled behaviour

None (no libc behaviour is involved).

## Refusals, leftovers

None refused. The 10-19 job logs on the build hosts lack per-phase lines, so the 10-19 split rests on job wall time
and plant counts; the per-plant logs of every past plants run were pruned. Git history: commit subjects only, to date
the step (rules 11-27 allow history to find a regression).
