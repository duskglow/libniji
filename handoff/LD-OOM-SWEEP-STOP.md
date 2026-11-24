# LD-OOM-SWEEP-STOP (TEST-REDUNDANCY-AUDIT M6): the loader OOM sweeps stop after two completed runs

State: branch `task/ld-oom-sweep-stop`, cut from origin/next 0495c4684. Code commits 0e9bf37c6 (the stop rule) and
9eacaa2a1 (the dlopen sweep counts its refusals per text); later commits touch handoff/ only. Owner ruling 2026-11-23
("Accept recommendations on test fixes": yes, as the audit recommended; condition: measure the saving from CI gates'
nextest lines).

## What changed

`crates/ld/tests/dlopen_new_objects.rs` (`dlopen_new_objects_out_of_memory_is_clean`) and `crates/ld/tests/start_objects.rs`
(`start_objects_out_of_memory_is_clean`) now use dlopen_process_objects' stop rule: every refused limit is swept, the
sweep ends once two runs in a row complete. Each prints the first limit at which the program ran. The dlopen sweep
also prints, for each refusal text, how many limits gave it and the first of them. Reason and measured share are in
each test's doc comment.

## Measurement (before the change, full sweep, mei role s4, `--test-threads 1`, job ld-oom-sweep-stop-measure)

| sweep | limits swept | first completed | stop (2nd in a row) | runs above the stop | refused above the stop | test time |
|---|---|---|---|---|---|---|
| dlopen_new_objects | 189 (1-48 MiB, 256 KiB) | 27392 KiB | 27648 KiB | 84 (44%) | 0 | 15.92 s |
| start_objects | 189 | 24576 KiB | 24832 KiB | 95 (50%) | 0 | 9.02 s |

After the change (job ld-oom-sweep-stop-reasons): the dlopen sweep runs in 5.93 s, 102 refused, 2 ran. Its dlopen
refusals: `cannot read file data` at 53 limits (from 13568 KiB), `cannot allocate memory` at ONE limit (18688 KiB);
the rest are start failures (status 127). The CI-side before/after (audit T1: median nextest `PASS [ s]` of the next CI
gates; before 58.5 s and 27.9 s) can only be taken after landing: OWED to the coordinator / next audit.

## Plants (`handoff/LD-OOM-SWEEP-STOP-plants.txt`, final run ld-oom-sweep-stop-plants-5)

- `dlopen-view-map-failure-two-lines` (crates/boundary/src/ld/dlopen.rs, the candidate file's view mapping failure,
  the sweep's `cannot read file data`): CAUGHT ("one dlopen failure with its text").
- `dlopen-start-map-failure-two-lines` and `start-map-failure-two-lines` (crates/ld/src/map.rs MapError::System text
  given a second line) against each sweep: both CAUGHT ("one loader line expected"). Run total: 3 CAUGHT, 0 WARN.
- SUPERSEDED: ld-oom-sweep-stop-plants (grow_map: wrong site, all survived); -plants-2 (NO_MEMORY/NO_ROOM survived:
  reached at one limit only, see below); -plants-3 (Refusal::Truncated survived: not the site, text unchanged; stopped
  early); -plants-4 (started on an unsynced role, stopped).
- Gate ld-oom-sweep-stop-gate-2 (direct tier, --base origin/next, head dd9c78446 = final code): rc 0, selected
  niji-ld::dlopen_new_objects and niji-ld::start_objects. clippy -D warnings and `cargo xtask static`: clean on mei s4.

## Informational

- The dlopen sweep reaches `<file>: cannot allocate memory` (NO_MEMORY / NO_ROOM) at one limit of 189 (18688 KiB),
  before and after the stop, so whether a rebuilt build reaches it depends on its layout: a plant there survived. A gap
  of the 256 KiB step, not of the stop; a finer step between the first refusal and the stop would cover it (not done).
- start_objects' refusal reasons include `No such file or directory` beside the memory texts: an open failing for lack
  of memory may be reported as a missing file at start. Not examined (outside the story).

## Next step

None for this story once green; the CI before/after measurement is owed after landing.
