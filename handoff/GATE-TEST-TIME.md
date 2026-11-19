# GATE-TEST-TIME: why the CI gate's test steps got 2.3-3.7x slower (11-12 -> 11-19), and two fixes

State: branch `task/gate-test-time` from origin/next 255a6901d; HEAD: see handoff/reports/GATE-TEST-TIME.json.
Code commits: 170d1e7d5 (headers-gen row index), 53918b5f3 (run-once graph: dev-dependencies). Decisions: none
new (local tooling details; no D-row). Plants: handoff/GATE-TEST-TIME-plants.txt.

## Method (deviation from the brief, step 1)
The brief asked for both nextest suites re-run at b31418181 and at next's head on a quiet role. I did not: that is
two FULL test runs (forbidden: "never a FULL gate"), and a single pair would not separate the effect that turned
out to dominate (the verdict cache, below). Instead: the per-test `PASS [ s]` lines of the last 160 green CI gates
on the CI host (Zen 1, 48 threads; `~/logs/ci-gate-*.affected.log`), parsed with scratch scripts (copies in
the CI host's ~/work/gate-test-time/: gtt.py ranks two gates per test, gtt_series.py gives one line per gate,
load.py lists the jobs that overlapped a gate in jobs.tsv). Crate names were normalised (liella -> niji).

Concurrent load (other jobrun jobs overlapping the gate, average count): b31418181 none (quiet); cc18a11ef 1.1
(another gate); 255a6901d 1.1 (tests, counts, plants). Load explains part of the spread between gates, not the step
changes below (each shows within one gate, as a pass-1/pass-2 ratio, or across quiet and busy gates alike).

## Where the added time is (pass test-seconds; wall = nextest Summary)
| | 11-12 b31418181 | 11-19 255a6901d |
|---|---|---|
| pass 1 wall / test-s / tests | 267 / 4440 / 6483 | 1024 / 14161 / 6847 |
| pass 2 wall / test-s / tests | 323 / 4687 / 5172 | 730 / 10451 / 5330 |
New tests: pass 1 +391 tests = 1374 s; the rest (+8350 s) is tests that already existed.

1. **Math accuracy sweeps re-run in pass 1 (largest; not a regression).** complex_ld_cr_products 145 -> 4393 s,
   complex_f128 131 -> 1743, special_ld 93 -> 800, special_f128 96 -> 785 (~7300 s of +9700). These sweeps sit
   behind the verdict cache (D-10.57): a shard runs only when an input changed (the test binary, the static
   libniji program, the host libraries). b31418181 was a cache-hit train. Over 160 gates pass 1 is bimodal:
   ~300-390 s (cached) or ~630-1480 s (any libniji byte changed), and since 11-13 almost every train misses
   (any libc change rebuilds the static program). Pass-1 wall is throughput-bound (~14 tests in flight), so a
   miss adds ~400-1000 s. Not changed here: the key is correct by design (a cache never serves a stale result).
   Cheaper sweeps belong to a math story (each shard ~200-300 s on the CI host).
2. **Pass 2 (interposed exports) about 2x slower since 11-14 23:45 (train 37a467944).** pass-2 test-s ~4800 ->
   ~9700, wall ~285 -> ~525 s, every train since. Pure-compute tests that start no process slowed in pass 2 only:
   strstr_find pass 2 / pass 1 in the same gate 3.5x (11-12) -> 7.4x (11-19); heap_threads 4x in both passes.
   The step coincides with the TB-FOREIGN-WORDS-LOW landing (tb.rs, heap.rs, tp.S, errno.rs: the changed files
   common to the two trains that show it, a5d3a0d7f and 37a467944); its handoff says a block libniji did not make
   keeps no heap cache ("the heap takes the slow path"). Every nextest test thread is made by the host's
   pthread_create, so in pass 2 every allocation takes that path. NOT VERIFIED by an A/B run (inferred from 160
   gates). Not a test cause: INFORMATIONAL for the heap / TB stories (allocation cost in threads libniji did not
   make).
3. **headers-gen is quadratic (fixed, 170d1e7d5).** `headers-gen --check` 340.9 s -> 4.3 s (Zen 1 build host,
   one CPU, load ~8 of 60). It drove the `headers-gen-check` step (114 s on the CI host) and three tests:
   generator_prechecks_run_the_checks (283 s pass 1, 266 s pass 2: it runs the check twice),
   headers_gen_regenerates_the_abi_layouts (151 / 144 s; now 3.6 s measured), header_p2_fixed_rows (197 s; runs
   `headers-gen --out`, then the C probes). Cause: every name of every header scanned all visibility.tsv rows
   (`cond.rs` rows / dont_care / is_covered, `enums.rs`, one site in headers.rs). Now a (header, name) index and a
   per-header include-error mask built once; the output is byte-identical (--check passes against the committed
   headers).
4. **xtask's unit tests ran twice (fixed, 53918b5f3).** bin/xtask (502 tests, 1846 test-s in pass 2 on 11-19)
   is not run-once: the package-graph walk followed niji-rt's dev-dependency on niji-boundary (xtask -> niji-ld ->
   niji-rt), which cargo never links into xtask; the byte rule cannot help (xtask names the marker in literals).
   The walk now follows dev edges only from the test's own package.
5. Ruled out: lost parallelism (tests in flight per pass ~14-17 at both ends); process start-up as the main
   cause (the slowed tests start no process; spawn-heavy tests are not among the top movers); repeated builds
   inside tests (none found among the top 30).

## Verified (Zen 1 build host, role n3, code head c085514d0)
- Gate `gate-test-time-gate-4` (direct tier = xtask bounded set: xtask unit tests, clippy, static, cheap steps;
  FULL is the train's). Superseded, all green on older heads: `gate-test-time-gate` (53918b5f3, 515 passed),
  `gate-test-time-gate-3` (a29b017e4, 516 passed).
- Plants `gate-test-time-plants-4` (handoff/GATE-TEST-TIME-plants.txt, 8 plants). Superseded:
  `gate-test-time-plants` (2 SURVIVED: include-error union and kind filter, which the committed tables never
  exercise; hence a29b017e4's synthetic-row test; stopped after 5 plants) and `gate-test-time-plants-3` (7 CAUGHT,
  1 SURVIVED: the build clause, which the fixture never decided; hence c085514d0). Two jobs started on a stale
  tree after a BUSY sync (`-gate-2`, `-plants-2`) were stopped before they proved anything.
- `headers-gen --check` equals the committed headers after the change (the strongest check of identical output).

## Expected effect once landed (estimates from the CI logs, measured: false)
- headers-gen-check step 114 s -> ~2 s; pass 1 ~-600 test-s (the three header tests); pass 2 ~-1850 test-s
  (bin/xtask once) = ~-130 s pass-2 wall at ~14 in flight.

## Open / next step
- Measure on the train: compare the next CI gates' `step: test/headers-gen-check` line and the pass-2
  `once: xtask::bin/xtask` line plus Summary against 255a6901d.
- Math sweeps (item 1) and the foreign-thread heap path (item 2): stories of their own (numbers above).
- The `products` step stays with PLANTS-BUILD-SLOWDOWN.
