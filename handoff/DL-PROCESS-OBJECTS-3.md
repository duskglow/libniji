# DL-PROCESS-OBJECTS-3: part 3 of 3, TLS past 512 modules (audit HL-3 `MODULES_MAX`)

**State.** Branch `task/dl-process-objects-3` from next `5f07429d3` (parts 1 and 2 landed there). Role keke:~/w/r5
(Zen 1 host). Code commits `255b2a7f7` (part 3, walk guard, tests), `c2c019770` (walk-bound fix, plants, leak
check, record), `c0545d281` (record body on one line), `78ed1180c` (vector-slot check: two plants survived
without it), `2465b7531` (second plants file). Decision: `records/pending/DL-PROCESS-OBJECTS-3-1.toml` (D-NEW,
series D-27, amends D-27.73).

**What part 3 does** (design: handoff/DL-PROCESS-OBJECTS.md section 8; the record has the detail).
- Module table past 512 entries: `THREAD_MODS_PAST` (`Gens<512, 23>`) in boundary `ld/tls.rs`, mapped by
  `reserve_thread_mods(id)` from `dlopen.rs` `tls_room_past` before the walk publishes; `thread_mod` reaches it.
- `ltls::ModuleIds`: free bits past 512 in the caller's words (`Loader::ids_more: Wide<u64>`); no ceiling.
- `ltls::Surplus<S>` generic over its store; the loader's store `tls::Places` (512 inline, then `Wide`).
- dtv: NO header word added (deviation from the design, recorded): the vector in place keeps 512 slots and
  `__tls_get_addr`'s fast path is unchanged; `tls_slow` answers from a grown vector or maps one (`grow_dtv`,
  mapping `[old, words, vector...]`); the walk (`tls_visit`) uses each vector's own room (`dtv_room`);
  `thread_tls_release` -> `release_grown` restores the in-place vector, waits out walks (empty walk, `WALK`), unmaps.
- Walk guard (#7267): `View::find_past` bounded by `Gens::mapped_in` (count from 0; re-read once when reached).
- Tests (crates/ld/tests): `dlopen_process_objects` `dlopen_tls_modules_past_the_vectors_in_place` (1100 modules,
  threads before/after, one access from a new thread to the highest module ID must leave its slot in the thread's
  vector (`%fs:8`), close half + 200 more, churn of 40 growing threads with a VmSize leak check) and
  `dlopen_waits_for_readers_past_three_quarters_of_the_table` (#7380); `tls.rs`
  `tls_surplus_and_module_ids_past_the_in_place_room`.

**Results.**
- Plants: `dl-process-objects-3-plants` (14 plants, c2c019770's tree) was stopped after 7: 5 CAUGHT, 2 SURVIVED
  (dtv-grow, dtv-grow-need: the slow path stays correct without a grown vector, only walking the table on each
  access). Fixed with the vector-slot check (78ed1180c). `dl-process-objects-3-plants2`
  (handoff/DL-PROCESS-OBJECTS-3-plants-2.txt: the two survivors and the 7 not reached) on 2465b7531: 9/9 CAUGHT,
  no WARN-RC-ONLY / WARN-NO-MSG (only the WARN-DEV-SHM note).
- Lint: `dl-process-objects-3-lint2` (clippy niji-ld, niji-boundary), `dl-process-objects-3-lint3` (static green;
  affected: Direct tier, no FULL escalation).
- Gate `dl-process-objects-3-gate` and `dl-process-objects-3-startup` (layout plan and `__tls_get_addr` bytes
  kept in keke ~/work/DL-PROCESS-OBJECTS-3/, then `cargo xtask test`, then fullprof): see the report json.

**Next step (exact).** If the run ended before these jobs: `~/bin/jobwait dl-process-objects-3-gate
dl-process-objects-3-startup` on keke, read `~/logs/<label>.log`; then precheck on the final head and
`ops handback-check`.

**Not to redo.** Parts 1 and 2; the design; the plants (14/14 caught across the two runs).
**Rows to read:** D-27.70, D-27.73, D-27.74; the pending record.
