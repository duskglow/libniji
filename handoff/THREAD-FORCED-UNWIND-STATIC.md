# THREAD-FORCED-UNWIND-STATIC (handoff, also the full report; runs -1 and -2)

State: branch `task/thread-forced-unwind-static-2` (THREAD-FORCED-UNWIND-STATIC's head eb72560b3 plus a merge of
origin/next d4cbe1667), role mei:~/w/s1. Head: see `handoff/reports/THREAD-FORCED-UNWIND-STATIC-2.json`.

## Decisions in force
* D-22.21 (records/decisions/D-22/D-22.21.toml, amends D-22.19): a static program's exiting or cancelled thread
  unwinds with the unwinder the program links. `csys/src/machine/unwinder.S` gives `niji_machine_unwind_force` /
  `niji_machine_unwind_get_cfa`; the C layer leaves it out of the `runtime` build (crates/boundary/build.rs); `cargo
  xtask so` links the form returning 0 into libc.so.6 / libc-owner.so and adds the `-DNIJI_STATIC_ARCHIVE` form (weak
  references to `_Unwind_ForcedUnwind`, `_Unwind_GetCFA`) to libc.a as `niji-unwinder.o`, with checks in so.rs.
  `cancel::unwinder()` asks the linked unwinder first, then `dlsym` (D-22.19).
* Ruling #7081: a `-static-libgcc` program that loads no libgcc_s.so.1 keeps the exit without an unwinder.
* D-22.19, ruling #6904 (uncached lookup), ruling #6884 (swallowed exit aborts) unchanged.

## Run -2 (THREAD-FORCED-UNWIND-STATIC-2)
* Step 0 on the inherited head: workspace clippy + `cargo xtask static` green (`-lint0`).
* Merged origin/next (brief allows it): the xtask/src/so.rs conflict with LAYOUT-RO-STABLE kept both sides
  unchanged (RODATA1_FIRST fragment, check_read_only_segment next to the unwinder member and its checks; the link takes
  both `-T` and the unwinder object). `-so1`: `cargo xtask so` passed every check, forced_unwind_products 4/4.
* libniji probe of ruling #7081's case (`forced_unwind.c`/`.cc` with `-static-libgcc`, no libgcc_s.so.1): C
  `-fexceptions`: `order=K` (handlers do not run), rc 0; C buffers build: `order=31K`, rc 0; C++ with
  `-static-libstdc++`: no destructor, no catch, mutexes owned by frame objects stay locked (`trylock after join=16`),
  and the `std::async` case waits forever (the promise is never broken). New test
  `static_libgcc_c_programs_without_a_shared_unwinder_exit_through_their_buffers` pins the two C texts (libniji run
  only); the C++ hang is why C++ is not in it.
* Ruling #7081's differences row, registered compat record and reference probe text: NOT written; see the
  hand-back (RULING NEEDED). Do not write them until the coordinator answers.
* Plants: `handoff/THREAD-FORCED-UNWIND-STATIC-2-plants.txt` (run -1's four on the merged head, plus
  `slg-buffers-skipped` for the new test).

## Run -1 (summary)
Probe of the static / -static-libgcc builds on the reference (static C++, three static C builds, dynamic C++ with
`-static-libgcc` which still loads libgcc_s.so.1 via libstdc++): all print the dynamic programs' text. libgcc_eh.a's
`_Unwind_ForcedUnwind`/`_Unwind_GetCFA` are GLOBAL HIDDEN. Tests `static_programs_unwind_with_the_unwinder_they_link`,
`static_libgcc_cxx_programs_unwind`. Replacing libc.a's member that defines the entry replaced the whole library
(members are joined by copies.rs): hence the extra member. Entries live in `.text.<name>` (TEXT-PREFIX check).

## Not to redo
The probe tables above.

## Next step
None on this branch if the hand-back is COMPLETE; the ruling #7081 row/flag waits for the coordinator. Plan rows:
D-22.19, D-22.21.
