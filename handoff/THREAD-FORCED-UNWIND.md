# THREAD-FORCED-UNWIND (handoff, also the full report; legs 1 and 2)

State: branch `task/thread-forced-unwind-2` (leg 1's `task/thread-forced-unwind` merged with origin/next 50abc44ed),
role mei:~/w/s1. **Held** (coordinator, #6903): `cargo xtask so` is red only on STARTUP-DATA-WINDOWS (8 written
start-up pages > rw-budget-fast 7); LAYOUT-BYTE-BUDGETS-2 did not retire that contract. The story reruns after
LAYOUT-BYTE-BUDGETS-3 lands.

## Decisions in force
* D-22.19 (records/decisions/D-22/D-22.19.toml): forced unwind through the program's unwinder, found at exit time
  with the loader's `dlsym` entry (RTLD_DEFAULT); nothing at start-up or thread creation; no unwinder loaded by
  libniji; processes without one keep the buffers-only exit (D-22.11). Leg 2: a C++ handler that ends without
  rethrowing the exit ends the process (`FATAL: exception not rethrown`, then `abort`), ruling #6884.
* Ruling #6885: static and `-static-libgcc` programs still skip destructors; a follow-up story is owed.

## What was done
* Leg 1 (70ddb590c): `crates/boundary/src/cancel.rs` (`forced_unwind`, `stop`, `swallowed`, `fallback`),
  `crates/rt/src/thread.rs` (`UnwindArea`, 48 B), tests `crates/core/tests/forced_unwind_products.rs` with
  `oracle/forced_unwind.cc` and `oracle/forced_unwind.c` (reference and products, fixed expected text).
* Leg 2: merged origin/next (3bad4a0d6); `swallowed` now ends the process (17558dda6); the oracle's `swallow`
  case prints from the handler and the test checks stdout, the diagnostic and SIGABRT (libniji only); plants file
  updated (`swallow-no-abort` replaces `swallow-no-reunwind`).

## Evidence (mei, Skylake-SP)
* `thread-forced-unwind-2-lint0` (inherited head 8231794ab): workspace clippy + `cargo xtask static` rc 0.
* `thread-forced-unwind-2-so1` (merged head 3bad4a0d6, before the swallow change): `cargo xtask so` 1 of 13
  contracts broken, STARTUP-DATA-WINDOWS (the `.got` byte addressed by `locale::refresh_words` alone on its page,
  8 > 7); WINDOW-BIN-BYTES ok (R2's contract). The forced_unwind test did not run (chained after `so`).
* `thread-forced-unwind-2-lint1` (head after the swallow change): see the report JSON.
* Cost (probe built against the so1 products, user-mode instructions via perf_event_open, threads included):
  dlsym pair 3,656 per exit (reference 1,969); create + `pthread_exit` from a C++ frame + join 68,518
  (reference 13,863); create + return + join 40,424 (reference 2,493). The lookup runs on every exit, not once
  per process; it is ~5% of an exiting thread's cost.

## Open questions
* RULING NEEDED: caching the unwinder lookup (brief: cache it if not once per process). Not done: a safe cache
  needs a shared slot behind a seqlock keyed on the loader's add/remove counters (new statics on a story held on
  start-up data layout) for ~3.3k of 68.5k instructions on a cold path. Recommend: leave uncached.
* INFORMATIONAL: thread create + join costs 40,424 user instructions on libniji vs 2,493 on the reference (outside
  this story), and the unwind itself costs more than the reference's beyond the lookup; worth a perf story.

## Not to redo
The 20-line C++ and C probe tables (leg 1) and the swallow probe (leg 2): settled.

## Next step
After LAYOUT-BYTE-BUDGETS-3 lands: merge origin/next, `cargo xtask so` (must be green), then
`cargo test -p niji --test forced_unwind_products` (the swallow assertion has not run yet), plants
(`cargo xtask plants handoff/THREAD-FORCED-UNWIND-plants.txt --dir /mnt/zram/claude/thread-forced-unwind-2/plants-1`),
the direct gate, `cargo xtask test`, the hot_text_products start-up check, precheck. Read D-22.19 and D-22.11.
