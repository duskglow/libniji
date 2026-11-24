# LOCALE-COLLATE-TAILORINGS-3 report (2026-11-23): the CI merge conflict

Result: run -2's branch (the shared collation root, D-31.32) merged with origin/next c4a2b6934. The train had
stopped on `crates/locale/tests/reference.rs` (merge_next rc 3). Next had appended the transliteration test
`strings_the_charmap_lacks_are_transliterated_as_the_reference_does` (with its `Under` file source) at the end of
the file, where -2 had appended `generated_root_collation_is_the_references_en_us` (with its `Root` file source).
The resolution keeps both, -2's first. Nothing else conflicted; no code changed beyond the merge.

## Commits (branch task/locale-collate-tailorings-3)

* The merge of origin/next (resolution above).
* This report, the handoff update and the report record.

## Checks (mei, role s2)

* Step 0, the inherited head 507435a94: lct3-lint0 (`cargo xtask clippy` + `cargo xtask static`), rc 0.
* The merged head: lct3-lint (`cargo clippy --workspace --all-targets --all-features -- -D warnings` +
  `cargo xtask static`), both passed. The job's rc 2 came from a trailing `cargo xtask affected` call that lacked
  `--list`, which is a usage error. It is superseded as a job; its clippy and static output was clean.
* Gate: `cargo xtask affected --list --base origin/next` escalates to FULL for the same reasons as run -2
  (Cargo.toml, crates/*/Cargo.toml, spec/locale/collate-root.txt), so the FULL gate is the train's. Instead,
  lct3-tests ran `cargo xtask so` + `cargo test -p niji-collate-root -p niji-locale`. That covers both tests in
  the resolved file. lct3-tests passed (rc 0): niji-locale unit 5, tests/prog.rs 18 and tests/reference.rs 18 passed, 0 failed. reference.rs took 104 s, which is in line with run -2's 113 s for the generated-root test.
* Plants: lct3-plants ran run -2's 12 plants (`handoff/LOCALE-COLLATE-TAILORINGS-2.plants`) on the merged head.
  Rc 0: 11 CAUGHT, 1 EQUIVALENT (after-level-1e9b, reason as in run -2), 0 SURVIVED, no WARN lines. Results: handoff/LOCALE-COLLATE-TAILORINGS-3-plants.txt.
* Precheck: `cargo xtask precheck --jobs lct3-tests,lct3-plants --head <final head>` on the synced final head,
  run at hand-back.

## RULING NEEDED: none. INFORMATIONAL: run -2's items stand (handoff/LOCALE-COLLATE-TAILORINGS-2-report.md).

## Refusals: none. Leftovers: none running.

## Next

Unchanged from run -2: a new story (size L), stacked on LOCALE-PROBE-GEN-2 and LOCALE-SOURCES-GEN-2
(handoff/LOCALE-COLLATE-TAILORINGS.md, "Exact next step").
