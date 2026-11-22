# TIMED-WAIT-PAST-DEADLINE: timed waits return on a past deadline, never loop

State: branch `task/timed-wait-past-deadline`, code commit 4c81d0500 (on origin/next fa79409c7); this note and
`handoff/reports/TIMED-WAIT-PAST-DEADLINE.json` in the commit after it. Role mei:~/w/s4.
Record: `records/pending/TIMED-WAIT-PAST-DEADLINE-1.toml` (D-NEW, series D-22; CI assigns the id).

## The timed waits (step 1)

Every wait that reaches `futex::wait_until`, `Ctx::wait` or the kernel's PI lock:
- `pthread_mutex_timedlock` / `clocklock` and `mtx_timedlock`: `Mutex::lock_plain` (normal, recursive, errorcheck,
  adaptive, pshared), `lock_pp`, `lock_robust`, `lock_pi` (kernel `FUTEX_LOCK_PI`/`LOCK_PI2`, retries only on
  EINTR/EAGAIN: never looped).
- `pthread_cond_timedwait` / `clockwait`, `cnd_timedwait`, `pthread_cond_timedwait@GLIBC_2.2.5`: `Cond::wait`.
- `pthread_rwlock_timedrdlock` / `clockrdlock` / `timedwrlock` / `clockwrlock`: `RwLock::wait_change`.
- `sem_timedwait` / `sem_clockwait`: `Sem::wait`.
- `pthread_timedjoin_np` / `pthread_clockjoin_np`: `thread::wait_tid`.
- Not affected: barriers (no timed form); `aio_suspend` and `gai_suspend` build their deadline from the monotonic
  clock plus a relative timeout clamped at 0, so it is never negative.

## The fix (step 3)

- `futex::past(at)`: `tv_sec < 0` is before every reading of both clocks. `futex::wait_until` returns ETIMEDOUT
  for it without the syscall. The cancellable path (`ThreadCtx::wait`, boundary `sync.rs`) waits until {0, 0}
  instead, so a due cancellation request is still acted on; the kernel then returns ETIMEDOUT at once.
- `futex::woken(r)` sorts every loop's result: Ok, EAGAIN, EINTR = wake-up (re-check); any other error ends the
  call with that error (ETIMEDOUT, ECANCELED, and kernel rejections EINVAL, EFAULT, ENOSYS/EPERM from a filter).
  The semaphore keeps its own match (EINTR ends `sem_wait`): Ok/EAGAIN retry, every error ends.
- No fast path changed: the checks sit after the uncontended attempt, beside the syscall.

## Probe (step 2; black box, reference glibc 2.39 on mei vs libniji, static link)

`conformance/c/s22_past_deadline.c --probe`: 38 call forms x 7 deadlines ({-1,0}, {LONG_MIN,0}, {1,0},
{-1,999999999}, {0,0}, {-1,1e9}, {-1,-1}) x contended/free, 476 cells, each in its own child (3 s guard).
- Reference, valid past deadlines: contended ETIMEDOUT (C11 thrd_timedout), free 0, every form, except the PI mutex
  contended with a negative tv_sec: EINVAL. Cond waits: ETIMEDOUT.
- libniji before: 108 cells never returned (all forms but PI with a negative tv_sec). After: 0; every valid-deadline
  cell equals the reference.
- Still different (pre-existing, not changed; see RULING NEEDED): 46 cells with an invalid tv_nsec beside a
  negative tv_sec (reference: plain mutex and mtx contended ETIMEDOUT, join ETIMEDOUT/0 before the tv_nsec check;
  robust/PI free 0; sem free EINVAL; libniji EINVAL first, or takes the free semaphore), and 24 prio-protect cells:
  an unprivileged caller cannot lock a PP mutex from the reference (EINVAL, ceiling = min FIFO priority), libniji
  takes it.

## Tests (step 4)

- `conformance/c/s22_past_deadline.c` (clause s22_4_c_5 in `crates/core/tests/s22_threads_clauses.rs`): 136 cells,
  every form contended with {-1,0}, {LONG_MIN,0}, {1,0}, plus free with {-1,0}; 30 s alarm per cell fails it as
  "blocks". Passes on the reference (gcc -pthread, 0 failed) and libniji.
- `crates/rt/tests/pthread_sync.rs`: `a_wait_the_kernel_refuses_ends_the_call_with_its_error` (a Ctx whose every
  wait returns EINVAL/ENOSYS/EFAULT, a 1000-wait budget panics on a retry loop; mutex x4 kinds, cond, rwlock rd/wr,
  sem; Block and both clocks) and `a_deadline_with_a_negative_second_has_passed`.

## Jobs (mei)

See the report JSON (`jobs`) and the hand-back. Plants: `handoff/TIMED-WAIT-PAST-DEADLINE-plants.txt` (6).

## Open / next

- RULING NEEDED: invalid tv_nsec with a negative tv_sec (46 cells above): match the reference's ordering, or keep
  EINVAL (POSIX lists EINVAL for a bad tv_nsec when the call would block). Recommend a separate story; no change here.
- INFORMATIONAL: prio-protect mutex locks succeed in libniji where the reference refuses an unprivileged caller.
- Do not redo: the probe matrix (scripts and outputs in mei:~/work/timed-wait-past-deadline/).
