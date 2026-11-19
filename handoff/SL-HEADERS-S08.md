# SL-HEADERS-S08: slice 8, signals, threads, contexts (handoff after run 5, SL-HEADERS-S08-5, merge-only)

**State.** Branch `task/sl-headers-s08-5` (run 4's branch plus origin/next merged). Head: see `handoff/reports/SL-HEADERS-S08-5.json`.
`headers-gen` writes 94 headers; `header_check.py` 0 failed of 6435 (all slices; run 4 had 0 of 5382). Say "slice 8" in commits.

**Done in run 5 (merge only, no new header content).**
* Merged origin/next. next had split `xtask/src/headers.rs` into `xtask/src/headers/*.rs` (aggregates, cond, rows, values, ...), with its own
  `@@init`, function-pointer members, packed/aligned aggregates and pointer literals. Run 4's monolith edits for those were dropped in favour of
  next's modules; nothing slice-8-only stays in the shared emitter except what is listed next.
* Four small generic generator edits slice 8 still needs, now on next's modules: `LANGUAGE_MODE` (`max_align_t`) and `union_tag`/`cxx_mask`
  (a union typedef without a C++ spelling is declared in C++, `pthread_attr_t`) in `headers/cond.rs`; an include right after an `#if` line is
  conditional (`headers.rs`, `sched.h` declares `size_t`). `fmtmsg.h.in` takes the `MM_NULL*` pointer values from the table with `@@value`.
* Differences doc: every row of both sides kept (next had no rows where run 4's three sit). `headers/reserved.tsv`: both lists. `header_probe.py`:
  `standard_names()` holds both lists (slice 17's and slice 8's). Generated headers and `appendix_c_layouts.rs` regenerated, not hand-merged.
* Plants file `handoff/SL-HEADERS-S08-4-plants.txt`: paths of three plants follow the split (cond.rs, values.rs).

**Not to redo.** The member emitter work: it is next's `aggregates.rs`. No second `LANGUAGE_MODE` or union-tag code anywhere else.

**Open.** `macros.tsv` has five TTYDEF_* rows in pty.h that type_probe.py does not reproduce (slice of pty.h). P6 (`conformance/run.sh
--niji-headers`) was not rerun in this merge-only run: the 45 of 47 of run 4 are on the old head. A full layouts regeneration would re-add
`__sigset_t` rows (abi_extract.py TYPES lists it).

**Next step.** None for slice 8 beyond the train's FULL gate and landing.
