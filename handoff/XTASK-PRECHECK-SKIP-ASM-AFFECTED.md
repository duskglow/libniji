# XTASK-PRECHECK-SKIP-ASM-AFFECTED

Branch `task/xtask-precheck-skip-asm-affected`; code commit b87c9b1be (handoff commits after it are prose only).
Role keke:~/w/n2. Jobs on b87c9b1be: `xtask-precheck-skip-asm-affected-plants` (2 plants, both CAUGHT, rc 0);
`xtask-precheck-skip-asm-affected-gate` (direct gate; still running at the refresh: jobwait it first). Superseded:
`xps-asm-affected-gate`, `xps-asm-affected-plants` (jobrun refused the labels; nothing ran).

## Done
- `cargo xtask precheck --skip <check>,...` (`cheap` names every cheap step; exclusive with `--only`). With a skip of
  `static` and/or `clippy` only, the verdict ends `(skipped: clippy, static; `ops handback-check` runs them on the head
  merged with next)` and is not partial; any other skip says `(partial: --skip ...; not a hand-back precheck)`. Unit
  test `skip_of_the_merged_checks_is_a_hand_back_precheck`. `remez-check` was not bounded (optional; not done).
- `cargo xtask affected`, direct tier only (wide and full unchanged), in `csys_map_in` / `csys_user` / `crt_map`:
  exported symbols not in MACHINE_EXPORTS -> `abi_reach` (targets naming it, programs calling it, targets and programs of
  its `spec/symbols.tsv` section; FULL if none); trait-method user -> the module as if changed; `XTASK_ASM_STEPS`
  (native_examples.rs -> `cargo xtask native` with products); `LOADER_TARGETS` for a file so.rs links into ld.so with no
  source user, or used only by `crates/boundary/src/ld/`; unreadable files stay FULL and add namers of the file or its
  `niji_` names (`csys_labels`); `csys/start/*.S`, `csys/owner/owner.S` -> every conformance program + namers.
  `sym_refs`: a C `extern T name;` line is a declaration (points.S escalated on niji_csys.h).
- Table of all 36 `.S` files before/after: `handoff/XTASK-PRECHECK-SKIP-ASM-AFFECTED-table.md`. FULL before 27, after 6
  (the two generated compat_aliases.S; context.S, entry.S, sqrtl.S, tlsdesc.S unreadable, by the brief).

## Decisions in force (local, mine)
- The trait-method rule is general (any user file), not only sys.rs: the soundness argument is the same (it is the
  selection a direct edit of the method's body gets). It also maps call.S, thread_start.S and fpu.S's mathret.rs.
- Exported symbols' word occurrences in Rust/C sources are not followed as users (`read`, `close` are everywhere);
  reach is by namers + spec section.
- `_Unwind_ForcedUnwind`/`_Unwind_GetCFA` are not in spec/symbols.tsv: selected by namers only.

## Open / not done
- Plants for 4 of the 6 rules are missing: xtask step (barrier.S), loader (ld_names.S / tool.S), unreadable
  (tlsdesc.S), crt (csys/start/crti.S). Each needs the products (ld.so, examples, conformance), minutes per plant.
- Exact next step: add to `handoff/XTASK-PRECHECK-SKIP-ASM-AFFECTED-plants.txt` one plant each (labels:
  `niji_machine_ld_strcmp` -> a niji-ld `ldso_*` test; `niji_machine_running_on_valgrind` -> a niji-ld test that
  loads under ld.so; `niji_machine_barrier` -> `cargo xtask native`; crti.S `_init` prologue -> one conformance
  program via `cargo xtask so` + `cargo xtask conformance`), run `cargo xtask plants` with `--timeout 1500`.
- `ops handback-check` could pass `--skip static,clippy` instead of its `--only` list (liella-ops; not this repo).
