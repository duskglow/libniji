# FORCED-UNWIND-REF-RACE report

Branch `task/forced-unwind-ref-race` (from origin/next 43058b7df). One-page state and cause: `handoff/FORCED-UNWIND-REF-RACE.md`.

## Commits (all mine)

1. `Keep asynchronous cancellation out of frames with handlers in forced_unwind.cc`: the fix (test program only).
2. `Add the plants file for FORCED-UNWIND-REF-RACE`: the 8 #25 plants rerun.
3. `Add a skip-unwind plant that reaches the forced-unwind tests`: `handoff/FORCED-UNWIND-REF-RACE-plants-2.txt`.
4. Handoff note, this report, `handoff/reports/FORCED-UNWIND-REF-RACE.json`.

## Results (keke, Zen 1 host, 60 vCPU)

- Reproduction (diagnostic loops, 6 busy loops beside 8 workers, host load ~21):
  before 2400 runs each: reference 4 aborts (all `cancel-async`, "terminate called without an active exception",
  SIGABRT), libniji 0. After 8000 runs each: reference 0, libniji 0. Jobs `forced-unwind-ref-race-before`
  (old source) and `forced-unwind-ref-race-after` (this branch's source; products copied to RAM).
- Gate `forced-unwind-ref-race-gate` (direct tier, `cargo xtask affected --run --base origin/next`): 1 suite,
  niji::forced_unwind_products 5 passed, 0 failed, rc 0.
- clippy (`cargo clippy --workspace --all-targets --all-features -- -D warnings`) and `cargo xtask static`: clean.
- Plants `forced-unwind-ref-race-plants` (8 #25 plants of THREAD-FORCED-UNWIND-3 and -STATIC-2): 7 CAUGHT,
  0 SURVIVED, 1 WARN-NO-MSG:
  - WARN-NO-MSG `skip-unwind`: the mutant stops at the product build (`xtask so: ... no member refers to
    niji_machine_unwind_force`, the check STATIC-2 added), so it never reaches the tests. It is not caused by this
    branch (origin/next has the same check). Covered by the new variant `skip-unwind-kept` (below).
  - `static-entry-zeroed`: CAUGHT by the plants tool's 600 s timeout. The four other tests passed and
    `static_programs_unwind_with_the_unwinder_they_link` was still running: the mutant's static C++ program hangs
    until the per-run watchdog, which `support/timeout.rs` scales with the load average, so it outlived 600 s.
    In STATIC-2's run the same plant failed with its message. A hang still fails the test.
- Plants `forced-unwind-ref-race-plants2` (`skip-unwind-kept`: keeps the reference, never calls the unwind):
  1 CAUGHT (rc 101, `(cxx_frames_unwind|c_cleanups_run).*(FAILED|panicked)` in the log), 0 WARN.

## Findings

- INFORMATIONAL: libniji's run has no window: asynchronous requests act at the next safe point (D-22.11), here
  inside `pthread_testcancel`, so the frame with handlers is always left at a call site. No difference beyond the
  documented one (differences row `s22_3_c_2`).
- INFORMATIONAL: the `skip-unwind` plant of THREAD-FORCED-UNWIND-3 is now stopped by the product build check.
  Plant files that reuse it should use `skip-unwind-kept`.
- No RULING NEEDED. No refusals.

## Leftovers

Nothing running. RAM scratch on keke under /dev/shm/claude/forced-unwind-ref-race (plants dirs kept, as the rules say).
