# XTASK-AFFECTED-EXAMPLES: a change to a crate's examples selects the step that runs them

State: branch `task/xtask-affected-examples`, code head `1b8e7ddf1` (on origin/next `ebb670553`), plus this note
and the report json. Decision: pending record `records/pending/XTASK-AFFECTED-EXAMPLES-1.toml` (D-NEW, series D-X;
CI assigns the id).

## What changed (xtask/src/affected.rs)

- `example_steps(repo, dir, f)`: for a file under `<crate>/examples/`, derives the steps from what uses it, not
  from a list: (1) the examples it belongs to (`examples/<n>.rs`; a module directory `examples/<d>/` -> every
  example declaring `mod <d>;`; another top-level file -> every example naming it); (2) the xtask sources (not
  affected.rs) naming such an example, as `"<n>"` or `examples/<n>.rs`; (3) the `t.time("<step>", || <m>::<fn>(..))`
  step of `xtask/src/main.rs` calling that module and the arm `"<cmd>" => <m>::<fn>(` running it alone.
  A file no example uses, or a test step with no arm to run it alone, is FULL.
- `classify_path`'s examples/benches branch keeps the crate's clippy and adds those commands (with a note).
  `Selection::example_steps` makes `needs_products` true (`cargo xtask so` first: both steps link the products).
- `classify` no longer sends all of `crates/native/examples/` to FULL.
- `Repo::examples`: the top-level `<crate>/examples/*.rs` sources (for the `mod <d>;` lookup).

Mapping today (step 1 of the brief, found by grep over xtask/src, crates/*/tests, validation/, tools/):
| path | runs it | selects |
|---|---|---|
| crates/native/examples/demo/ | `mod demo;` in all three native examples | `native`, `rust-dylib` |
| crates/native/examples/native_static.rs, native_dynamic.rs | native_examples.rs (native-examples step) | `native` |
| crates/native/examples/native_rust_dylib.rs | rust_dylib.rs (rust-dylib step) | `rust-dylib` |
| crates/boundary/examples/secure_zero_codegen.rs | native_examples.rs step 6 | `native` |
| crates/boundary/examples/bench_*.rs, copy_sweep.rs | `cargo xtask bench` only (off the gate) | clippy only |

No crate has a `benches/` directory; no test target builds an example.

## Proof

- Unit tests: `direct_tier_maps_an_example_to_the_step_that_runs_it` (5 paths x Direct/Wide, plus products),
  `direct_tier_keeps_clippy_only_for_examples_no_gate_step_runs` (bench examples; an unplaceable file is FULL),
  `example_steps_follow_the_xtask_source_that_names_the_example` (fixture: by-path naming, a step without an arm is
  FULL, a module off the test steps selects nothing).
- Plants `handoff/XTASK-AFFECTED-EXAMPLES-plants.txt`: 8 of 8 CAUGHT (job xtask-affected-examples-plants-2 on mei).
- Replay `cargo xtask affected --list --files crates/native/examples/demo/mod.rs ...`:
  before (next, job xtask-affected-examples-before): demo/mod.rs and native_rust_dylib.rs FULL ("a product crate or
  its linker input"); secure_zero_codegen.rs and bench_mem.rs clippy only.
  After (job xtask-affected-examples-lint-2): demo/mod.rs -> `cargo xtask native` + `cargo xtask rust-dylib`;
  native_rust_dylib.rs -> `rust-dylib`; secure_zero_codegen.rs -> `native`; bench_mem.rs clippy only; no FULL.
- Gate (affected tier, `--base origin/next`) xtask-affected-examples-gate: 3 suites, 465 passed, 0 failed, peak 1.2G.
- `cargo xtask clippy`, `static`, `records check` clean (xtask-affected-examples-lint-2).
- End-to-end `cargo xtask affected --run --files crates/native/examples/demo/mod.rs`: job
  xtask-affected-examples-e2e: clippy niji-native ok, products 313 s ok, `cargo xtask native` 51 s ok,
  `cargo xtask rust-dylib` 67 s ok, `affected: ok` (about 7 min, where it used to be the FULL gate).

## Notes

- INFORMATIONAL: the brief says the native examples selected "clippy only" on next; on `ebb670553` they escalated to
  FULL (classify). Only boundary's `secure_zero_codegen` selected clippy alone. Either way the change narrows the
  gate to the steps that run the file and covers secure_zero_codegen.
- Limit: a step is found only when main.rs's test step calls the naming module directly (`|| <m>::`); a module
  reached only through another module would select nothing. No such case today.

Next step: none for this story (lands via CI).
