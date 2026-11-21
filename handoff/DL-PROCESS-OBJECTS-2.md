# DL-PROCESS-OBJECTS-2: part 2 of 3, the loader's object tables grow (audit HL-3 without TLS)

**State (leg 1 ended by a context refresh).** Branch `task/dl-process-objects-2` from next `ef7bc6fd3`; role
mei:~/w/s3. The whole part-2 conversion is written and `cargo clippy -p niji-boundary -p niji-ld --all-targets
--all-features -D warnings` is clean (job `dl-process-objects-2-c4`). Nothing has run yet: job
`dl-process-objects-2-t1` (`xtask so` + `init_order`, `dlclose`, `dlopen_process_objects` tests) was started
detached at the end of leg 1: `~/bin/jobwait dl-process-objects-2-t1` first. Design: handoff/DL-PROCESS-OBJECTS.md
sections 3-7 (not to redo). Part 3 (TLS, `MODULES_MAX`) is NOT started and stays for a later story.

**What changed (all WIP commits on the branch; squash nothing, they are pushed).**
- `ld/table.rs`: `ZeroValid` (all-zero valid entries), `Gens<N, G>` (directory of never-moved generations whose
  generation 0 lives elsewhere; `at`/`at_in`, `index`/`index_in` (address to index: gen-0 range test, then one per
  mapped generation), `reserve`, `MOST`; reuses `Arena::gen_of/gen_start/gen_len`), `Grown<T, N, G>` (gen 0 in
  place + `Gens`; `addr_in` for lock-free readers), `Wide<T>` (contiguous mremap scratch), `Records<T>`
  (contiguous lock-held copy of non-`ZeroValid` records, written prefix `len`), `Spill<T, N>` (N in place, then the
  whole list moves to a `Wide`).
- `ld/dlopen.rs`: `CAP` stays 512 = generation 0 of every table; `OBJ_GENS` = 23 (indices fit u32). `Nodes` is now
  `{ gens: Gens }` (gen 0 = `NODES` in `.lbss`, unchanged; `const assert size_of::<Node>() == 176`), with
  `get/get_mut/upto/Index/IndexMut/addr/index_of/reserve`. `Recs<T>` = published `Grown` + `Records` copy past CAP
  (`upto(k)` gives the slice; `set` writes both). Loader: `log`/`global`/`retired` are `Spill`s, new `readers: Gens`
  (readers' table directory), `room`, `scratch` (address of a `Scratch` mapping made at the first growth past CAP:
  `Wide` buffers for live_set/close_end/exit_set/load_set/reopen/dlsym), `fin: Grown<Fin>` + `fin_top` (a
  `Finis`'s entries past CAP, LIFO across nested dlclose; `Finis::entry` reads them by address, no loader borrow).
  `Loader::grow_to(len, written)` -> `#[cold] grow_cold` reserves everything for `len` objects; called in
  `candidate` where `k >= CAP` used to refuse ("cannot load more objects" is gone; failure = `NO_ROOM`). Stack
  arrays stay for n <= CAP (`room()` picks stack array or scratch). u16 indices -> u32 (`ix32`/`ix`; deps arena
  `Arena<u32, ..>`, `Node::root: u32`, `Rel` newtype for init edges). `Binds` rows are `width` words
  (`max(end, CAP)/64`). `View::obj/syms` return `Option` (gen decode); `View::find` decodes `l_next` by gen-0 range
  then `Gens::index_in`. `node_address` and new `reader_entry(i)` decode via `LOADER`. `room_short` keeps the
  top-past-3/4 clause only while the top is within CAP; `RETIRED_WAIT` = 128 (a count).
- `ld/readers.rs`: `slot(i)` (static TABLE below CAP, else `reader_entry`), `ENTRY_BYTES`; `len()` no `min(CAP)`.
- `crates/ld/src/init.rs`: `exit_order(n, succ, rank, out: &mut [u32], work: &mut [ExitWalk])`; `EXIT_MAX` gone;
  `ExitWalk` per-unit state. Tests in `crates/ld/tests/init_order.rs` updated (+ a 2000-unit chain).
- `dladdr.rs`, `dlsym.rs`: the call sites.

**Decisions to record** (pending record `records/pending/DL-PROCESS-OBJECTS-2.toml`, D-NEW in D-27, as part 1's
`records/pending/DL-PROCESS-OBJECTS-1.toml`): the dual path (stack arrays within CAP, reserved `Scratch` past it,
so `dlclose`/exit never allocate); the `fin` LIFO store; `room_short` rule; start-time objects still < CAP (the
start writes `NODES` before any loader).

**Exact next steps.**
1. `jobwait dl-process-objects-2-t1`; fix failures (cyc helper: `$S/cyc.sh <pairs.py|-> <label> [cmd]`).
2. Tests in `crates/ld/tests/dlopen_process_objects.rs` (read how part 1 builds its corpus first): 2000 objects
   held at once; `dladdr`, `dl_iterate_phdr`, `_dl_find_object` and `l_name` over all; `dlclose` and slot reuse;
   the exit order past 512 (finalisers run, dependents first). Name spec clauses as part 1's tests do.
3. Gate cost: `dlopen_process_objects_out_of_memory_is_clean` took 474 s (48 limits): stop the sweep after the
   first few completed runs, keep the refused range; report before/after time.
4. Plants file `handoff/DL-PROCESS-OBJECTS-2-plants.txt` (`cargo xtask plants`, `msg:` each): each bound restored
   alone (`Nodes::reserve` -> `len <= CAP`, `Recs::reserve`, readers `Gens::reserve`, `Spill` lists, `Scratch`,
   `fin`), and the later-generation decode of each lock-free reader (`View::node_addr`, `View::find`'s
   `index_in`, `Recs::addr_in`, `readers::slot`, `Finis::entry`).
5. Reference corpus beyond 512 objects vs the reference (part 1's probe: `~/work/DL-PROCESS-OBJECTS/refprobe.sh`).
6. Counts of `dladdr`, `dl_iterate_phdr`, `_dl_find_object` before/after (no bench row exists; instructions
   counter on a probe loop, next vs branch).
7. Gate (direct tier) + `xtask test` + full-profile `so` + `hot_text_products`; layout-bins diff vs merge base;
   precheck; report json; handback-check.
