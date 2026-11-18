# SL-HEADERS-REGISTRY handoff: a header slice adds files, never edits a shared list or function

**State:** branch task/sl-headers-registry (from origin/next de4cdfc8d), head: see handoff/reports/SL-HEADERS-REGISTRY.json.
Tooling only: no generated header content changed (libc-include/ byte for byte the same).

**What changed**
* `headers/slices/<slice>.tsv` (s05, s06, s09, s14): one row per header, columns `header`, `p1`, `uapi`. A file per
  slice, not one sorted `headers/slices.tsv`: in one sorted file two slices whose headers sort next to each other
  insert between the same two lines, which git reports as a conflict; separate files never meet.
  P5 runs on every row, the table P1 units on `p1` 1 (was the SLICE6+SLICE9+SLICE14 lists), P8 on `uapi`
  (was P8_PAIRS).
* `spec/tools/header_checks/<slice>.py`, found by listing the directory: `units(ctx)`, optional `problems(ctx)`.
  s05.py: the C 7.20 identity unit and its missing-typedef FAILs; s09.py: the sys/syscall.h number unit.
  `p1_units(ctx)` takes the loaded tables once; a slice opts in by the `p1` column, no new parameters.
* `xtask/src/headers/slices/<slice>.rs` (a `Slice`: line markers, inline tokens, linked-elsewhere names), listed by a
  new `xtask/build.rs` (sorted names, `#[path] mod` lines in `$OUT_DIR/header_slices.rs`): no shared `mod` line.
  s05.rs: `@@intc`, `@@stubs`, `@@iso10646`; s06.rs: the linked-elsewhere names; s09.rs: `@@syscalls`.
  Core keeps `@@generated`, `@@emit`, `@@owns`, `@@owns-macro`, `@@if`, `@@when`, `@@enum`, `@@value`.
* `headers-gen` (so `--check` in static) and `header_check.py` refuse a slice row whose header has no template, a
  header in two rows, a template in no row (each message names the row, `headers/slices/s09.tsv:19`); headers-gen
  also refuses a marker two slices define or a slice redefining a core marker.
* `plan/headers.md` §4.1 "Adding a slice".

**Proof (mei, role s2)**
* Before (origin/next) and after, `verdicts.sh` (job logs sl-headers-registry-before2, -after1): `headers-gen: ok, 67
  headers`; libc-include tree sha256 9479844757908e57…bf0 both; `header_check: 0 failed of 4563` both; the 52
  NOTE lines (P8, reference also fails) sorted-identical (f219f1f5…); the 4563 units as (configuration, text) pairs
  identical (297aac60…; unit file numbering inside a configuration and P8 order may differ, the set may not).
* Merge demonstration (local scratch worktree, branches deleted): two scratch slices s97/s98 (template, table, check
  module, generator module) from the head; merged with each other and one after the other onto the head: "Merge made
  by the 'ort' strategy" three times, 8 files added, 0 conflicts. Each slice touched 4 new files, no existing one.
* Plants (job sl-headers-registry-plants, head b47ed87a0): 11 plants, 10 CAUGHT, 0 SURVIVED, 1 WARN-NO-MSG
  (reg-missing-row-check: the plants tool's products build runs headers-gen first, which refuses the row, so
  header_check.py's own copy of the row check never ran on the mutant; explained in the plants file).
* Unit tests (xtask headers::tests): 8 passed, two new (slice_modules_are_listed_and_markers_unique,
  slice_table_problems_name_the_row). clippy and static: job sl-headers-registry-lint, rc 0.
* Gate (sl-headers-registry-gate, `cargo xtask affected --run --base origin/next`): the affected tool escalates
  this change to FULL (every `xtask/` file and every `headers/` table counts as "generated table or its input"), so
  it was still running at hand-back (over 50 minutes): pending.

**Moving an open slice branch onto this (for the coordinator's continuation notes)** — after `git merge origin/next`
conflicts in `spec/tools/header_check.py` and/or `xtask/src/headers.rs`:
1. Save the slice's own edits first: `git diff $(git merge-base HEAD origin/next) HEAD -- spec/tools/header_check.py
   xtask/src/headers.rs > <scratch>/slice.diff` (run before the merge, or with `ORIG_HEAD` in place of `HEAD`).
2. Take next's two files: `git checkout origin/next -- spec/tools/header_check.py xtask/src/headers.rs`.
3. Re-express the saved diff as new files only:
   * headers added to `HEADERS`/`SLICE<N>` -> `headers/slices/sNN.tsv` rows `header<TAB>p1<TAB>uapi`; `p1` 1 when the
     branch added the header to the lists `p1_units` filtered on; `uapi` = the kernel headers of its `P8_PAIRS`
     entries, comma-separated, else `-`.
   * its own units (`p8_units`, extra P1 units, a new parameter of `p1_units`) -> `spec/tools/header_checks/sNN.py`,
     `units(ctx)` returning `(config id, label, text)`; data from `ctx` (`cfgs ids vis macros typedefs layouts
     slice_rows rows hp`), never a new parameter.
   * a new `Some("@@x") => ...` arm of `generate()` (and a `Gen` method only it calls, a table only it loads in
     `load()`) -> `xtask/src/headers/slices/sNN.rs`: `Marker { name: "@@x", run: fn(&mut Gen<'_>, &[&str]) }`, the
     table read in the marker with `tsv(g.root, ...)`; an inline replacement -> `Inline`; `LINKED_ELSEWHERE` names
     -> `linked_elsewhere`.
   * a real change to shared generator code (a `Gen` method `@@emit` uses, `load()` columns all slices need) stays an
     edit of headers.rs, in its own commit, named in the hand-back: only that can still conflict.
4. Check: `cargo xtask headers-gen --check` (also fails on a template in no slice row) and
   `python3 spec/tools/header_check.py --include libc-include`: the same `N failed of M` the branch had before the
   merge (M not smaller: a smaller M means a header or unit was lost in the move).

**Not to redo:** the before/after verdicts above; the merge demonstration.
**Next step:** land; then each open slice (S07, S08, S10-S13, S15-S20, VIS-FIX) merges once with the steps above.
**Not verified:** whether `cargo fmt` reaches the slice modules (they are declared only inside the `include!` of
build.rs's output, which rustfmt may not follow); clippy does compile and lint them.
**Improvement for later:** `affected` escalates every slice to FULL (headers/ tables and xtask files). With slices
now confined to `headers/slices/`, `spec/tools/header_checks/` and `xtask/src/headers/slices/`, those paths could map
to a direct tier (headers-gen --check, header_check.py, xtask headers tests) instead.
