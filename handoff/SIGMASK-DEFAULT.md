# SIGMASK-DEFAULT: `sigprocmask` / `pthread_sigmask` default versions against the reference

State: branch `task/sigmask-default-2` (SIGMASK-DEFAULT-2, continuing `task/sigmask-default`, base origin/next
30d07e74b). SIGMASK-DEFAULT's commits: b0fc4d03e (fixes, test, docs), the watchdog fix, 45b5058e4 (probe output,
plants), 3e6a4f0c9 (handoff). SIGMASK-DEFAULT-2: a44697810 (s18_spawn_vm test fix and plants), this note and
handoff/reports/SIGMASK-DEFAULT-2.json on top.

## What changed (SIGMASK-DEFAULT)

* `signal_compat::sigprocmask` (crates/legacy), shared by `sigprocmask`, `pthread_sigmask@@GLIBC_2.32` and its
  `GLIBC_2.2.5` alias: (1) with a null `set`, `how` is not significant (POSIX): any value returns 0 and fills the
  old set (was EINVAL); (2) only `__val[0]` (the kernel's 64 signals, 32 removed) of the caller's old set is
  written, the other 120 bytes keep their contents, as the reference does (probe: spec/behaviour/
  pthread-sigmask-compat.glibc.txt).
* Differential test `old_pthread_sigmask_binds_and_matches_the_reference` (crt_products.rs), 280 cases, no masks;
  asserts D-20.1 for 32/33. Conformance `s20_internal_signal.c`: threads that block every signal, cancel + setuid.
* docs/differences-from-glibc.md: row "Reserved real-time signals" (no compatibility option, OD-8).

## SIGMASK-DEFAULT-2: the CI red (s18_spawn_vm line 93)

* Cause, proven by a diagnostic run (job `sigmask-default-2-diag` on keke, the program printing each differing
  word): word 0 of the mask was identical before and after the spawns; words 1..9 differed. `before` and `after`
  were uninitialised `sigset_t`s compared with memcmp over 128 bytes; since fix (2) the query no longer
  overwrites the tail, so the test read indeterminate stack bytes. Not a mask round-trip defect.
* Every internal mask save/restore was checked; none calls the public functions, all use the raw kernel call
  (`Sys::sigprocmask`, full 64-bit word) or the syscall directly: posix_spawn (spawn.rs `spawn`: block-all and
  restore), system/popen (spawn.rs `run_shell`: SIGCHLD block and restore), thread start (thread.rs: block-all
  around clone, the new thread's mask from its descriptor), process.rs (child mask), sigsetjmp/siglongjmp
  (setjmp.S), getcontext/setcontext/swapcontext (context.S), abort (rt fatal.rs: unblock only), sighold/
  sigrelse/sigpause/sigblock/sigsetmask (signal_compat, same raw call). Cancellation and the set*id broadcast
  (cancel.rs, setxid.rs, fork.rs) do no mask save/restore.
* Fix (a44697810, then the follow-up commit): `before`/`after` start zeroed; the window sets the whole mask
  (SIG_SETMASK: SIGUSR2, SIGRTMIN, SIGRTMAX) and runs `system("exit 3")` too, and the mask on entry is compared
  with the one main saw before any spawn. The first version used SIG_BLOCK, and its plant spawn-restore-missing
  SURVIVED: the failing spawns before the window had already left everything blocked, so before == after (strace
  of the mutant binary showed the all-blocked mask from the first spawn on). Setting the mask whole fixes that.
* Plants: handoff/SIGMASK-DEFAULT-2-plants.txt (SIGMASK-DEFAULT's four plus spawn-restore-missing,
  spawn-restore-partial (drops signals 33..64 on restore), system-restore-missing).

## Proof (SIGMASK-DEFAULT-2, keke ~/w/r1)

* Inherited head 3e6a4f0c9: clippy and static clean; s18_spawn_vm reproduced the CI failure (job
  `sigmask-default-2-lint`, rc 1 by design). First fix: `sigmask-default-2-s18` rc 0; its gate and plants
  (`sigmask-default-2-gate`, `sigmask-default-2-plants`, head 4ae08cc71) were stopped after the surviving plant
  (4 CAUGHT, spawn-restore-missing SURVIVED) and are superseded.
* Final head: `sigmask-default-2-gate2` (clippy, static, `cargo xtask test`: FULL, runs every conformance
  program, crt_products and s20 clauses) and `sigmask-default-2-plants2` (/dev/shm/claude/SIGMASK-DEFAULT-2/plants-2).
  Results in the hand-back.

## Decisions in force

* D-20.1 (one reserved signal 32; no reported set contains it) decides item 4. OD-8 (owner). No new record.

## Not to redo

* The reference's 8-byte write / untouched tail is settled by the probe. The internal-caller audit above.

## Next step

None once the gate and plants are green.
