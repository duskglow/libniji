# PLANTS-PRODUCTS-ONCE (handoff, leg 1 of the run)

State: branch `task/plants-products-once` (from origin/next 45f916b8d). Role mei:~/w/s1. Code: `xtask/src/plants.rs`,
`xtask/src/affected.rs` (`unit_tests_may_read_products`). Leg 1 commit is WIP: written, NOT YET COMPILED (no cargo on
the agent host); scratch scripts in the run's scratch dir (`measure.py`, `agg.py`, `paths.py`, `measure.tsv`).

## Step 1: measurement (plants logs 11-24..26 on mei, keke, shiki; jobs.tsv wall; read-only)

| | value |
|---|---|
| plants jobs | 155 (keke 119, mei 24, shiki 12); wall sum 80.6 h |
| product builds on each job's critical path (longest copy: baseline + mutant + restore builds) | 60.5 h (75 %) |
| jobs where no plant read the products (every build unused, by the tool's own per-plant line) | 32 (21 %): 6.8 h of their 9.5 h wall |
| jobs where one copy's plants read none (that copy's baseline unused) | 6 (5415 cpu-s) |
| mutant builds / restores | 775 (40.7 h) / 19 (0.8 h) |
| median wall / median build path | keke 30.7 / 23.2 min; mei 14.5 / 11.3; shiki 24.6 / 16.9 |
| two-copy jobs, baseline build per copy (median) | 908 s (both copies build it at the same time) |
| plants on test-only paths over all 903 committed plants files | 119 of 5804 (2 %); 21 files only such plants |

Examples: affected-layout-rows-plants3 (mei, xtask unit tests only): 544 s wall, 344+344 s baseline builds, 0 read.
Header stories (headers-stage2-gaps-d-5, mei, 1835 s; d-6 on keke ~4050 s): every plant's command is
`sh -c 'cargo xtask headers-gen && python3 spec/tools/header_check.py ...'` (shell syntax, so "may use") or
`cargo test -p xtask --bin xtask layout_offsets`: 16 full rebuilds, none read (header_check.py compiles against
libc-include/ with -nostdinc -fsyntax-only; header_checks/*.py name no products).

## Design (in the WIP commit)

- `simple_commands`: a `sh -c '<list>'` or bare list of simple commands (`&&`, `||`, `;`, quoted words) judged one by
  one; any other shell syntax -> None (may use).
- `RUNS_NO_PRODUCTS`: `cargo xtask headers-gen`, `python3 spec/tools/header_check.py`, `python3
  spec/tools/header_checks/<x>` (no argument naming products) read none.
- `Need { reads, mutant }` per plant (`needs`): reads = uses || unit tests may read (`unit_tests_may_read_products`:
  xtask by `product_unit_tests` against libtest filters; other crates: src names products) || traced; mutant = (uses
  || traced) && `may_change_products(path)` (false only under crates/*/{tests,benches,examples}/ and xtask/tests/).
- `assign`: mutant plants dealt over min(copies, #mutant) builder copies; read-only plants join builders (one
  builder when no mutant plant); no-read plants fill non-builder copies. Copy builds baseline only if one of its
  plants reads (per layout). Restore rule: a reads plant needs held unmutated products in its layout.

## Exact next step

1. Sync, `cargo clippy -p xtask --all-targets -D warnings`, fix compile errors; `cargo test -p xtask --bin xtask plants::`.
2. Unit tests to add: `simple_commands` cases; header chain -> no use; `may_change_products`; `needs`; `assign`
   (mutant plants never in a non-building copy; read-only plants never add a builder); a toy run where a copy whose
   plants read nothing builds nothing.
3. Plants for the new rule (docs: brief step 4: a plant making `may_change_products` false for `crates/x/src` must be
   caught; one sending a mutant plant to a non-building copy).
4. Proof on mei: before (origin/next tool) / after on AFFECTED-LAYOUT-ROWS-plants.txt, a header plants file that
   applies to the tree, a libm plants file; same verdict per plant.

Slip to report: one `sed -i` on a scratch file (not the repo) in leg 1.
