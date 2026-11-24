# TZSELECT-SESSION-EST530: tzselect's custom-TZ session diverged from the reference in CI

State: branch `task/tzselect-session-est530`, cut from origin/next c289b63c0 (nothing to merge: the tzselect tests
are still in `xtask/tests/tzselect_pldd.rs`). Code commit b65109327 (test only; no library or utility change).

## Cause (black-box probes of the reference, keke = Zen 1 host, 60 vCPU)

- A session is started at a chosen phase of the second (busy-wait on `date +%N`), input `11 / <TZ> / UTC0 / 1`.
- EST5:30:59: the clock block ("Selected time is now:" / "Universal Time is now:") is normally absent; it appears
  in 2 of 100 sessions over a 10 ms sweep of the whole second (phases 890 and 900 ms) and in 34 of 200 over a 1 ms
  sweep of 850-950 ms (hits 880-911 ms). Every time it appears, the two lines' seconds are equal (zone line one
  second behind the consistent value): a second ended between the reference's clock reads. A session takes ~115 ms.
- EST5 (whole-minute offset): block shown 200 of 200 in the same window, 100 of 100 over the second.
- Probe at phase 0-100 ms and 970-1000 ms only (20 runs each): never shown for EST5:30:59, EST5:30:01,
  EST-24:59:59, EST5:30:30.
- libniji's rule (csys/util/tzselect.c: no clock lines when `tm_gmtoff % 60 != 0`) is the reference's no-tick
  behaviour. The test compared against a reference whose output depends on sub-second timing, hence the CI flake.
  Not chased further into the reference (PURPOSE rule): a stopped clock makes it deterministic.

## Fix (test only)

- `CLOCK_PIN_C`: an LD_PRELOAD library built by the test (`cc -shared -fPIC -O2 -nostdlib`) that stops
  CLOCK_REALTIME (`time`, `clock_gettime` REALTIME/REALTIME_COARSE, `gettimeofday`, `timespec_get`) at
  `NIJI_TEST_CLOCK`; other clocks go to the kernel. `-nostdlib` because the test links name their C library by path
  (`target/niji-test/libc-by-path.so`, no SONAME): a preload with DT_NEEDED libc.so.6 does not load there
  ("libc.so.6: cannot open shared object file"); with no DT_NEEDED its getenv/syscall bind to the process's libc.
- `PINS`: 1795543500 (2026-11-24T18:05:00Z, the CI run), 1795546800 (19:00:00Z, hour boundary), 1798761599
  (2026-12-31T23:59:59Z, year boundary). Every session at the first; the TZ-string sessions at all three
  (146 session pairs, the test takes ~8 s).
- Pinned sessions compare the clock lines exactly (they were masked); a check that the reference's UTC line is the
  pinned one proves the pin reaches its helper programs. The other tests keep the unpinned, masked `session`.
- `CLOCK_UNCOMPARED = ["EST5EDT,365,0"]`: the one row whose clock lines stay masked, because comparing them shows
  a libniji defect (below).

## Found: libniji defect (follow-up story needed)

`TZ=EST5EDT,365,0` (DST from zero-based day 365 to day 0): libniji's localtime gives EDT at 2026-11-24T18:05Z,
19:00Z and 2026-12-31T23:59:59Z; the reference gives EST at all three. POSIX: `n` counts leap days, so in a common
year day 365 is the next year's January 1, start after end, DST only before Jan 1 02:00 and from the start: EST in
November is right. `crates/core/src/time/rule.rs` `day_of` maps day 365 that way but has no callers; the evaluation
localtime uses lives elsewhere (not located: budget). Next story: find it, fix it, then drop the row from
`CLOCK_UNCOMPARED`.

## Plants (`handoff/TZSELECT-SESSION-EST530-plants.txt`)

clock-pin-dropped (test without LD_PRELOAD: "clock is not pinned"), seconds-offset-shows-clock (libniji shows
the lines for EST5:30:59), clock-line-a-second-late (libniji's zone line one second off: caught only because the
lines are now compared). Results in the report file.

## Leg 1 end state (REFRESH at 19:40Z, 81 calls)

Proven on 2a8d2fe2d (code b65109327, base c289b63c0): keke jobs tzselect-session-est530-test3 (8 passed, clippy,
static), -gate (direct tier ok), -plants (3 CAUGHT), -precheck (GREEN); handback-check ok. BUT origin/next
d3e273eb5 landed XTASK-TESTS-TZSELECT-FOLD: `xtask/tests/tzselect_pldd.rs` is deleted on next, its tests are in
`xtask/tests/tz_tools.rs` (modify/delete conflict). Those jobs are SUPERSEDED once ported.

EXACT NEXT STEP: `git merge origin/next` (brief allows it); resolve by taking next's deletion of tzselect_pldd.rs
(`git rm xtask/tests/tzselect_pldd.rs`) and re-applying this branch's diff (`git diff c289b63c0 b65109327 --
xtask/tests/tzselect_pldd.rs`) into tz_tools.rs (CLOCK_PIN_C, clock_pin, session_at, mask_clock,
CLOCK_UNCOMPARED, PINS, the pinned loop in tzselect_sessions_match_the_reference, the module-doc sentence; check
for name clashes with tz_tools' own helpers). Change the plants file's path and `--test tzselect_pldd` to
`tz_tools`. Then sync, test (`cargo test -p xtask --test tz_tools` + clippy + static in one job), gate, plants
(new dir plants-2), precheck as a jobrun job (it takes ~8 min: remez checks), update report/json, handback-check.

## Not to redo

The probe scripts (keke `~/work/tzselect-session-est530/{probe,sweep,dbg}.sh`) settled the cause; no faketime on
keke. Next step: the EST5EDT,365,0 localtime story.

Full report: `handoff/TZSELECT-SESSION-EST530-report.md`.
