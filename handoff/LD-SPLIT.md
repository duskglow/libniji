# LD-SPLIT / LD-SPLIT-2 / LD-SPLIT-3: the loader's ld.rs split into modules by concern (run 3: merged with next, done)

**State.** Branch `task/ld-split-3` (LD-SPLIT-2's commits plus a merge of origin/next `d9c350f22`). Code head
`240854e10`; later commits touch handoff/ only. Role keke `~/w/n3`. Report for this run:
handoff/LD-SPLIT-3-report.md (JSON handoff/reports/LD-SPLIT-3.json). LD-SPLIT-2's report (module map, the split's
own proofs, the start.rs / so.rs proposal): handoff/LD-SPLIT-2-report.md.

**What run 3 did.** CI could not merge LD-SPLIT-2 because COMPAT19-TUNABLE had changed ld.rs on next. The merge
conflict was resolved by rerunning `tools/ld_split.py` on next's ld.rs (ruling #6193), with no hand edits to the
conflict. COMPAT-19's eight new items went to a new module, `ld/exec_stack.rs`. Next's edits to existing items
moved with those items. The generator also got two fixes, List.ld made `pub(super)` and `Arch` dropped for
exec_stack. ld.rs's module map lists the new module.

**Decisions in force.** Ruling #6193 (regenerate on the next being merged). Proof: every item of next's ld.rs is in
the split exactly once, byte-identical apart from `pub(super)` (135/135).

**Proved on 240854e10 (keke):** `ld-split-3-step0` (inherited head: clippy+static rc 0); `ld-split-3-lint-2`
(clippy -D warnings, static, affected list); `ld-split-3-gate` (9 suites, 633 passed); `ld-split-3-plants`
(6/6 CAUGHT, COMPAT19-TUNABLE's ld.rs plants re-pointed at their new files); `ld-split-3-hottext` (30 passed);
`ld-split-3-compare` (17/17 shared products export the same symbols as origin/next; start-up instructions equal:
ret 1,069,447 vs 1,069,448 (mean of 10, within the run-to-run spread run 2 measured), printf and dlopen identical;
release layout-bins.tsv identical to base).

**Not run:** FULL `cargo xtask test` (owner 11-19: the CI train is the FULL gate); swap root (the same check as
run 2, and the merge only moves items).

**Do not redo:** the split, the merge resolution, the proofs above, the base build (keke `~/work/ld-split-3/base`).

**Next step:** land it. The held loader stories (DL-NAME-PATH-2, DL-PROCESS-OBJECTS, DL-RUNPATH, DL-RDEBUG-VERSION,
DL-OBJECT-PATHS) then rebase onto the modules. Any loader story that lands before this one: rerun
`LD_ORIG=<next's ld.rs> python3 tools/ld_split.py . 9`, give new items a module in MODS, restore ld.rs's `//!`
module map (the generator does not write it), then rerun the 135-item check (scratch `verify_moves.py`). Later:
split crates/rt/src/start.rs and xtask/src/so.rs as LD-SPLIT-2's report proposes.

Scratch: `~/w/.scratch/LD-SPLIT-3/work/` (ld.next2.rs, verify_moves.py, compare.sh, base_build.sh, logs).
