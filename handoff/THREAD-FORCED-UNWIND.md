# THREAD-FORCED-UNWIND (handoff, also the full report)

State: branch `task/thread-forced-unwind` (from origin/next 5f3572005), code commit 70ddb590c, role mei:~/w/s4.
Result: **the fix works** (every probe case matches glibc 2.39) but `cargo xtask so` is red on one layout contract
(STARTUP-DATA-WINDOWS, 8 written start-up pages > budget 7), so the story is held: `INCOMPLETE: LAYOUT-JITTER`.

## Decisions in force
* D-22.19 (records/decisions/D-22/D-22.19.toml, reserved id): forced unwind through the program's unwinder, found
  at exit time with the loader's `dlsym` entry (RTLD_DEFAULT); nothing at start-up or thread creation; no unwinder
  loaded by libniji; processes without one keep the buffers-only exit (D-22.11); a swallowed exit restarts the unwind.
* D-22.11's "interim until phase 14" note is now superseded by D-22.19 for dynamic programs (the record of D-22.11
  was not edited; the train may want a pointer).

## What was done
* `crates/boundary/src/cancel.rs`: `continue_unwind` tries `forced_unwind()` (cold) first, then `fallback` (the old
  path). `stop` (the `_Unwind_Stop_Fn`) runs old-style records of frames already left (below the last CFA), jumps to
  the newest `__pthread_register_cancel` buffer when it lies below the current frame's CFA, and at
  `_UA_END_OF_STACK` calls `fallback` (remaining buffers, records, `thread::finish`). `swallowed` (the exception's
  cleanup) restarts the unwind. Class `NIJIEXIT`.
* `crates/rt/src/thread.rs`: `Thread::unwind: UnwindArea` (48 bytes, 16-aligned) holds the exception object (it
  must outlive the stack the landing pads reuse).
* Tests: `crates/core/tests/forced_unwind_products.rs` with `oracle/forced_unwind.cc` and `oracle/forced_unwind.c`,
  built dynamically with the host compilers and run on glibc (reference) and on the products; the expected text is
  fixed in the test. Plants: `handoff/THREAD-FORCED-UNWIND-plants.txt` (3; NOT RUN: the plants tool's product build
  fails on the same contract).

## Evidence (mei, Skylake-SP)
* Probe (black box), glibc 2.39 vs origin/next: no destructor, no handler, mutex left locked (`trylock`=16), C with
  `-fexceptions` ran no `pthread_cleanup_push` handler (`order=K` vs `3a1K`). After the change: identical output to
  glibc on all 20 C++ lines (pthread_exit, catch forced/all, lock release, cancel at pause, read, nanosleep,
  cond_wait, sem_wait, testcancel, join, async-type; thread_local/key order; C++ cleanup_push; pthread_once reset;
  std::async broken_promise; main-thread exit) and on the C plain/-fexceptions orders.
* `thread-forced-unwind-so3` (code 70ddb590c): `cargo test -p niji --test forced_unwind_products` 2 passed (the reference
  runs pass too). `cargo xtask so` in the same job: STARTUP-DATA-WINDOWS broken (below).
* `thread-forced-unwind-lint`: workspace clippy (all targets, all features, -D warnings) and `cargo xtask static` rc 0.
* Layout: `thread-forced-unwind-baseso` rebuilt origin/next's two product files in the role: all 12 contracts hold.
  With the change: `.got` bytes addressed by `locale::refresh_words` straddle 0x86a000/0x86b000 in the traced static
  program, 8 written pages > rw-budget-fast 7. `libc.a`'s `.data.rel*` sections total 45552 B on base and 45224 B
  with the change (-328 B): read-only relocated data before the GOT shrank, i.e. code partitioning moved, not a
  start-up path (the new code is `#[cold]` or reached only by address; no start-up code changed). Not chased (owner
  ruling #4347: no section moving, no bound change).

## Open questions / rulings
* RULING NEEDED: STARTUP-DATA-WINDOWS red from -328 B of `.data.rel*` (layout jitter). Recommend holding until the
  GOT/data-window placement is fixed upstream, or a coordinator call to land with the red accepted for Yohane #25.
* RULING NEEDED: a C++ handler that swallows the exit (no rethrow): libniji restarts the unwind from the handler's
  end (thread still exits, outer destructors run). The reference was not probed on this (crash rule). Recommend keep.
* RULING NEEDED: static programs (libc.a + libgcc_eh) and `-static-libgcc` programs still skip C++ destructors.
  A libc.a-only weak reference to `_Unwind_ForcedUnwind` would cover static programs (a build change: an archive-only
  object, kept out of libc.so.6 so no start-up lookup). Recommend a follow-up story.
* NOT MEASURED: the per-exit cost of the two `dlsym` lookups (and, in a C program with no unwinder, the dlerror
  record the miss creates once per thread).

## Next step
After the layout ruling: rerun `cargo xtask so`, then the plants (`cargo xtask plants
handoff/THREAD-FORCED-UNWIND-plants.txt --dir /mnt/zram/claude/thread-forced-unwind/plants-1`), the direct gate
(`cargo xtask affected --run --base origin/next`), `cargo xtask test` and the hot_text_products start-up check, then
precheck. Do not redo the probes.
