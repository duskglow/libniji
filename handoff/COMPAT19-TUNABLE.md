# COMPAT19-TUNABLE handoff

State: branch `task/compat19-tunable` (cut from origin/next 50abc44ed). Code, tests, records and plants are done
(legs 1 and 2). Brief: ~/state/briefs/COMPAT19-TUNABLE.md. Role: mei:~/w/s4. Full report:
handoff/COMPAT19-TUNABLE-report.md; json: handoff/reports/COMPAT19-TUNABLE.json.

## Decisions in force
* D-27.66 (this story): `GLIBC_TUNABLES=niji.compat.execstack=1` accepts executable-stack objects (program, start-up
  dependency or preload, `dlopen`), makes the main stack (one mprotect with PROT_GROWSDOWN on the top page of the
  initial stack) and every library-made thread stack executable (libc `__niji_stack_exec`, NIJI_PRIVATE; flag read
  under the thread-list lock in `pthread_create`); a caller-supplied stack is left alone (probe). Secure: ignored.
* COMPAT-19 record: status implemented; the owner's allowlist (ruling 2026-11-20) recorded as an amendment,
  registered, not implemented. Generated views (compat-flags.md, decision views) are train-owned: not regenerated here.

## Done (do not redo)
* Code (leg 1) plus leg-2 fixes found by the new tests: dependency acceptance moved into `open_candidate` (the start
  search's check refused before the tunable was consulted); start refusal text carries the ways in
  (`Refusal::at_start`); program name falls back to argv[0]; `ld.so PROGRAM` refusal uses the program text;
  `dlopen` caller falls back to argv[0] for the main program.
* Tests: `crates/ld/tests/exec_stack.rs` (5 tests, all green on mei, no SKIP: program direct + via ld.so, dependency,
  dlopen after threads, set-group-ID secure, GCC nested-function smoke).
* Records: COMPAT-19.toml, D-27.66.toml, differences row, docs/tunables.md row, docs/RELEASE-NOTES.md "Unreleased".
* Plants: handoff/COMPAT19-TUNABLE-plants.txt (14).

## Jobs (mei)
* `affected` escalates to FULL (xtask/src/so.rs changed: LIBC_PRIVATE gains `__niji_stack_exec`): FULL is the
  train's. Instead: compat19-tunable-crates (cargo xtask so; tests of niji-ld, niji-rt, niji-boundary, xtask;
  niji --lib; s01_startup_clauses), compat19-tunable-xtest (cargo xtask test), compat19-tunable-hot
  (NIJI_FULL_PROFILE hot_text_products), compat19-tunable-plants. Started 07:41Z by ~/work/compat19-tunable/seq.sh.

## Open
* Start-up cost: one relaxed load + branch in `run_init` and one relaxed load per `pthread_create` (under the lock it
  holds). Not counted with wl_percall in this run (RULING NEEDED in the report).

## Results so far (leg 2, 08:20Z)
* compat19-tunable-t3 (so; exec_stack/reloc/preload/ldso_search; niji --lib exec_stack; xtask clippy; xtask static):
  rc 0, 37 passed, no SKIP. compat19-tunable-crates: rc 0, 136 suites, 4481 passed, 0 failed.
* compat19-tunable-plants: 12 CAUGHT, 2 SURVIVED (the two secure-mode plants: a second guard, now `equivalent:`)
  -> SUPERSEDED by compat19-tunable-plants2 (started ~08:12Z from ~/work/compat19-tunable/plants.txt, identical to
  handoff/COMPAT19-TUNABLE-plants.txt; code on the role is the same as the branch's).
* Running at refresh: compat19-tunable-xtest, then compat19-tunable-hot (both via ~/work/compat19-tunable/seq.sh),
  and compat19-tunable-plants2.
* SUPERSEDED: compat19-tunable-t1, -t2 (older code, failures fixed), -rg (records-gen bar in a cell, fixed), -rg2
  (records-gen check, ok), compat19-tunable-plants (2 survivors, rerun).
* Report and json are written and pushed (handoff/COMPAT19-TUNABLE-report.md, handoff/reports/COMPAT19-TUNABLE.json);
  add the plants2/xtest/hot results to the report when they end.

## Next step (leg 3)
1. On mei: `~/bin/jobwait compat19-tunable-xtest compat19-tunable-hot compat19-tunable-plants2` (rc 4 for hot only
   means it has not started yet: wait on xtest/plants2 first). Summaries: `~/bin/test-summary ~/logs/<label>.log`;
   plants: `grep -E ": (CAUGHT|SURVIV|EQUIV|WARN)" ~/logs/compat19-tunable-plants2.log` and its summary line.
2. A red: fix, rerun only that job. Green: update the report's Plants/Tests lines, commit, push.
3. Sync (rc 4 = BUSY while jobs run: wait for them first), then on the role
   `cargo xtask precheck --jobs compat19-tunable-t3,compat19-tunable-crates,compat19-tunable-xtest,compat19-tunable-hot,compat19-tunable-plants2 --head <sha>`
   (check `--help` for the list separator).
4. `python3 ~/src/liella-ops/bin/ops handback-check COMPAT19-TUNABLE --head <sha> --jobs "<same list>"`, then hand
   back COMPLETE with `FULL: the train's (xtask/src/so.rs: xtask (its commands are part of the gate))` and the
   RULING NEEDED line from the report.
