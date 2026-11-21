# DL-PROCESS-OBJECTS: no limit on the objects, TLS modules and names a process holds (audit HL-3, HL-5)

**State.** Branch `task/dl-process-objects` from next `df83365cd`; role mei:~/w/s2. The story lands in three parts,
each keeping everything working: **1 the arenas (HL-5)**, 2 the object tables (HL-3 without TLS), 3 TLS (HL-3's
`MODULES_MAX`). This note holds the design (written before the code); the run's results are at the end.

## Design (one section per table)

**Rules every table follows.**
- *Generation 0 is the existing fixed table, in place*: a process within the old values runs the same code on the
  same bytes (start-up rows and faults cannot move). Generation g >= 1 holds `C << (g - 1)` entries (`C` = generation
  0's), a private anonymous mapping made at its first use, **never moved, never unmapped**, as the heap's
  registries (T14, `rt/src/heap.rs` `gen_of`): an address handed out once (a handle, `l_name`, `dlpi_name`, a dtv)
  stays valid for the process.
- A generation's address goes into a directory word (Release) before anything in it is published; a lock-free
  reader loads it (Acquire) only for an index or offset it reached through a published chain or count.
- *The only failure is a mapping's*: the call fails with its own error (`dlopen` NULL, dlerror
  `<name>: cannot allocate memory`) and the attempt unwinds as today. `dlclose` and the exit never need memory
  (their scratch is reserved by the `dlopen` that made it necessary).
- *Indices and spans are u32*: 4 Gi objects, bytes or entries is a representation bound past the kernel's own
  (every object takes at least two mappings; `vm.max_map_count` is 65,530 by default), the kind HL-32 keeps.

### 1. Path arena (`Loader::names`, 128 KiB; HL-5) - part 1
- **Grows**: generation 0 is the 128 KiB in the loader's mapping, then 128 KiB, 256 KiB, ... A path with its NUL is
  one span inside one generation: one that does not fit in the rest of its generation starts the first later
  generation large enough (the tail stays unused until the top falls below it). `Node::path` stays
  `(u32 offset, u32 len)`; the offset is virtual (`gen_of`). `names_top` is virtual too: `unwind` and `reclaim` set
  it back as today, lower offsets are reused, generations stay mapped.
- **Read without a lock**: `dladdr` (`View::path`, `dli_fname`); by address, a `dlopen`ed object's `l_name` (the
  link-map chain: programs, debuggers through `r_debug`) and its readers'-table name word (`dlpi_name`). Never
  moved, so every such address stays valid until the slot is reused after the grace period, as today.
- **Hot paths**: `dladdr` decodes one span: offset < 128 KiB is today's arithmetic; past it, one directory load.
  `dl_iterate_phdr` reads the name word, unchanged.

### 2. Dependency-list arena (`Loader::deps`, 16 Ki entries; HL-5) - part 1
- **Grows**: as the path arena (generation 0 the 16 Ki entries in place). Each object's list (its `DT_NEEDED`
  matches, `adopt_deps`/`load_set`; its bindings, `record_binds`) is one span: room for the list's most entries
  (its `DT_NEEDED` count; for bindings, the objects past the start's) is found before the list is written.
- **Read without a lock**: nothing (`deps_of`/`binds_of` run under `CALL`: `dlclose`'s live set, local scopes,
  the exit order). One arena type serves both arenas; the generations cost nothing here.
- `record_binds` keeps its fallback when a mapping fails (the definer becomes `nodelete`): never a failed `dlopen`.
- Entries stay u16 while `CAP` is 512; part 2 widens them with the indices.

### 3. Link-map nodes (`NODES`, 512 x 176 bytes in `ld.so`'s `.lbss`) - part 2
- **Grows**: generation 0 stays `NODES` (the start writes the start-time nodes there before any loader exists,
  DL-23); generations >= 1 are mapped by the loader. Index to address: below 512 today's arithmetic, else the
  directory; address to index (`index_of`, `View::find`'s step): one range test for generation 0, then the mapped
  generations' ranges (at most ~23).
- **Read without a lock**: `View` (`dladdr`: chain, ranges, paths, `l_name`), `node_address` (the readers' table),
  debuggers (`l_next`). Never moved: handles and link maps stay valid.
- **Hot path**: `View::find` keeps the generation-0 range test first; past 512 objects one more compare per
  generation.

### 4. Object records (`Loader::objs`, `Loader::syms`, 512 each) - part 2
- **Grows**: two copies (`Obj` and `Object` are `Copy`, a few hundred bytes per object): the *published* records
  in generations never moved (generation 0 the arrays in place), which `View::obj`/`View::syms` read without a
  lock; and, past 512 objects only, a *contiguous* lock-held copy in a `MapStack` (moved by `mremap`), which the
  relocator and the lookups take as `&[Obj]` / `&[Object]` (`objs_of`, `syms_of`). Within 512 the in-place
  arrays are both, as today: no API change to `niji_ld`.
- **Hot path**: `dladdr`'s symbol search reads `View::syms(i)`: one compare for generation 0.

### 5. Lock-held lists and per-call scratch - part 2
- `global`, `log`, `retired` (512 each in the loader): 512 in place, then a `MapStack` (only the lock holder reads
  them; `n_log` stays the count).
- Stack scratch sized `CAP` (`Finis` ~21 KiB, `live_set`, `local_scope`, `exit_set`, `close_end`'s marks) and
  `Binds` rows `CAP` bits wide: `ld/table.rs` `Table`s (64 in place, then a mapping) sized to `n`, rows `n` bits
  wide. `dlclose` and the exit must not allocate: the `dlopen` that grows `n` reserves their scratch for the new
  `n` (a failed reservation fails that `dlopen` cleanly).
- `RETIRED_WAIT`/`room_short` (a quarter of `CAP`): a count of retired objects (128, as today), no table size.

### 6. Readers' table (`readers::TABLE`, 512 x 5 words static) - part 2
- **Grows**: generation 0 the static table; generations >= 1 mapped under the lock and in the directory before
  `PUBLISHED` passes their first entry.
- **Read without a lock**: `dl_iterate_phdr` (`iterate`) and `_dl_find_object`, below `PUBLISHED` (Acquire).
- **Hot path**: `View::entry(i)`: below 512 the static table as today, else one directory load.

### 7. Exit order (`init::EXIT_MAX`, 512 units) - part 2
- `exit_order`'s internal arrays become caller work slices (as `init_order`'s `work`), from the scratch of 5.

### 8. TLS (`ltls::MODULES_MAX`, 512) - part 3
- `ModuleIds` (a 512-bit free set) and `Surplus<512>` (one place per module): lock-held, 512 in place then a
  mapping; `take` has no `MODULES_MAX` ceiling. The surplus's bytes (HL-33, 64 KiB) stay as they are.
- `THREAD_MODS` (16 entries beside the nodes, 496 static): generation 0 as today, generations >= 1 in a
  directory; read without a lock by thread creation (`fill_static`) and `tls_slow`, below `THREAD_MODS_LEN`.
- **dtv**: today every dtv has room for 512 slots (`THREAD_DTV_WORDS`, `link.rs` `with_dtv_slots(MODULES_MAX)`).
  After: each dtv carries its own capacity (a header word); a module ID past it goes to `tls_slow`, which gives
  the calling thread a vector twice as large (a mapping; the old one is released when the thread ends, since a
  `dlopen`'s thread walk may still write it); that walk skips the slots a thread's vector lacks (the thread grows
  its own at its first access). New threads get room for the modules then loaded.
- **Hot path**: `__tls_get_addr` reads the vector's own capacity instead of `THREAD_DTV_WORDS` (one load either
  way); the generation test is unchanged. Its bench rows must stay within `bench/instr_limits.tsv`.

### Tests per part
- Part 1: one `dlopen` set whose paths pass 128 KiB (several times over) and whose dependency lists pass 16 Ki
  entries; `dladdr`, `dl_iterate_phdr` and `l_name` give every object's full path (spans in generations >= 1);
  `dlclose` and a second load (spans reused); an address-space sweep: each run completes or fails cleanly.
- Part 2: 2000 objects held at once, `dladdr` and `dl_iterate_phdr` over all of them, `dlclose` and slot reuse,
  the exit order past 512. Part 3: TLS in all 2000 (static and dynamic TLS models), threads made before and after
  the growth reading TLS of objects loaded later.
