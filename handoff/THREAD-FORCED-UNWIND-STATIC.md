# THREAD-FORCED-UNWIND-STATIC (handoff, also the full report)

State: branch `task/thread-forced-unwind-static` (stacked on THREAD-FORCED-UNWIND-3's head e50789a9d), role
mei:~/w/s2. Code head: see the report JSON (`handoff/reports/THREAD-FORCED-UNWIND-STATIC.json`).

## Decisions in force
* D-22.21 (records/decisions/D-22/D-22.21.toml, amends D-22.19): a static program's exiting or cancelled thread
  unwinds with the unwinder the program links. `csys/src/machine/unwinder.S` gives `niji_machine_unwind_force` /
  `niji_machine_unwind_get_cfa`. The C layer leaves the file out of the `runtime` (libc-so) build
  (crates/boundary/build.rs) and builds the form that returns 0 everywhere else. `cargo xtask so` links that form into
  libc.so.6 (and libc-owner.so) and adds the `-DNIJI_STATIC_ARCHIVE` form (weak references to `_Unwind_ForcedUnwind`,
  `_Unwind_GetCFA`) to libc.a as the member `niji-unwinder.o`. Checks in so.rs: no member defines the entry before;
  some member refers to it; afterwards the new member is libc.a's only reference to the two names, weak;
  libc.so.6's dynsym names neither. `cancel::unwinder()` asks the linked unwinder first, then `dlsym` (D-22.19).
* D-22.19, ruling #6904 (uncached lookup), ruling #6884 (swallowed exit aborts) unchanged.

## What was done
* Probe (black box, glibc 2.39, Skylake-SP host): `forced_unwind.cc` and the three `forced_unwind.c` builds linked
  `-static`, and `forced_unwind.cc` linked `-static-libgcc` (libstdc++.so still loads libgcc_s.so.1), print the
  dynamic programs' text. libgcc_eh.a's `_Unwind_ForcedUnwind`/`_Unwind_GetCFA` are GLOBAL HIDDEN (toolchain
  `readelf`), so a `-static-libgcc` program's own copy is invisible to any lookup.
* First attempt replaced the member defining the entry in libc.a: the archive's members are joined into one object by
  then (copies.rs), so it replaced the whole library (link errors). Fixed as above.
* Tests (`crates/core/tests/forced_unwind_products.rs`): `static_programs_unwind_with_the_unwinder_they_link` (static
  C++ with libstdc++.a, three static C builds; the plain C build must link no unwinder; reference and libniji both
  checked against the fixed text), `static_libgcc_cxx_programs_unwind`. Static links take gcc's crtbeginT.o/crtend.o
  (`__dso_handle`).
* Plants: `handoff/THREAD-FORCED-UNWIND-STATIC-plants.txt` (static form returns 0; Rust side does not ask).

## Evidence (mei)
* `thread-forced-unwind-static-so2`: `cargo xtask so` passed with the new checks; `-fu3`: forced_unwind_products 4/4.
* `-lint1`: workspace clippy (all features), `cargo xtask clippy`, `cargo xtask static`: rc 0.
* `-gate` (direct tier, f248de28a; `affected` named unwinder.S FULL, "the train's", and ran its bounded set):
  12385 passed, 1 test red: hot_text_products `init_no_library_text_precedes_the_hot_group` (TEXT-PREFIX: the
  entries' `.text.unlikely.*` sections lay ahead of the hot group). 711cc86eb uses `.text.<name>`.
* `-recheck` (code head 711cc86eb): `cargo xtask so`, hot_text_products 31/31 and forced_unwind_products 4/4, then
  `cargo xtask test`, then `NIJI_FULL_PROFILE=1` so + hot_text_products.
* `-plants3` (711cc86eb): 4 CAUGHT, 0 SURVIVED, 0 WARN (`handoff/THREAD-FORCED-UNWIND-STATIC-plants.txt`). Two are
  caught by the build checks in so.rs (msg names their error), two by the static test (`linked-skipped` also fails the
  dynamic tests, which the tool names first). Superseded: `-plants` (msg named the test, both stopped at the build
  check: WARN-NO-MSG), `-plants2` (same 4 plants, previous head: 4 CAUGHT).
* Then `-precheck` on the final head (see the hand-back).

## Open questions
* RULING NEEDED: a `-static-libgcc` program that loads no libgcc_s.so.1 (C with `-fexceptions`; C++ with
  `-static-libstdc++`) keeps the buffers-only exit (its unwind cleanups and destructors do not run). Its unwinder is
  hidden; the only way to unwind would be libniji loading libgcc_s.so.1 itself at the exit and mixing it with the
  program's copy, which D-22.19 rules out. Recommend: keep the buffers-only exit.

## Not to redo
The static / -static-libgcc probe table above.

## Next step
None on this branch if the hand-back is COMPLETE. Plan rows: D-22.19, D-22.21.
