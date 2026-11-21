# VALGRIND-LOADER-VIEW: full leak stacks through libc under valgrind, and helgrind (GitHub #13, the last of it)

Branch `task/valgrind-loader-view` (from origin/next d5764940d). Brief `~/state/briefs/VALGRIND-LOADER-VIEW.md`.
Role keke `~/w/n3` (Zen 1 host). Job scripts on keke `~/work/VALGRIND-LOADER-VIEW/` (copies in
`~/w/.scratch/VALGRIND-LOADER-VIEW/job/`). Full report: [VALGRIND-LOADER-VIEW-report.md](VALGRIND-LOADER-VIEW-report.md).

## State (leg 3, the last leg)

Done: item 1 (D-X.152, valgrind-only hidden whole-file views, option (b)); item 2 (D-X.153, libc.so.6's defaults
with a compat version beside them named `name@@VERSION` in `.symtab`). Acceptance in a swapped trixie root with
trixie's valgrind and the whole libc6-dbg: 8 of 8 PASS (memcheck leak record reaches `main`, no `???` frames;
helgrind thr, thr_old and race equal to stock, race reports 2 races at race.c).

Decisions in force: D-X.152, D-X.153 (records/decisions/D-X/).

## Do not redo
* The (a)/(b) start-up tables (in D-X.152; logs `~/logs/valgrind-loader-view-measure-a4.log`, `-measure-b.log`).
* The swapped-root acceptance (`~/logs/valgrind-loader-view-leg3.log`, `=== VGACC pass=8 fail=0`).
* The layout-bins diff base-views vs head: only LLVM hash suffixes differ (`~/work/VALGRIND-LOADER-VIEW/layout.diff`).

## Open
* RULING NEEDED: the loader module's unit tests (`crates/boundary/src/ld`, feature `ld`) are built by no gate
  or CI runner (the library tests use `export,selftest`); D-X.152's test runs only with
  `cargo test -p niji-boundary --lib --features ld`. Recommend: `cargo xtask affected` adds that run when a
  file under `crates/boundary/src/ld/` changes (a tooling story).

## Next step
None for this story beyond landing; see the report's hand-back section for the jobs.
