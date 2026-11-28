# XTASK-PRECHECK-SKIP-ASM-AFFECTED

Branch `task/xtask-precheck-skip-asm-affected`; code head cab4e13d3 (commits after it are handoff prose only).
Role keke:~/w/n2. Jobs on cab4e13d3: `xtask-precheck-skip-asm-affected-gate2` (direct gate; FULL is the train's,
xtask changed), `xtask-precheck-skip-asm-affected-plants3` (all 6 plants, one per mapping rule).
Superseded: `xtask-precheck-skip-asm-affected-gate` and `-plants` (green, on b87c9b1be), `-plants2` (4 product-level
plants on 0c0a5847d: 3 CAUGHT, crti.S SURVIVED, which found the wrong crt rule), `xps-asm-affected-gate`,
`xps-asm-affected-plants` (jobrun refused the labels; nothing ran).

## Done
- `cargo xtask precheck --skip <check>,...` (`cheap` names every cheap step; exclusive with `--only`). With a skip of
  `static` and/or `clippy` only, the verdict ends `(skipped: clippy, static; `ops handback-check` runs them on the head
  merged with next)` and is not partial; any other skip says `(partial: --skip ...; not a hand-back precheck)`. Unit
  test `skip_of_the_merged_checks_is_a_hand_back_precheck`. `remez-check` was not bounded (optional; not done).
- `cargo xtask affected`, direct tier only (wide and full unchanged), in `csys_map_in` / `csys_user` / `crt_map`:
  exported symbols not in MACHINE_EXPORTS -> `abi_reach`; trait-method user -> the module as if changed;
  `XTASK_ASM_STEPS` (native_examples.rs -> `cargo xtask native` with products); `LOADER_TARGETS` for loader-only
  files; unreadable files stay FULL and add the namers of the file or its `niji_` names; `csys/start/*.S` -> the
  targets naming the file or its stem (FULL if none); `csys/owner/owner.S` -> those plus every conformance program.
- crt rule changed in this leg (coordinator accepted challenge #8843): conformance programs link `-static -nostdlib`
  with niji-crt1.o / niji-rcrt1.o only (from machine/entry.S), so no csys/start object reaches them.
- Table of all 36 `.S` files before/after: `handoff/XTASK-PRECHECK-SKIP-ASM-AFFECTED-table.md`. FULL before 27, after 6.
- Plants: `handoff/XTASK-PRECHECK-SKIP-ASM-AFFECTED-plants.txt` (6 plants, results of plants3 appended).

## Decisions in force (local)
- The trait-method rule is general (any user file), not only sys.rs: same soundness argument.
- Exported symbols' word occurrences in Rust/C sources are not followed as users; reach is namers + spec section.
- `_Unwind_ForcedUnwind`/`_Unwind_GetCFA` are not in spec/symbols.tsv: selected by namers only.

## Open
- ld_names.S's `niji_machine_ld_strcmp` is called by nothing in libniji (only valgrind calls it); no cargo test checks
  its semantics or its presence in ld.so's symbol table (tools/deb/check-system.sh checks presence in the deb). The
  loader rule's plant is on tool.S instead.
- barrier.S: `cargo xtask native` reads the call site only, so no edit to the barrier's body changes its verdict; the
  plant is caught by a Rust user (s04_strings_symbols sym_explicit_bzero).
- `ops handback-check` could pass `--skip static,clippy` instead of its `--only` list (liella-ops; not this repo).

## Next step
None: gate2 rc 0, plants3 6 of 6 CAUGHT (rc 0); handed back COMPLETE.
