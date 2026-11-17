# DL-STARTUP-OBJECTS: no fixed limit on the objects loaded at program start (GitHub issue #9)

**State.** Branch `task/dl-startup-objects` (from next 86b4af50b). Role mei:~/w/s3. Pending record
`records/pending/DL-STARTUP-OBJECTS-1.toml` (D-NEW, series D-27, amends D-27.16 and D-27.37).

**What changed.**
- `crates/boundary/src/ld/table.rs` (new): `Table<T, N>`: the first `N` entries inline and *uninitialised* until
  reserved, spills to a private anonymous mapping (doubling) beyond `N`; `reserve` is the only fallible step
  (`NoMemory`). `Kept<T>`: a table published once for the process (dlopen / exit readers). `PathStore`: the
  `$ORIGIN` path store in 16 KiB chunks (a longer path gets its own chunk).
- `ld.rs` `link`: every per-object table, the `DT_NEEDED` links (128 inline, was a fixed 1024 of `(u8, u8)`), each
  search's requester chain (was silently cut at 64 deep), TLS modules/placements, symbols, global scope, biases,
  hooks are `Table`s; indices `usize`. Start records: `START_OBJS` static for <= 64 objects, else `START_SPILL`
  (read through `start_obj`, used by `readers.rs` and `dlopen.rs`). Exit: `FINI_HOOKS`/`FINI_IDX` for <= 64, else
  `FINI_SPILL`. `run_init` computes the fini order as the init order reversed (one walk, not two).
  `list` (`--list`, trace): objects and chains are `Table`s, paths in a `PathStore` (`Obj.path` is the slice).
  The three "too many objects" texts and "too many DT_NEEDED entries" are gone; a failed mapping ends with
  `ld.so: <name>: cannot allocate memory`, status 127.
- `crates/ld/src/init.rs`: `init_order`/`fini_order` take caller storage (`out: &mut [usize]`, `work: &mut [Walk]`)
  and edges of any index width (`E: Into<usize>`); `OBJECTS_MAX` removed. `dlopen.rs`: `NEW_MAX` is its own
  constant (64, unchanged); `adopt_tls` (which refused a start of more than 64, so `dlopen` failed after one)
  uses `Table`s.

**Proof (all on mei, fast profile products).**
- Issue reproducer (`~/work/DL-STARTUP-OBJECTS/probe.sh repro`, job dl-startup-objects-counts): N = 63, 64, 65,
  255, 256, 257, 300 all rc 0 on libniji and on the reference.
- Tests: `crates/ld/tests/start_objects.rs` (300 generated libraries: every constructor once, destructors in exact
  reverse, `dl_iterate_phdr` sees them all, `dlopen` afterwards works, `--list` lists all 300; and an
  address-space sweep 1-48 MiB: from the first limit at which the loader runs, every run either succeeds or ends
  127 with one `ld.so:` line, never a signal; observed reasons include `cannot allocate memory`);
  `init_order.rs` `init_order_long_chain` (300 objects, 299 deep). Green in job dl-startup-objects-counts5.
- Start-up counters, dynamic niji program with N libraries (`count2.sh`, perf instructions:u / page-faults,
  20 runs x3, deterministic to +-2 instr): N=0 9,043,823 -> 9,032,665, faults 84 -> 81; N=3 9,322,123 ->
  9,312,219, 99 -> 97; N=12 10,120,407 -> 10,113,747, 135/136 -> 133; N=65 (base refuses) 15,392,757 / 371.
  The static start (`cbench_startup`) does not link the loader: unchanged by construction (not re-measured).
  A first version (inline tables filled at construction, as the old arrays were) measured +7k instructions and
  +1-2 faults: superseded by the uninitialised inline storage.
- Plants: `handoff/DL-STARTUP-OBJECTS-plants.txt` (9: each old bound restored alone), job dl-startup-objects-plants2.
- Gate: dl-startup-objects-gate2 (static, `affected --run`, `xtask test`, full-profile `so` + `hot_text_products`).

**Open / RULING NEEDED.** The `dlopen` loader (`ld/dlopen.rs`) keeps fixed capacities: `CAP` = 512 objects in the
process (start-time ones included; `adopt_start` and `readers::fill_start` give up at a start of >= 512, so then
`dlopen` fails and `dl_iterate_phdr`/`_dl_find_object` report no start-time object), `NEW_MAX` = 64 new objects
per `dlopen` call, `init::EXIT_MAX` = 512, and `THREAD_MODS` = 512 static-TLS modules recorded for new threads.
Recommend a follow-up story (DL-LOADER-GROW) to make the loader's node table growable the same way.

**Informational.** Under an address-space limit, a candidate file whose mapping fails is reported as
`cannot open shared object file: No such file or directory` / `cannot read file data` (existing
`open_candidate` texts), not as a memory error. No plant covers the requester-chain depth (a 65-deep RPATH chain
corpus would). The init walk is O(objects x links) (the edge scan per object): fine at hundreds of objects.

**Next step.** Read the gate and plants verdicts (labels above); on green the branch is ready.
