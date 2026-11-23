# LD-OOM-SWEEP-STOP (TEST-REDUNDANCY-AUDIT M6): the loader OOM sweeps stop after two completed runs

State: branch `task/ld-oom-sweep-stop`, cut from origin/next 0495c4684; code commit 0e9bf37c6 (the HEAD named in the
hand-back adds only this note and the report). Owner ruling 2026-11-23 ("Accept recommendations on test fixes": yes,
as the audit recommended; condition: measure the saving from CI gates' nextest lines).

## What changed

`crates/ld/tests/dlopen_new_objects.rs` (`dlopen_new_objects_out_of_memory_is_clean`) and `crates/ld/tests/start_objects.rs`
(`start_objects_out_of_memory_is_clean`) now use dlopen_process_objects' stop rule: every refused limit is swept, the
sweep ends once two runs in a row complete. Each prints the first limit at which the program ran
(`..., N ran, the first at Some(K) KiB`). The reason and the measured share are in each test's doc comment.

## Measurement (before the change, full sweep, mei role s4, `--test-threads 1`, job ld-oom-sweep-stop-measure)

| sweep | limits swept | first completed | stop (2nd in a row) | runs above the stop | refused above the stop | test time |
|---|---|---|---|---|---|---|
| dlopen_new_objects | 189 (1-48 MiB, 256 KiB) | 27392 KiB | 27648 KiB | 84 (44%) | 0 | 15.92 s |
| start_objects | 189 | 24576 KiB | 24832 KiB | 95 (50%) | 0 | 9.02 s |

The runs above the stop are complete runs (the costliest kind), so the time share is at least the run share.
After the change: see the report (gate PASS lines on the role). The CI-side before/after (audit T1 method: median of
the nextest `PASS [ s]` lines of the next CI gates, before: 58.5 s and 27.9 s) can only be taken once this lands:
OWED to the coordinator / next audit.

## Plants

`handoff/LD-OOM-SWEEP-STOP-plants.txt`: the table growth primitive (`grow_map` in `crates/boundary/src/ld/table.rs`,
first mapping and `mremap`) returns a bad address instead of failing, in each sweep; each must be caught below the stop.
Results:
- `ld-oom-sweep-stop-plants` (SUPERSEDED, head 0e9bf37c6): 4 plants on `grow_map` (first map, mremap) x 2 sweeps, all
  SURVIVED: neither sweep's refusals come from a table growth (wrong site; says nothing about the stop rule).
- `ld-oom-sweep-stop-plants-2` (head 96963b846): `start-map-failure-two-lines` (MapError::System text made two lines,
  crates/ld/src/map.rs) CAUGHT by start_objects_out_of_memory_is_clean ("one loader line expected");
  `dlopen-no-memory-two-lines` (NO_MEMORY and NO_ROOM in crates/boundary/src/ld/dlopen.rs given a second `open: ` line)
  SURVIVED. The dlopen sweep's observed `cannot allocate memory` refusal therefore comes from another site (candidates:
  `crates/boundary/src/ld/image.rs:401`, or dlerror text handling that drops the planted line). NOT yet shown: that a
  refusal-path defect is caught below the stop in the dlopen sweep.
- Gate `ld-oom-sweep-stop-gate` (direct tier, head 0e9bf37c6; later commits touch handoff/ only): rc 0, selected
  niji-ld::dlopen_new_objects and niji-ld::start_objects. clippy -D warnings and `cargo xtask static`: clean on mei s4.

## Exact next step

Find the site of the dlopen sweep's `cannot allocate memory` refusal: run the dlopen_new_objects OOM test with
`--nocapture` and one refused limit's stdout, then plant that site (a second `open: ` line) in this plants file and
run `cargo xtask plants` again; then precheck and hand back. If no site below the stop can be planted, the dlopen
sweep's stop has no plant proof: report it.

## Informational

- start_objects' refusal reasons under the limits include `No such file or directory` beside the two memory texts: an
  open that fails for lack of memory may be reported as a missing file at start. Not examined here (outside the story).

## Next step

None for this story once green; the CI before/after measurement is owed after landing.

## Report

See `handoff/reports/LD-OOM-SWEEP-STOP.json` and the results section below.
