# DL-SLOT-REUSE: reuse freed loader slots below the top (leg 1 of the run, then REFRESH)

**State.** Branch `task/dl-slot-reuse` from next `d5764940d`; role mei:~/w/s4. Code written, clippy
(`-p niji-boundary`) clean, the five `dlopen_process_objects` tests green on mei (job `dl-slot-reuse-t3`,
tree of commit "WIP: churn test names outside the corpus are not plugins"). Not yet done: measurements,
plants, gate, start-up checks, decision record, report.

**What the change does (`crates/boundary/src/ld/`).**
- `dlopen.rs` `load_set`: the set is found and staged at the top as before; then, when
  `Loader::holes` (freed slots below `n`, kept by `reclaim`) >= the set's size, cold `settle()` moves it
  into the first run of freed slots that fits (first fit): nodes, `objs`/`syms` records, reader entries
  (`readers::restage`), dependency lists renumbered in place (`table.rs` `Arena::rewrite`), top slots
  left unused (`free_slot`). Validated before anything moves. After it, `end = k.max(l.n)` bounds the
  whole-table work (versions, scope, local scope, records, `record_binds(.., end, ..)`,
  `SetTables::new(.., end)` binds width); `l.n = end`.
- Load order kept (spec/behaviour/dynlink.md P-12, chain in load order): `Loader::reordered` (set by
  `settle`) makes the chain tail come from `chain_tail` (walk) and `exit_set` number the `dlopen`ed
  units along the chain (`chain_next`); `readers::REORDERED` (set by `settle`, `readers::reorder`) makes
  `dl_iterate_phdr` take cold `iterate_chain` (walk via `dlopen::chain_find` = `View::find`). Hot cost:
  one Acquire load + branch per `dl_iterate_phdr` call; `load_set` one compare when no holes.
- `reclaim`: names top kept over paths of unloaded objects still in their grace period (`!nd.freed`,
  was `!nd.gone`): an own-code defect (a lowered top could let the next `dlopen` overwrite the path of a
  retired node a reader may still read). Describe neutrally; INFORMATIONAL in the report.
- `unwind` of a set that `settle` moved: its slots become freed slots again, `holes` restored.
- NOT done, residual: the path and dependency arenas are still bump-allocated, so under FIFO churn their
  tops still climb by each set's path bytes and list entries (bytes per open, not slots). Measure it in
  the churn table and report it as the next step (first-fit gap placement of spans) unless cheap.

**Tests** (`crates/ld/tests/dlopen_process_objects.rs`, `CHURN` program): `dlopen_slot_reuse_fifo_churn_stays_bounded`
(K=8, N=3000 two-object sets, distinct handles <= 10, load order every 97th open, dladdr paths) and
`dlopen_slot_reuse_churn_with_concurrent_readers` (reader thread: dl_iterate_phdr names/headers, dladdr of
a stack address; distinct <= 276). Before the change distinct = N.

**Next steps (exact).**
1. Churn table (brief step 1): a scratch probe in `~/work/DL-SLOT-REUSE/` on mei (the CHURN program
   extended to print, at N = 1k, 10k, 100k: distinct handles, VmSize/VmPeak from /proc/self/status, user
   instructions per open+close pair over the window via perf_event_open on itself) for K = 8, 100, 600;
   run on base (next `d5764940d` built in /mnt/zram/claude/DL-SLOT-REUSE/base-src), head, and the
   reference (system cc, normal link). Note: on base K=8 N=100k will grow the tables to ~200k slots.
2. Plants file `handoff/DL-SLOT-REUSE-plants.txt`: (a) reuse before the grace period: in `settle` treat
   `nd.gone` without `nd.freed` as free (expect the readers churn test / `reader` or a crash); (b) never
   reuse: `if l.holes >= k - base` -> `if false && ...` (expect `freed slots below the top not reused`);
   (c) chain tail by index when reordered (expect `order`); (d) `iterate` ignores REORDERED (expect
   `order`); (e) exit_set by index when reordered (expect `exit finalisers`). cmd: `cargo xtask so && cargo
   test -p niji-ld --test dlopen_process_objects`. Run via `cargo xtask plants` (§4 of the reference).
3. Gate (direct tier, `cargo xtask affected --run`), `cargo xtask clippy`, `cargo xtask static`, start-up
   checks (`cargo xtask test`; `NIJI_FULL_PROFILE=1 cargo xtask so && NIJI_FULL_PROFILE=1 cargo test -p niji
   --test hot_text_products`), layout-bins diff vs merge base, counts for the N=100 rows of
   DL-PROCESS-OBJECTS-2 (`~/work/DL-PROCESS-OBJECTS-2/count.sh`; phdr walk +2 expected).
4. Decision record: `python3 /home/claude/src/liella-ops/bin/ops reserve --story DL-SLOT-REUSE D-27`
   (first fit after staging; chain order kept; reorder flag), then precheck, report json, handback-check.
