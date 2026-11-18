# DL-DLOPEN-NEW-OBJECTS: no limit on the new objects one `dlopen` call loads (audit HL-4)

**State.** Branch `task/dl-dlopen-new-objects` (from next e035b3034). Role keke:~/w/n3. Pending record
`records/pending/DL-DLOPEN-NEW-OBJECTS-1.toml` (D-NEW, series D-27, amends D-27.58). Plants file
`handoff/DL-DLOPEN-NEW-OBJECTS-plants.txt` (10 plants).

**What changed.**
- `crates/boundary/src/ld/dlopen.rs`: `NEW_MAX` (64) is gone. `load_set`'s per-object tables (`Binds` rows, pass-2
  marks, init `order`/`work`, `Ctors`) are `ld/table.rs` `Table`s (64 in place, then a private mapping, doubling),
  bundled in `SetTables` and reserved together right after the breadth-first walk, before versions, TLS, relocation
  and publication, so a failure still unwinds the attempt: `dlopen` NULL, `dlerror` `<file>: cannot allocate
  memory`. The new-to-new dependency edges were a fixed `[(u8, u8); 1024]` (a second cap, and an 8-bit index): now a
  `Table<(u16, u16), 256>` pushed as found (u16: indices stay below `CAP` = 512, HL-3).
  `place_tls`: the new modules are pushed (failure: `put_back` the ID, free the place, `cannot allocate memory`).
  `close_end`: the released TLS module IDs are pushed; if the table cannot grow, the batch so far is released at
  once (as a separate `dlclose` would) and the table reused, so `dlclose` never needs memory and never drops a
  module (the old code silently dropped IDs past 64, leaving them in the new-thread template).
- `crates/boundary/src/ld.rs`: `NewTls.mods` is a `Table<[u64; 5], INLINE>` (`len` field gone; `NewTls::none()`).
- `crates/boundary/src/ld/table.rs`: `Table::push`, `Table::clear` (keeps the storage).
- The text `cannot load more objects in one call` remains only for the dependency-list arena `DEPS` (HL-5).

**Proof.**
- Reference probe (`~/work/DL-DLOPEN-NEW-OBJECTS/refprobe.py`, same corpus as the test, built with the host
  toolchain; jobs dl-dlopen-new-objects-refprobe, -refprobe2): N = 64, 65, 100, 200, 300 all rc 0; at 200: sum 26100,
  constructors `cT, c200 .. c0`, nothing unloaded by `dlclose(libt)`, `d0 .. d200, dT` at `dlclose` and at exit,
  the thread after the `dlclose` runs. The test asserts exactly this.
- libniji before: next refuses a set of more than 64 new objects (`NEW_MAX`; plant new-max-64 restores that cap and
  must fail the test). After: `crates/ld/tests/dlopen_new_objects.rs` green on the role (job dl-dlopen-new-objects-t4):
  `dlopen_new_objects_past_every_old_bound` (301 new objects in one call, 2079 new-to-new links, a TLS variable in
  each of 300 libraries, an IFUNC in library 150, library 300 bound to `libt.so` outside its dependencies; dlclose;
  a thread created after it; second load; exit order) and `dlopen_new_objects_out_of_memory_is_clean` (address-space
  sweep 1-48 MiB: 91 ran, 98 refused, never a signal).
- Lint (job dl-dlopen-new-objects-lint): clippy workspace all-targets all-features `-D warnings`, `xtask static`,
  `records check`: green.
- Final head: job dl-dlopen-new-objects-final (clippy `-D warnings`, static, records check, `affected --run`, `xtask test`,
  full-profile `so` + `hot_text_products`) and job dl-dlopen-new-objects-plants3 (10 plants, each old bound alone).
- Superseded: plants2 (7 caught; edges-u8 survived with 201 objects, below a wrapping 8-bit index, so the test now
  loads 301; new-tls-64 and close-tls-64 survived because a never-written `static __thread` was folded away and the
  libraries had no PT_TLS, so each library now exports its TLS variable and the test checks PT_TLS); gate (stopped:
  tree superseded; `affected` was ok); refprobe3 confirms the reference at N = 300 with real TLS modules.
- Start-up cost: the change touches no start-up code (only `dlopen`/`dlclose` paths); the start-up bounds and layout
  contracts are checked by the gate's `xtask test` and `hot_text_products`. No primed base products on the host, so
  no layout-bins diff against the base was made.

**INFORMATIONAL.** In the address-space sweep, some refused `dlopen`s report `dlerror` NULL (text "(null)") or
"ELF load command address/offset not page-aligned" / "cannot read file data" when a mapping of an object fails under
the limit: the existing mapping/candidate paths, not this change's tables (same kind as DL-STARTUP-OBJECTS's note).
A follow-up should make a failed object mapping say "cannot allocate memory" and make sure `dlerror` always has a
text after a failed `dlopen`. Not covered by a plant: the `close_end` batch release (needs a mapping failure in
`dlclose` after 64 TLS modules) and the `SetTables`/edge reservation failure paths (need a mapping failure at that
exact point; the sweep exercises the area but does not target it).

**Not to redo.** `u32` edges do not satisfy `init_order`'s `Into<usize>` bound; `u16` does.

**Next step.** Read the gate and plants verdicts; on green the branch is ready. Later stories: HL-3 (`CAP` 512,
readers' table, `THREAD_MODS`, exit order), HL-5 (`NAMES`, `DEPS` arenas).
