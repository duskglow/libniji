# DL-PROCESS-OBJECTS report (run 1: part 1 of 3, the arenas, audit HL-5)

**Result.** Part 1 is done and meant to land on its own (it keeps everything working): the path arena (128 KiB) and
the dependency-list arena (16 Ki entries) grow by generations, and memory is their only bound. Parts 2 (the object
tables, HL-3 without TLS) and 3 (TLS) remain; the design for all three is in handoff/DL-PROCESS-OBJECTS.md.

**Commits** (branch `task/dl-process-objects`, from next `df83365cd`): `e2d3770d1` the design (before the code);
`f9d3f8467` the arenas and the tests; then fixes found by the role's lint and test runs (`f8caf76b5` test lint
allows and pending record, `9575aa616` one unsafe operation per block, `cc668ba65` `_GNU_SOURCE` in the test
program, `cbada8da4` the expected count includes the corpus's `libc-dl.so`); then this report.

**What changed.**
- `crates/boundary/src/ld/table.rs`: `Arena<T, N, G>` (`T`: `Plain` integers). Generation 0 is `N` entries in place;
  generation g >= 1 holds `N << (g - 1)` entries in a private mapping made at first use, never moved or unmapped.
  Virtual offsets (`gen_of`, as the heap's T14 registries); a span never crosses a generation (`open` finds the
  room first, `push` refuses to cross, `append` writes a span and its terminator). `addr_in` decodes an offset
  without a lock and without a reference (the directory word is loaded with Acquire); offsets fit 32 bits.
- `crates/boundary/src/ld/dlopen.rs`: `Loader::names` (`Arena<u8, 128 KiB, 15>`) and `Loader::deps`
  (`Arena<u16, 16 Ki, 18>`) replace the fixed arrays and their `names_top`/`deps_top`. `keep`, `path`, `deps_of`,
  `binds_of`, `adopt_deps`, `load_set` (each list reserves room for its `DT_NEEDED` count), `record_binds` (counts
  the bindings first; a failed mapping keeps the `nodelete` fallback), `reclaim` and `unwind` (`set_top`),
  `View::path` (`Names::addr_in`), the `l_name` / readers' name address (`Names::addr`). The texts "out of room for
  object names" and "cannot load more objects in one call" are gone: a mapping failure gives
  `<name>: cannot allocate memory`.
- `crates/ld/tests/dlopen_process_objects.rs` (new): see Proof.
- `records/pending/DL-PROCESS-OBJECTS-1.toml`: D-NEW, series D-27 (no id reserved; CI assigns it).

**Proof.**
- Reference (black-box probe `~/work/DL-PROCESS-OBJECTS/refprobe.sh` + `prog.c` on the build host, job
  `dl-process-objects-refprobe2`): the test's corpus built with the host toolchain: N = 300 libraries in a
  directory of 1056 bytes, 320,292 path bytes (2.4 x 128 KiB), 44,850 dependency entries (2.7 x 16 Ki): status 0;
  `sum 45150`, `dladdr 300`, `phdr 301`, `lmap 301` in both loads (the reference's C library is the system one,
  outside the directory; libniji's corpus names its `libc-dl.so` there, hence 302 for libniji).
- libniji before: next fails this corpus at 128 KiB of paths ("out of room for object names") and at 16 Ki entries
  ("cannot load more objects in one call"); plants `names-128k` and `deps-16k` restore each bound alone and must
  fail the test. After: `dlopen_process_objects_paths_and_dependencies_past_the_old_arenas` and
  `dlopen_process_objects_out_of_memory_is_clean` (address-space sweep 2-96 MiB: every run completes or fails
  cleanly): both green on the role (in the gate's `affected --run`; the second in job `dl-process-objects-t4`
  too). libniji's output matches the reference's line for line (302 in-directory objects with its `libc-dl.so`).
- Plants (`handoff/DL-PROCESS-OBJECTS-plants.txt`, 5: each old bound alone, the reader's later-generation decode,
  the `l_name` address, a span crossing a generation), job `dl-process-objects-plants`: 5 CAUGHT, 0 SURVIVED,
  0 EQUIVALENT, 0 WARN-RC-ONLY, 0 WARN-NO-MSG. `names-128k` fails at `libp117.so` (about where 128 KiB of
  ~1066-byte paths runs out, as next does), `deps-16k` at the dependency bound.
- Gate (job `dl-process-objects-gate`: clippy workspace all-targets all-features `-D warnings`, `xtask static`,
  `records check`, `so`, `affected --run` (direct tier), `xtask test`, full-profile `so` + `hot_text_products`):
  rc 0 on `cbada8da4`: clippy and static clean, records check ok; affected (direct tier) 6955 run, 6955 passed
  (11 slow), 784 skipped (both new tests among them); `xtask test` 4846 passed, 607 skipped, then 126 passed;
  `hot_text_products` 32 passed; peak 16G (page cache included), oom_kill 0.
- INFORMATIONAL (gate time): `dlopen_process_objects_out_of_memory_is_clean` took 474 s in the gate (48 limits,
  each loading the 301-object set) and was the last test to finish. Part 2 should stop the sweep after the first
  few completed runs (every larger limit completes too), keeping the refused range that is the test's point.
- Cost: no start-up code changed (the loader is built at the first `dl*` call); generation 0 is the old array in
  place, so a process within the old room touches the same pages. `dladdr` adds one compare per path decode
  (code read); `dl_iterate_phdr` reads the same name word. No bench row covers `dladdr`, `dl_iterate_phdr` or
  `dlopen` (`bench/instr_limits.tsv`), so no counter row could move; the start-up bounds and layout contracts are
  the gate's `xtask test` and `hot_text_products`.

**Superseded jobs.** `dl-process-objects-t1` (test lints), `-t2` (one unsafe op per block), `-t3` (the test program
lacked `_GNU_SOURCE`), `-t4` (the expected count missed `libc-dl.so`; libniji's paths were all right),
`dl-process-objects-refprobe` (same `_GNU_SOURCE` miss).

**Next part (2, the object tables).** Design sections 3-7 of handoff/DL-PROCESS-OBJECTS.md: the nodes beyond
`NODES`, the object records (published generations plus a contiguous lock-held copy past 512), the lock-held lists
and per-call scratch (reserved by the `dlopen` that grows `n`, so `dlclose` and the exit never allocate), the
readers' table, `exit_order`'s work slices, u32 indices (the dependency entries widen then). Tests: 2000 objects
held, `dladdr` and `dl_iterate_phdr` over all, `dlclose` and slot reuse, the exit order past 512. Then part 3 (TLS).
