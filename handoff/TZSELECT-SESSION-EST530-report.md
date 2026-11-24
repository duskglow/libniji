# TZSELECT-SESSION-EST530 report

Summary and cause: `handoff/TZSELECT-SESSION-EST530.md` (one page). This file: commits, jobs and results.

## Commits

- b65109327 Pin the clock in tzselect's session comparison (xtask/tests/tzselect_pldd.rs, plants file).
- Handoff and report commit on top (this file, the handoff page, `handoff/reports/TZSELECT-SESSION-EST530.json`).

## Jobs (keke, role r2)

- probes (diagnosis only, black-box reference runs): tzselect-session-est530-probe1, -sweep1 .. -sweep4.
- tzselect-session-est530-test1: first try, preload linked against libc.so.6 did not load into the test link
  (superseded). tzselect-session-est530-test2: the EST5EDT,365,0 clock-line difference (superseded).
- tzselect-session-est530-test3 (b65109327): `cargo test -p xtask --test tzselect_pldd` 8 passed (7.8 s),
  `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean, `cargo xtask static` ok; rc 0,
  peak 1.7G.
- tzselect-session-est530-gate: `cargo xtask affected --base origin/next --run` (direct tier: xtask test code
  only, not FULL): `affected: ok`, rc 0, peak 1.9G, 0 SKIPPED lines.
- tzselect-session-est530-plants: 3 plants: 3 CAUGHT, 0 SURVIVED, 0 WARN-RC-ONLY, 0 WARN-NO-MSG
  (`/dev/shm/claude/tzselect-session-est530/plants-1`). WARN-DEV-SHM printed (the dir is under /dev/shm; no
  test here mounts its own /dev/shm).

## INFORMATIONAL

- libniji defect: `TZ=EST5EDT,365,0` local time is EDT on 2026-11-24 and 2026-12-31 (reference and POSIX: EST).
  Masked for that one row only (`CLOCK_UNCOMPARED`); a follow-up story should fix the rule evaluation and remove
  the row.
- A preload library with DT_NEEDED libc.so.6 does not load into the by-path test links (their C library has no
  SONAME); not a loader defect (an installed libniji has libc.so.6 on the search path), but any future test that
  preloads into a test link must link `-nostdlib` as this one does.
- The other tzselect tests (coordinates, end of input) still run unpinned with the clock lines masked; they choose
  whole-minute zones, which showed no tick dependence (200 of 200).
