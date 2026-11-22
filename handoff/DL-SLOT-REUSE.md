# DL-SLOT-REUSE: reuse freed loader slots below the top (run 2: merge with next)

**State.** Branch `task/dl-slot-reuse-2` (continues `task/dl-slot-reuse`), role keke:~/w/n4. Run 1's code is
unchanged; run 2 merged origin/next (`7a582470f`) to clear the train's conflict in `crates/boundary/src/ld/dlopen.rs`.
Decision D-27.74 (amends D-27.73). Run 1's full report with the churn table: `handoff/DL-SLOT-REUSE-report.md`;
plants: `handoff/DL-SLOT-REUSE-plants.txt`.

**The merge (run 2).** The only conflict was in `struct Loader`: next added the program's `$ORIGIN` state
(`origin_tried`, `origin_len`, `origin`, DL-RUNPATH) right where this branch added `holes` / `reordered`. Both sets
of fields are kept. The Loader is zero-initialised, so no constructor needed merging. I checked next's new code
against slot reuse; it needed no design decision:
- `by_given` (bare-name `dlopen` matching) skips `gone` slots. `settle` marks the top slots it vacates that way
  (`free_slot`).
- The origin is per process, not per slot.
- `given_name` rewrites only start-time records (below `n_start`), and `settle` never moves those.

**What the change does (`crates/boundary/src/ld/`, run 1).**
- `dlopen.rs` `load_set`: the set is found and staged at the top as before. When `Loader::holes` (freed slots
  below `n`) >= the set's size, the cold `settle()` moves it into the first run of freed slots that fits. Slots are
  past the readers' grace period. Nodes, records, reader entries and dependency lists are renumbered.
- Load order is kept by `Loader::reordered` / `readers::REORDERED`: the chain tail, the exit order and
  `dl_iterate_phdr` follow the chain.

**Rulings in force.** #7379: accept `dl_iterate_phdr` +4 instructions a call (DL-FIND-OBJECT-FAST removes them).
#7380: the three-quarters-wait test goes into DL-PROCESS-OBJECTS-3.

**Run 2 jobs (keke).** lint1 (clippy -D warnings + static), gate1 (direct), loader1 (runpath_search, dlclose,
dlopen_process_objects), plants1 (6 plants), xtest1 (`cargo xtask test`), hottext1 (NIJI_FULL_PROFILE
hot_text_products). Results are in `handoff/reports/DL-SLOT-REUSE-2.json` and the hand-back.

**Do not redo.** Run 1's churn table, counts, layout diff and D-27.74. The merge does not change this branch's code,
so its counts carry over (in the report as not re-measured).

**Next step.** A follow-up story: first-fit gap placement of path and dependency spans, with the same grace-period
rule, so the address space under churn is bounded too (it still grows ~100 B per open).

Plan rows to read: D-27.73, D-27.74.
