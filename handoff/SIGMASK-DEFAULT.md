# SIGMASK-DEFAULT: `sigprocmask` / `pthread_sigmask` default versions against the reference

State: branch `task/sigmask-default` (from origin/next 30d07e74b). Code commits b0fc4d03e (fixes, test, docs),
the watchdog fix after it, 45b5058e4 (reference probe output, plants file); this note and the report json on top.

## What changed

* `signal_compat::sigprocmask` (crates/legacy), shared by `sigprocmask`, `pthread_sigmask@@GLIBC_2.32` and its
  `GLIBC_2.2.5` alias:
  1. With a null `set`, `how` is not significant (POSIX `pthread_sigmask`): any value, 3 and -1 included,
     returns 0 and fills the old set. It returned `EINVAL` before.
  2. It returns the kernel's 64-signal word (32 removed); the entry points (crates/boundary s20_signals.rs,
     `report_mask`) write only `__val[0]` of the caller's old set. Before, all 128 bytes were written (zeros after
     the first 8). Probe: the reference writes 8 bytes and leaves the other 120 as they were (a 0xa5-filled set
     keeps its 0xa5 bytes, "+0" on all 280 cases).
* The probe `crates/core/tests/oracle/pthread_sigmask_compat.c` runs its 140 cases through `sigprocmask` too.
  `old_pthread_sigmask_binds_and_matches_the_reference` (crt_products.rs) has no masks left: per case it
  requires the reference's case, return value, errno, the "+N" byte count and suffix, and asserts D-20.1 for 32
  and 33: libniji's old set = the reference's without 32; its mask afterwards = the reference's on every bit but
  33, and 33 follows the caller's set (BLOCK/UNBLOCK/SETMASK); a caller's set never blocks 32. 280 cases.
* `conformance/c/s20_internal_signal.c` (clause tests `s20_1_c_2`, `s20_2_f_0`, `s20_3_g_0`): main and two
  threads block every signal (all-bits `pthread_sigmask` SETMASK plus `sigfillset` `sigprocmask`); an all-bits set
  blocks 33, never 32; one thread is cancelled in `read`; `setuid(getuid())` completes with the other waiting; a
  watchdog thread ends the run after 120 s if either hangs (sleep remainder loop: the setuid replay's 32 reaches
  it too and ended a plain `sleep` early, the first run's false FAIL).
* docs/differences-from-glibc.md: new row "Reserved real-time signals" (one reserved signal, SIGRTMIN 33, 33
  blockable, 32 never shown; no compatibility option, reason OD-8 recorded in the row). It was missing.
* spec/behaviour/pthread-sigmask-compat.md and .glibc.txt (the reference's full probe text, 282 lines).

## Decisions in force

* D-20.1 (one reserved signal 32; "no reported set ... contains it") decides item 4: the hidden 32 in the
  old set is D-20.1's behaviour, now asserted in the test. OD-8 (owner, 2026-10-04).
* No new decision record: fixes 1 and 2 follow POSIX and the probe (no deviation).

## Proof

* `sigmask-default-t1` (mei): clippy, static, so, crt_products old_pthread_sigmask ok; s20_signals_clauses 3 FAILED
  (the watchdog false hang, fixed) - SUPERSEDED. `sigmask-default-t2`: s20_signals_clauses 41 ok, crt_products
  old_pthread_sigmask ok, reference probe text captured.
* Gate `sigmask-default-gate` (mei, direct tier, --base origin/next, code head 45b5058e4): rc 0, affected ok,
  conformance 3 passed, peak 13G. Plants `sigmask-default-plants` (handoff/SIGMASK-DEFAULT-plants.txt, RAM dir
  plants-1): 4 plants, 3 CAUGHT, 0 SURVIVED, 1 WARN-NO-MSG: `caller-set-blocks-32` was caught (rc 101), but the
  conformance harness's panic shows only "status 1", not the program's lines; the program's stdout in the plant
  copy shows the new checks failing: "FAIL an all-bits set blocks 33 ..., never 32" and the hang guard
  ("cancel or setuid did not complete"), i.e. the thread test catches a blockable 32.
* Plants for fixes 1 and 2 (`null-set-checks-how`, `old-set-whole-128-bytes`) and D-20.1's hidden 32
  (`old-set-shows-32`): CAUGHT by old_pthread_sigmask_binds_and_matches_the_reference.

## Open / informational

* The differences row offers no compatibility option (reserving a second signal is process-wide from start-up;
  OD-8). Recommend none; the coordinator may ask the owner if a COMPAT is wanted.
* Not to redo: the reference's 8-byte write / untouched tail is settled by the probe (.glibc.txt).
* Rule slips: two no-op shell lines used an empty heredoc and an empty here-string (no file was written by
  them; every file came from the Write/Edit tools or a python script reading files).

## Next step

None for this story once the jobs are green.
