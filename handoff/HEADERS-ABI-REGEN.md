# HEADERS-ABI-REGEN: one command keeps headers, ABI layouts and layout tests consistent

Branch `task/headers-abi-regen` (from origin/next 627ef49ca). Kind: tooling. Role mei:~/w/s2.

## What changed

- `xtask/src/headers.rs`: `cargo xtask headers-gen` (write mode) runs `abi::run(root, false)` after writing the headers.
  `spec/abi/layouts.tsv` feeds both generators, so one command now leaves `libc-include/`, `crates/abi/src/layout*.rs`
  and `crates/abi/tests/appendix_c_layouts*.rs` consistent. abi-gen writes only files whose text changed.
  `headers-gen --check` stays headers-only (`abi-gen --check` is its own step of `cargo xtask test`).
  Chosen over "fail naming the command": coupling cannot be forgotten, and abi-gen is deterministic and cheap.
- `xtask/src/affected.rs`: `GEN_PRECHECKS` (rule 10 in the module doc). A changed `spec/abi/layouts.tsv` selects
  `abi-gen --check`, `headers-gen --check` and `stubs`, run first in process in every tier (not `--only`); a red one
  prints the fix (`cargo xtask headers-gen`, then `cargo xtask stubs`) and ends the run in seconds, before the
  escalated FULL run starts (layouts.tsv still escalates to FULL). `--list` prints a `prechecks (...)` line.
- `xtask/src/spec.rs`: `abi_stubs` is `pub(crate)` (the new headers test calls it).
- Tests (no spec clause): `headers::tests::headers_gen_regenerates_the_abi_layouts` (copies headers/, spec/abi,
  spec/unicode, libc-include and the four abi outputs to a temp dir, adds a struct row, runs headers-gen alone, then
  abi-gen --check and the layout stub check), `affected::tests::layouts_table_selects_the_generator_prechecks`,
  `affected::tests::generator_prechecks_run_the_checks`.
- Plants: `handoff/HEADERS-ABI-REGEN-plants.txt` (7 plants).

## Other tables checked for the same two-step shape

| table | generators / checks reading it | same failure? |
|---|---|---|
| `spec/abi/layouts.tsv` | headers-gen, abi-gen, stubs (abi_* ids), abi-types (check) | yes: fixed |
| `spec/abi/initializers.tsv` | abi-gen, stubs (init_* ids) | no: no generator but abi-gen writes from it, so its usual command is abi-gen |
| `spec/abi/layouts-aarch64.tsv`, `initializers-aarch64.tsv` | abi-gen only | no |
| `spec/abi/prototypes.tsv` | headers-gen, math-gen (math prototypes), callsites/visibility (checks) | shape yes, failure not seen: a probed complete table (3778 rows), header slices do not add rows |
| `spec/abi/versions.tsv` | headers-gen, compat-gen (binding), math-gen (versions) | shape yes, failure not seen: probed table |
| `spec/abi/compat-plan.tsv` | compat-gen; compat-plan, compatprobe (checks) | no (one generator) |
| `spec/abi/langinfo.tsv`, `native.tsv`, `csys/syscalls.tsv`, `s10_math.tsv` | one generator each | no |
| macros, typedefs, visibility, callsites, noexcept, stubs.tsv, headers/*.tsv | headers-gen plus checks only | no (checks run in `static`/gate, not generators) |

## Results

All on mei, code head 4d55b1c13:
- `headers-abi-regen-lint2`: `cargo xtask clippy`, `cargo xtask static`, the 3 new tests and `escalations`: rc 0.
- `headers-abi-regen-plants`: 7 plants, 7 CAUGHT, 0 WARN.
- `headers-abi-regen-demo` (`~/work/HEADERS-ABI-REGEN/demo.sh`, RAM copy): (A) struct row + `headers-gen` only:
  `stubs` rc 0, `abi-gen --check` rc 0; (B) stale tree: `affected --run --files spec/abi/layouts.tsv` printed
  `FAIL precheck abi-gen-check`, `FAIL precheck stubs` (`abi_struct_niji_regen_probe` has neither a stub nor a
  test) and stopped, 46 s with the xtask build; (C) coupling removed: `headers-gen` then `stubs` rc 1. rc 0.
- `headers-abi-regen-gate`: the direct tier escalates to FULL (xtask generator sources); verdict in the hand-back.
- Superseded: `headers-abi-regen-lint` (clippy doc_lazy_continuation on the rule 10 doc item, fixed),
  `headers-abi-regen-unit` (older head).

## Next step

None required. INFORMATIONAL: if a story ever edits `prototypes.tsv` or `versions.tsv` by hand, add those tables
to `GEN_PRECHECKS` (math-gen --check, compat-gen --check, headers-gen --check) the same way.
