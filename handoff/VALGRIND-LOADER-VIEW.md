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

## VALGRIND-LOADER-VIEW-2 (verdict run; branch `task/valgrind-loader-view-2`, role mei `~/w/s2`, Skylake-SP)

* The red `valgrind-loader-view-gate` (keke, rc 1: only `cargo xtask static`, records: D-X.152 body had a newline)
  ran on a tree synced at 19:55, before commit 74f532495 (19:57) wrote D-X.152 in its one-line form. On the
  inherited head c892ed421 `cargo xtask static` (records, records-gen) and clippy (`--all-features -D warnings`)
  pass with no change; D-X.153 is one line too. No code or record changed in this run.
* Verdict jobs on this head (mei): `valgrind-loader-view-2-gate` (`cargo xtask affected --run`, Direct tier),
  `valgrind-loader-view-2-plants` (handoff/VALGRIND-LOADER-VIEW-plants.txt), `valgrind-loader-view-2-startup`
  (`cargo xtask test`, full-profile so, hot_text_products; it waits for the gate). Results: gate rc 0, 16 suites
  ok, 10261 passed, 0 failed, 1501 ignored, conformance 35/35, SKIPPED 0, peak 14G; plants 7 of 7 CAUGHT by their
  messages, no WARN lines; start-up all green (xtask test 7019 + 4862 + 128 + 46 passed, hot_text_products 33
  passed). Report record: handoff/reports/VALGRIND-LOADER-VIEW-2.json.
* Ruling #7385 resolved: AFFECTED-COVERAGE runs the `ld` unit tests; the valgrind-only path is accepted.

## VALGRIND-LOADER-VIEW-3 (merge run; branch `task/valgrind-loader-view-3`, role mei `~/w/s2`, Skylake-SP)

* CI could not merge next (conflict in `xtask/src/so.rs` after VALGRIND-STRING-REDIRECT landed). Merged origin/next
  9548c734e (commit 6d3bf87b4): both conflicts were adjacent additions of independent functions and their tests
  (this branch's versioned-default aliases, next's `check_valgrind_string_names`); both kept unchanged. No design
  decision was needed. The string-names check runs right after the link, before the aliases are added; an alias is
  a copy of its function's `.symtab` entry (same start and size), so valgrind's view merges the names and nothing
  moves (the acceptance below confirms it on the final product).
* Step 0 on the inherited head 12a0d4683: clippy (`--all-features -D warnings`) and `cargo xtask static` green.
  Same on the merged head.
* Jobs on the merged head (mei, job scripts `~/work/VALGRIND-LOADER-VIEW-3/`):
  `valgrind-loader-view-3-gate` (direct tier, 19 suites ok, 10408 passed, 0 failed, 1501 ignored, conformance
  37/0, SKIPPED 0, peak 14G); `valgrind-loader-view-3-plants` (7 of 7 CAUGHT, no WARN);
  `valgrind-loader-view-3-acc` (system debs from the head, check-system PASS 7 packages, then both swapped-root
  acceptances with trixie's valgrind 3.24 and the whole libc6-dbg); `valgrind-loader-view-3-startup`
  (`cargo xtask test`, full-profile so, hot_text_products).
* Acceptance, VALGRIND-LOADER-VIEW (`vgacc`): 8 of 8 PASS. The leak record runs malloc, vasprintf (printf.rs),
  `__vasprintf_chk`, asprintf, main (leak.c:9), no unnamed frames; helgrind thr, thr_old 0 errors, race 2 races at
  race.c, all as on stock. Still reachable on leak: 4,104 bytes in 1 block (stock 0), the same as the pre-merge run
  on keke: unchanged by the merge.
* Acceptance, VALGRIND-STRING-REDIRECT (`acc`): ls /, ls -l /etc, find /etc -name passwd, sort /etc/passwd and every
  replaced string function: 0 memcheck errors each, as on stock; the deliberate overread: 1 error in strlen, as on
  stock.

## Next step
None for this story beyond landing.
