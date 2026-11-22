# THREAD-EXIT-NO-UNWINDER (handoff, also the full report)

State: branch `task/thread-exit-no-unwinder` (from origin/next 279a24ee4), role keke:~/w/n4. Head: see
`handoff/reports/THREAD-EXIT-NO-UNWINDER.json`.

## Decisions in force
* D-22.23 (records/decisions/D-22/D-22.23.toml, amends D-22.21): when a thread exit (`pthread_exit`,
  `thrd_exit`, acting on cancellation) finds no unwinder, its first step walks the thread's stack with
  libniji's own CFI walker (`niji_rt::unwind::handler_above`, the backtrace walker); a frame whose CIE has a
  personality (`P`) and whose LSDA is non-null (or unreadable, or no `L`) means unwind work: the process
  writes the FATAL message and aborts (SIGABRT). Otherwise the buffers-only exit goes on as before.
  Objects: the executable, libc's image, then the loader's own iterate (`backtrace::frame_object`).
  libniji's own personality frames (`niji_machine_once`, `niji_machine_guard`: `backtrace::own_guards`) are
  skipped: their undo is also an old-style record the buffers-only exit runs. Found by the first gate:
  conformance `s22_cancel` (cancel inside a `pthread_once` routine) aborted with the message before the skip.
* COMPAT-38 (records/compat/COMPAT-38.toml): registered only, restores the exit that skips the unwind work.
* Differences row: docs/differences-from-glibc.md, after the `pthread_rwlock` waiting order row.
* Rulings #7118 (the reference's behaviour may be published), #7119 (failing loudly is better).

## Probe table (keke, Zen 1 host; probe script and none.c in the run's scratch, not in the tree)
Builds: `forced_unwind.c` (`-fexceptions`; buffers = `-fexceptions -U__EXCEPTIONS -fno-inline`; plain =
no `-fexceptions`), a thread with nothing to unwind (`none.c`, plain and `-fexceptions`), all
`-static-libgcc`; `forced_unwind.cc` with `-static-libgcc -static-libstdc++`. "hidden" = libgcc_s.so.1
bind-mounted over by /dev/null in `unshare -r -m`.

| build | reference present | reference hidden | base present/hidden | head present/hidden |
|---|---|---|---|---|
| C -fexceptions | 134, no message | 134, `libgcc_s.so.1 must be installed for pthread_exit to work` | 0, `order=K` (handlers skipped) | 134, FATAL message |
| C buffers | 134, no message | 134, same message | 0, `order=31K` (`a` skipped) | 134, FATAL message |
| C++ -static-libstdc++ | 134, no message | 134, same message | hang (timeout 124; dtors=0, async waits) | 134, FATAL message |
| C plain (buffers, no -fexceptions) | 0, `order=31K` | 134, same message | 0, `order=31K` | 0, `order=31K` |
| none.c, plain / -fexceptions | 0 | 134, same message | 0 | 0 |
| C buffers, dynamic libgcc_s (control) | 0, `order=3a1K` | 127 (loader: file too short) | 0, `3a1K` / 127 | 0, `3a1K` / 127 |

libniji never loads libgcc_s.so.1 itself, so "present" and "hidden" behave alike there.

## Tests
* `forced_unwind_products.rs::static_libgcc_programs_without_a_shared_unwinder_abort_when_frames_need_unwinding`
  (three builds: libniji 134 + message + no stdout; reference 134) and
  `::static_libgcc_c_programs_with_nothing_to_unwind_exit_cleanly` (plain C: both print `order=31K`, rc 0).
  They replace `static_libgcc_c_programs_without_a_shared_unwinder_exit_through_their_buffers` (pinned the old
  buffers-only text). Dynamic-case tests unchanged.
* `crates/rt/tests/unwind.rs::d22_23_handler_frames_above_the_start` (synthetic .eh_frame: LSDA set/null, no
  `L`, start frame only, listed vs `find` objects, `max`).
* Plants: `handoff/THREAD-EXIT-NO-UNWINDER-plants.txt` (5).

## Not covered / open
* No product test puts a handler frame in a shared library other than the executable (the `frame_object`
  path is unit-tested only through `find`); a frame beyond an object without CFI ends the walk early, and the
  exit then goes on (documented in D-22.23).
* Start-up checks (`cargo xtask test`, NIJI_FULL_PROFILE hot_text_products): see the hand-back's JOBS line.

## Next step
None on this branch if the hand-back is COMPLETE. Plan rows: D-22.19, D-22.21, D-22.23.
