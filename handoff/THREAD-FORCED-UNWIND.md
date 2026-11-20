# THREAD-FORCED-UNWIND (handoff, also the full report; legs 1-3)

State: branch `task/thread-forced-unwind-3` (leg 2's `task/thread-forced-unwind-2` merged with origin/next
07e98c389, which carries LAYOUT-BYTE-BUDGETS-3), role mei:~/w/s2. Leg 3 ran the checks the hold (#6903) had
blocked, found one plant that survived, closed the test gap and reran everything on the final head.

## Decisions in force
* D-22.19 (records/decisions/D-22/D-22.19.toml): forced unwind through the program's unwinder, found at exit time
  with the loader's `dlsym` entry (RTLD_DEFAULT); nothing at start-up or thread creation; no unwinder loaded by
  libniji; processes without one keep the buffers-only exit (D-22.11). A C++ handler that ends without rethrowing
  the exit ends the process (`FATAL: exception not rethrown`, then `abort`), ruling #6884.
* Ruling #6904: the unwinder lookup stays uncached (3,656 instructions per exit, leg 2's measurement).
* Ruling #6885: static and `-static-libgcc` programs still skip destructors; a follow-up story is owed.

## What was done
* Leg 1 (70ddb590c): `crates/boundary/src/cancel.rs` (`forced_unwind`, `stop`, `swallowed`, `fallback`),
  `crates/rt/src/thread.rs` (`UnwindArea`, 48 B), tests `crates/core/tests/forced_unwind_products.rs` with
  `oracle/forced_unwind.cc` and `oracle/forced_unwind.c` (reference and products, fixed expected text).
* Leg 2: `swallowed` ends the process (17558dda6); the oracle's `swallow` case; plant `swallow-no-abort`.
* Leg 3: merged origin/next (a36133a64); the first plants run left `buffer-wrong-frame` SURVIVED: no C build
  reached the stop function's frame check (the plain build has no unwinder; with `-fexceptions` the header's
  handlers are unwind cleanups, no jump buffer is registered). 1058efae8 adds a third C build,
  `-fexceptions -U__EXCEPTIONS -fno-inline`: jump-buffer handlers in f3 and `thread`, f2's cleanup attribute an
  unwind cleanup between them; the test checks the build imports `__pthread_register_cancel` (and the
  `-fexceptions` one does not). Under the plant libniji prints `31K` (f2's cleanup skipped) instead of `3a1K`.

## Evidence (mei, Skylake-SP)
* `thread-forced-unwind-3-lint0` (inherited head 260e4d491): workspace clippy + `cargo xtask static` rc 0.
* `thread-forced-unwind-3-so1` (merged head a36133a64): `cargo xtask so` all 13 layout contracts hold;
  STARTUP-DATA-WINDOWS ok, 6 written start-up pages (rw-budget-fast 6), 1 ro window; forced_unwind_products 2/2
  passed, no SKIPPED line (g++ present): the swallow assertion (#6906) ran and passed.
* `thread-forced-unwind-3-plants` (a36133a64): 2 CAUGHT, 1 SURVIVED (buffer-wrong-frame): superseded.
* Final head (see the report JSON for the head and the job list): `thread-forced-unwind-3-fu2`
  (forced_unwind_products 2/2), `thread-forced-unwind-3-plants2` (3 CAUGHT, 0 SURVIVED, 0 WARN; results in
  `handoff/THREAD-FORCED-UNWIND-plants.txt`), `-gate2` (direct tier), `-xtest` (`cargo xtask test`),
  `-hottext` (NIJI_FULL_PROFILE so + hot_text_products), `-lint2`, `-precheck` (rc 0). Diagnostic: `-basebins`
  built the merge base 07e98c389 in RAM; its `layout-bins.tsv` equals this head's in every bin (bins 0-8, bytes
  and sections, `.llvm.<n>` suffixes stripped): the start-up bins did not grow.
* Probes (leg 3, `forced_unwind.c` with `-fexceptions -U__EXCEPTIONS`): separate frames (`-fno-inline`, `-O0`):
  reference and libniji both `3a1K` for exit and cancel. At `-O1`, gcc inlines f2 into `thread`, so one frame
  holds f2's unwind cleanup inside the scope of the jump-buffer handler '1': libniji runs the cleanup, then the
  handler (`3a1K`); the reference run printed `31K`. See RULING NEEDED.

## Open questions
* RULING NEEDED: one frame holding both an unwind cleanup and an older jump-buffer handler (after inlining):
  libniji runs them in scope order (the cleanup first); the reference run printed the handler only. Recommend:
  keep libniji's order, add a differences row with no compat option (an option that skips a cleanup the program
  declared has no use); a reserved id is needed for the row.
* INFORMATIONAL (leg 2): thread create + join costs 40,424 user instructions on libniji vs 2,493 on the
  reference; worth a perf story outside this one.

## Not to redo
The 20-line C++ and C probe tables (leg 1), the swallow probe (leg 2), the jump-buffer probes above (leg 3).

## Next step
If the hand-back is COMPLETE: nothing on this branch; the train lands it. Follow-ups: the static /
`-static-libgcc` story (#6885) and the ruling above. Read D-22.19 and D-22.11.
