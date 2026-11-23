# FORCED-UNWIND-REF-RACE: the C++ forced-unwind program could end in std::terminate

State: branch `task/forced-unwind-ref-race`, cut from origin/next 43058b7df. Commits (all mine): the test-program fix
(`crates/core/tests/oracle/forced_unwind.cc`), the plants file, this note and the report. Exact head: see
`handoff/reports/FORCED-UNWIND-REF-RACE.json` and the hand-back.

## Cause (settled by disassembly of the built program and a reproduction)

CI train 880's red was the `cancel-async` case: `cancel-join` is the last line printed before
"terminate called without an active exception" (CI log line 793/796). That case enabled asynchronous cancellation
and spun `for (;;) pthread_testcancel();` inside `t_cancel`, a frame with a try/catch and a `Guard` destructor
(an LSDA; CIE augmentation "zPLR"). The built loop is `call pthread_testcancel@plt; jmp <call>`. A library that acts
on the request at once can interrupt the `jmp` (or the `call` before it executes). The unwinder then looks up that
exact address in `t_cancel`'s call-site table, finds no entry, and the C++ personality routine calls std::terminate
(Itanium C++ ABI behaviour for an address outside the table). Whether it happens depends on where the signal lands.
libniji acts on asynchronous requests at the next safe point outside libniji code (D-22.11, differences row
`s22_3_c_2`), here inside `pthread_testcancel`, so `t_cancel` is always left at a call site and the libniji run
never had the window.

Other candidates checked and ruled out: main ends with `pthread_cancel(blocker)` + `pthread_exit` (the blocker's
frames have no handlers, and main's `MainGuard` runs on main's own unwind); no destructor throws; the cases with
deferred cancellation (0-6) act only inside the library at call sites; the C program (`forced_unwind.c`) has no
asynchronous case.

## Fix

`setcanceltype(ASYNCHRONOUS)` and the `pthread_testcancel` loop moved into `async_spin()`,
`__attribute__((noinline, noclone))`, with nothing to clean up and no handler. Checked on the built program: its FDE
uses the plain "zR" CIE (no personality, no LSDA), and `t_cancel` calls it (`call async_spin`). So `t_cancel` is
left at that call instruction, which its table covers. Expected output unchanged
(`cancel-async: dtors=1 caught_fu=1 ...`): the destructor and the `catch (abi::__forced_unwind &)` still run. The
same source is built dynamically, `-static` and `-static-libgcc` by the test, so the fix covers all three.

## Reproduction (keke, Zen 1 host, 60 vCPU, host load ~21 plus 6 busy loops; script in the run's scratch)

| | runs of each | reference aborts | libniji aborts |
|---|---|---|---|
| before (origin/next source) | 2400 (8 workers x 300) | 4 (all cancel-async, "terminate called without an active exception") | 0 |
| after (this branch) | 8000 (8 workers x 1000) | 0 | 0 |

At the "before" rate (4/2400), 8000 runs would be expected to give about 13 aborts. Jobs:
`forced-unwind-ref-race-before`, `forced-unwind-ref-race-after` (diagnostic loops; the "before" one ran the old
source).

## Plants and gate

`handoff/FORCED-UNWIND-REF-RACE-plants.txt`: the 8 #25 plants of THREAD-FORCED-UNWIND-3 (3) and -STATIC-2 (5)
rerun on this head; results in `handoff/FORCED-UNWIND-REF-RACE-report.md`.

## Open questions / not to redo

- No ruling needed: test-program change only, no libniji code, no record.
- Don't replace the loop with a pure spin (no calls): libniji would never act (D-22.11) and the case would hang.

Next step: none beyond landing. Plan rows: D-22.11, D-22.19 (plan/07-decisions/s22-threads.md).
