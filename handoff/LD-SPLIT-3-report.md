# LD-SPLIT-3 report: merge next into the loader split

Branch `task/ld-split-3`: LD-SPLIT-2's branch (`50f9251c1`) plus a merge of origin/next `d9c350f22`. Role keke `~/w/n3`.

## Why

CI could not merge `task/ld-split-2` (CI CONFLICT in crates/boundary/src/ld.rs). COMPAT19-TUNABLE had landed on next
and added the COMPAT-19 executable-stack code to ld.rs while the split waited.

## Commits

1. `03318db34` merge origin/next. ld.rs was resolved by rerunning `tools/ld_split.py` on next's ld.rs, not by
   editing the conflict. The other files merged cleanly (ld/dlopen.rs included). The eight new items, `EXEC_STACK`,
   `PROT_GROWSDOWN`, `exec_stack_accepted`, `refuse_exec_stack_program`, `exec_stack_dependency`,
   `exec_main_stack`, `exec_stacks_at_start` and `exec_stack_candidate`, are one concern (COMPAT-19). They went to a
   new module, `ld/exec_stack.rs`, and the generator got that module in MODS/ORDER. ld.rs's `//!` module map gets
   its line. `exec_main_stack` is still reachable at its old path through `pub(crate) use self::exec_stack::exec_main_stack`,
   because dlopen.rs calls `super::exec_main_stack`. Next's edits to existing items (`run`, `run_init`,
   `open_candidate`, `own_object`/`load`, `link` and others) moved with those items into entry, initfini, image, link
   and trace.
2. `73b8576c9` handoff/LD-SPLIT-3-plants.txt: COMPAT19-TUNABLE's six non-equivalent plants on ld.rs, re-pointed at
   the files their code now lives in (entry, exec_stack, image, initfini), with the same tests and messages.
3. `240854e10` the merge's split did not compile on its own. exec_stack.rs reads `List.ld`, which was private to
   entry.rs, and the implied-import rule added an unused `Arch`. The generator now makes `List.ld` `pub(super)`
   (`secure` already was) and drops `Arch` for exec_stack. The files were regenerated from the generator.

## Proof (head 240854e10, keke)

| check | result |
|---|---|
| step 0 on the inherited head 50f9251c1: clippy (workspace, all targets, all features, -D warnings) + static | `ld-split-3-step0` rc 0 |
| items moved, none changed: every top-level item of next's ld.rs vs the split (ld.rs + 9 modules), ignoring `pub(super) ` and use/mod/`//!` lines | 135 in next, 135 in the split, 0 only on one side |
| unsafe-audit markers under ld.rs + ld/, next vs head | allow(unsafe_code) 70/70, `unsafe {` 59/59, unsafe(no_mangle) 10/10, UNSAFE( 71/71, SAFETY: 60/60, unsafe fn 2/2, unsafe impl 1/1, inline(never) 16/16 |
| clippy -D warnings + static + affected list | `ld-split-3-lint-2` rc 0 (`ld-split-3-lint` on 73b8576c9 found the two compile errors that commit 3 fixes) |
| direct-tier gate (`cargo xtask affected --run --base origin/next`) | `ld-split-3-gate` rc 0: 9 suites, 633 passed, 0 failed; peak 1.8G, oom_kill 0 |
| plants (`cargo xtask plants handoff/LD-SPLIT-3-plants.txt`) | `ld-split-3-plants` rc 0: 6 CAUGHT, 0 SURVIVED, 0 WARN-RC-ONLY, 0 WARN-NO-MSG (one WARN-DEV-SHM: no plant command here mounts /dev) |
| start-up hot text (`NIJI_FULL_PROFILE=1 cargo xtask so && ... --test hot_text_products`) | `ld-split-3-hottext` rc 0: 30 passed |
| exported symbols of every shared product vs origin/next d9c350f22 (both NIJI_FULL_PROFILE=1, release) | `ld-split-3-compare`: 17 same, 0 differ |
| dynamic start-up instructions:u (ASLR off, perf stat -r 10, products at paths of equal length) | ret 1,069,447 base / 1,069,448 head; printf 1,083,185 = 1,083,185; dlopen(libm)+dlsym+dlclose 1,713,156 = 1,713,156. The 1-instruction difference in a mean of 10 is inside the run-to-run spread run 2 measured for these probes (about 3) |
| release `layout-bins.tsv`, base vs head | identical |
| FULL `cargo xtask test` | not run: owner 11-19, the CI train is the FULL gate |

## Notes

- The generator does not write ld.rs's `//!` module map (run 2 added it in a separate doc commit). After a rerun,
  restore it. The generated ld.rs then differs from the committed one only by those lines (checked here: 23 lines,
  no other change).
- INFORMATIONAL: the direct tier for ld/*.rs selects the niji-boundary unit tests and their one-hop user, not
  `niji-ld --test exec_stack`. The plants run that test (their baseline passed on this head). This is how the
  affected map is today, and this split does not change it.
