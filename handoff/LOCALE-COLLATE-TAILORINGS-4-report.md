# LOCALE-COLLATE-TAILORINGS-4 report (2026-11-24): the second CI merge conflict, and the move

Result: run -3's branch (the shared collation root, D-31.32) is merged with origin/next deead7890. The train had
stopped on `crates/locale/tests/reference.rs` (merge_next rc 3) again. This time next had appended
`code_point_order_compiles_like_the_reference_c_utf8` (with its `CODE_POINT_ORDER` source) at the end of the file,
where this branch's generated-root test also sat. Following the coordinator's note ("resolve, then move the new
cases into their own module"):

* The conflict takes next's side whole.
* This branch's `Root` file source and `generated_root_collation_is_the_references_en_us` move unchanged into
  `crates/locale/tests/reference/collate_root.rs`. A diff against the removed block shows only two changes: one
  intra-doc link now reads `super::diff_collate`, and the closing brace moved with the function.
* `reference.rs` declares the new file as a child module, once, after its `use` lines:
  `#[path = "reference/collate_root.rs"] mod collate_root;`. This is the same pattern as
  `crates/abi/tests/appendix_c`. A file in a subdirectory of tests/ is not a test target of its own, so the test
  still runs in the `reference` target, under the name `collate_root::generated_root_...`. The plants'
  `cargo test ... generated_root` filter and their `msg:` regex still match it.
* D-31.32's `tests` now names `crates/locale/tests/reference/collate_root.rs`.

Nothing else conflicted, and no library or generator code changed.

## Commits (branch task/locale-collate-tailorings-4)

* a997a0f44: the merge of origin/next, with the resolution and the move.
* The commit after it: this report, the handoff update and the report record.

## Checks (shiki, role sh2)

* Step 0, the inherited head 8c8b819a4: lct4-lint0 (`cargo xtask clippy` + `cargo xtask static`), rc 0.
* The merged head: lct4-lint (`cargo clippy --workspace --all-targets --all-features -- -D warnings` +
  `cargo xtask static` + `cargo xtask affected --list --base origin/next`), rc 0.
* Gate: `affected --list` escalates to FULL for the same reasons as runs -2 and -3 (Cargo.lock, Cargo.toml,
  crates/collate-root/Cargo.toml, crates/locale/Cargo.toml, spec/locale/collate-root.txt), so the FULL gate is
  the train's. Instead, lct4-tests ran `cargo xtask so` + `cargo test -p niji-collate-root -p niji-locale` and
  passed (rc 0). niji-locale unit tests: 5 passed. tests/prog.rs: 21 passed. tests/reference.rs: 20 passed, 0
  failed, in 114 s. That includes `collate_root::generated_root_collation_is_the_references_en_us ... ok` and
  next's new `code_point_order_compiles_like_the_reference_c_utf8`.
* Plants: lct4-plants ran run -2's 12 plants (`handoff/LOCALE-COLLATE-TAILORINGS-2.plants`) on the merged head.
  The job's rc was 0, with 11 CAUGHT (each by its `msg:` naming the moved test), 1 EQUIVALENT (after-level-1e9b,
  with the same reason as in run -2), 0 SURVIVED and no WARN lines. Results:
  handoff/LOCALE-COLLATE-TAILORINGS-4-plants.txt.
* Precheck: `cargo xtask precheck --jobs lct4-tests,lct4-plants --head <final head>`, run on the synced final
  head at hand-back.

## RULING NEEDED: none. INFORMATIONAL: run -2's items stand (handoff/LOCALE-COLLATE-TAILORINGS-2-report.md).

## Refusals: none. Leftovers: none running.

## Next

Unchanged from run -2: a new story (size L), stacked on LOCALE-PROBE-GEN-2 and LOCALE-SOURCES-GEN-2
(handoff/LOCALE-COLLATE-TAILORINGS.md, "Exact next step"). Its tests go in modules under
`crates/locale/tests/reference/`.
