# DL-PROCESS-OBJECTS-3: part 3 of 3, TLS past 512 modules (audit HL-3 `MODULES_MAX`)

**State.** Branch `task/dl-process-objects-3` from next `5f07429d3` (parts 1 and 2 landed there). Role keke:~/w/r5
(Zen 1 host). Step 0 (clippy + static on the inherited head): green, job `dl-process-objects-3-lint0`.
Code commits `255b2a7f7` (part 3, walk guard, tests) and `c2c019770` (walk-bound fix, plants, leak check,
record); `c0545d281` makes the pending record's body one line (static's records check). Decision:
`records/pending/DL-PROCESS-OBJECTS-3-1.toml` (D-NEW, series D-27, amends D-27.73).

**What part 3 does** (design: handoff/DL-PROCESS-OBJECTS.md section 8; the record has the detail).
- Module table past 512 entries: `THREAD_MODS_PAST` (`Gens<512, 23>`) in boundary `ld/tls.rs`, mapped by
  `reserve_thread_mods(id)` from `dlopen.rs` `tls_room_past` before the walk publishes; `thread_mod` reaches it.
- `ltls::ModuleIds`: free bits past 512 in the caller's words (`Loader::ids_more: Wide<u64>`); no ceiling.
- `ltls::Surplus<S>` generic over its store; the loader's store `tls::Places` (512 inline, then `Wide`).
- dtv: NO header word added (deviation from the design, recorded): the vector in place keeps 512 slots and
  `__tls_get_addr`'s fast path is unchanged; `tls_slow` answers from a grown vector or maps one (`grow_dtv`,
  mapping `[old, words, vector...]`); the walk (`tls_visit`) uses each vector's own room (`dtv_room`);
  `thread_tls_release` -> `release_grown` restores the in-place vector, waits out walks (empty walk, `WALK`), unmaps.
  In-place vector = `INITIAL_DTV` (one new store in `install_tls`) or `tp - THREAD_TLS_SIZE`.
- Walk guard (#7267): `View::find_past` bounded by `Gens::mapped_in` (count from 0; re-read once when reached).
- Tests (crates/ld/tests): `dlopen_process_objects` `dlopen_tls_modules_past_the_vectors_in_place` (1100 modules,
  threads before/after, close half + 200 more, churn of 40 growing threads with a VmSize leak check) and
  `dlopen_waits_for_readers_past_three_quarters_of_the_table` (#7380: 200 pairs held, a stalled reader, the main
  thread must be seen waiting after <= 2 pairs; it uses live objects past 384 rather than "sets larger than the
  holes": since part 2 a dlopen can always proceed by growing, so the observable is when it waits); `tls.rs`
  `tls_surplus_and_module_ids_past_the_in_place_room`.

**Results so far.** `-t1` (on 255b2a7f7): dlclose 6/6, tls all, process-objects 6/7: the 2000-object test saw
1791/2000 via dladdr/phdr because the walk count started at 512; fixed in c2c019770 (count from 0), NOT yet re-run.
`-t2` (c2c019770): workspace clippy green, static red only on the record body (fixed in c0545d281); so/tests did
not run. `-t3`: killed (sync was BUSY, it ran an old tree): SUPERSEDED.
`dl-process-objects-3-plants` (14 plants, `handoff/DL-PROCESS-OBJECTS-3-plants.txt`, dir
/dev/shm/claude/DL-PROCESS-OBJECTS-3/plants-1) was started on c2c019770's tree and is RUNNING; it holds the role,
so `sync_role.sh` returns 4 (BUSY) until it ends.

**Next step (exact).**
1. `~/bin/jobwait dl-process-objects-3-plants` on keke; every plant CAUGHT (list WARN lines); a survivor: fix
   the test, re-run plants (new dir plants-2).
2. Sync (rc must be 0), then one chained job on the role, e.g. label `dl-process-objects-3-t4`: `cargo xtask
   static && cargo xtask so && cargo test -p niji-ld --test dlopen_process_objects --test tls --test dlclose`.
3. `cargo xtask affected` (no --run) for FULL escalation; then the direct gate (`~/bin/gate 16G ... cargo xtask
   affected --run`), then `cargo xtask test` and fullprof (`NIJI_FULL_PROFILE=1 cargo xtask so &&
   NIJI_FULL_PROFILE=1 cargo test -p niji --test hot_text_products`), one build at a time on the role.
4. Start-up check: `__tls_get_addr` fast path byte-identical and layout-bins unchanged vs next 5f07429d3
   (`target/product/fast/layout-bins.tsv`); install_tls has one new Relaxed store.
5. Foreground workspace clippy, static, `cargo xtask precheck --jobs <labels> --head <sha>`, report json
   (handoff/reports/DL-PROCESS-OBJECTS-3.json), `ops handback-check`.

**Not to redo.** Parts 1 and 2; the design. **Rows to read:** D-27.70, D-27.73, D-27.74; the pending record.
