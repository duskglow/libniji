# DL-TRACE-INTERP: the loader's trace list matches the reference

**State.** Branch `task/dl-trace-interp-2` (DL-TRACE-INTERP-2, continuing `task/dl-trace-interp`). Code head
`a389fd4e2` (merge of origin/next `ff292f6af`, test `30ff851ec`, plants file `a389fd4e2`); this note and
`handoff/reports/DL-TRACE-INTERP-2.json` are the commits after. Role keke `~/w/n1`; job scripts in keke
`~/work/DL-TRACE-INTERP-2/` (gate.sh, debs.sh, swap.sh; not in the tree). Full report: `handoff/DL-TRACE-INTERP-report.md`.

## Decisions in force

D-27.16 as amended by **D-27.56** (the trace list's interpreter rule), D-31.15 (ldd; interim filter of ruling #6043
removed, now cites D-27.56). `docs/differences-from-glibc.md` `ldd -v` row (libc/libm version-need blocks; no compat
option, reason in the row). Spec: `spec/behaviour/dynlink.md` "Trace lists".

## Done (DL-TRACE-INTERP)

* Probes (reference loader as a black box): interpreter listed only when an object names it in `DT_NEEDED`, at its
  breadth-first place; vDSO first, after the interpreter when that comes first; `NAME => PATH` when the program's
  `PT_INTERP` differs from the loader's path as run; library operand: the loader's path as run; no `DT_NEEDED`:
  `statically linked`, status 0.
* `crates/boundary/src/ld.rs` `list()` follows it for all entry points; libniji's own libraries name the interpreter
  after their own needs. `csys/util/ldd.c` interim filter removed. System build pins libc/libm under
  `/lib/x86_64-linux-gnu` (`tools/deb/build.sh --flavour system`). Test `list_interpreter_line_matches_reference`.

## Done (DL-TRACE-INTERP-2)

* CI red (static old-name: one `Liella` in spec/behaviour/dynlink.md) fixed by merging origin/next and writing libniji;
  the spec note and D-31.15 cite D-27.56 instead of the pending file. Conflict in the differences table: next's rows
  kept, the `ldd -v` row kept and given the reference column it lacked (5-column table).
* `tools/deb/build.sh`: the system build line read `NIJI_LIBDIR=$libdirNIJI_FULL_PROFILE=1` (one assignment of an
  unset variable: empty libdir, full profile off); split back into two assignments.
* The handoff's gap closed: the test now also runs `--list` with each side's loader through a symlink, so
  `NAME => PATH` is compared whole; two plants on it (`handoff/DL-TRACE-INTERP-2-plants.txt`, 8 plants).

## Results

See the report. Swap root (keke, trixie): verdict requires ldd-1..5 and every other command identical with NO
normalisation, ldd-6/7 (`-v`) identical once only libc.so.6/libm.so.6's version-need blocks are dropped (the row).

## Open / next step

Nothing in this story once the named jobs are green. Do not redo: the probes, the merge, the plants.
Read: spec/behaviour/dynlink.md "Trace lists", D-27.56, D-31.15, `handoff/DL-TRACE-INTERP-report.md`.
