# DL-EXIT-ORDER-PROBE: exit order of dlopen objects (handoff, run 1, context checkpoint)

**State:** branch `task/dl-exit-order-probe`, HEAD = the commit adding this note (report json beside it).
Status INCOMPLETE: the probe, the reference record and the pure order function with its tests are done;
the loader is not wired to the order yet, so Liella's behaviour is unchanged. Do not land as is
(nothing wrong ships, but the story's change is missing).

**Probe-settled (record `== exit-order (DL-EXIT-ORDER-PROBE)` in spec/behaviour/dynlink.glibc.txt, text in
spec/behaviour/dynlink.md):** the reference runs the program's destructor first, then start-time and
`dlopen`ed objects as ONE sequence: earliest loaded first, each object after every object that needs
it (`DT_NEEDED`, and also a global-scope binding: E21). NODELETE, unique pinning, handle open or not:
no effect (E1-E12). So DL-DLCLOSE's C-6 before C-7 is plain load order, not a unique-pinning effect.
Inside a dependency cycle the reference's order depends on the other loaded objects (E19-E30); no
rule matches it.

**Liella today** (`crates/boundary/src/ld.rs` `fini_all`, `dlopen::fini_loaded`): all `dlopen`ed objects
first, last initialised first, then the start-time objects in `init::fini_order` (program first). It
differs in every multi-object probe and in DL-0b's `== order` exit record (E before the program there).

**Done:** `liella_ld::init::exit_order(n, succ, rank, out)` (crates/ld/src/init.rs): Tarjan groups,
then of the free groups the one with the earliest unit; inside a group (a cycle) the last initialised
first. Tests: `exit_order_load_order_dependents_first` in crates/ld/tests/init_order.rs (the `== order`
exit record, E1-E3/E8/E9, E14, E17, E18, E20, E24, E25, E29, E30, errors): passed on keke (unit1,
before the clippy fixes; unit2 on the final head).

**Exact next step (the wiring, est. 30-40 calls):**
1. `ld.rs` `run_init`: store each `FINI_HOOKS` slot's start-time object index (new static `FINI_IDX`,
   same index space as `START_OBJS` / loader nodes 0..n_start; check the interp slot).
2. `fini_all`: when `LOADER != 0`, replace `fini_loaded()` + the hook loop by one call into `dlopen`
   that, under `STATE`, builds the sequence (start-time objects in FINI order, then live `dlopen`ed
   nodes `n_start..n`, not gone or finalising, in index order = load order: slots are reused from the
   top only), `succ` = chain edge to the next start-time unit, plus for a dlopen unit its `deps_of` and
   `binds_of` mapped to positions (skip gone ones); `rank` from the constructor log (`l.log`), start
   units by reverse FINI position; marks dlopen units finalising (as `Finis::add`), collects all hooks
   (a `Finis`-like array of CAP), unlocks, runs. `LOADER == 0` keeps today's loop (no cost for
   programs that never dlopen).
3. C test (new mode in crates/ld/tests/dlclose.rs or a new test file): the probe scenarios E1, E5, E14,
   E18, E21, E25 (needs a start-time lib) and the record's exit case, exact `dtor` sequences; cycles
   assert Liella's rule.
4. Decision record `records new decision D-NEW --series D-27 --story DL-EXIT-ORDER-PROBE`; differences
   row for the in-cycle order (RULING NEEDED below); plants on `exit_order` (group order, rank sort,
   free-group pick) and on the wiring (chain edge, binds edges); the start-up checks (exit path is
   loader code): `cargo xtask test` and the `LIELLA_FULL_PROFILE=1` hot_text_products pair.

**RULING NEEDED:** order inside a dependency cycle at exit. Recommend Liella's rule (the last
initialised first, the C++ construction/destruction principle; agrees with the reference in E29/E30,
differs in E19/E20/E26-E28), a differences row, no compat option (the reference's in-cycle order
depends on unrelated objects; matching it would mean replicating an emergent tie-break).

**Do not redo:** the probe runs (ref1-ref3 on keke, record committed).
**Process notes:** one empty heredoc and one stray `python3 -` (waited on stdin, killed by PID) slipped
into local commands this run.
