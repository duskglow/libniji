# RENAME-NIJI-DOCS: libniji (and Yohane Linux) in the prose, plus the old-name ratchet. State 2026-11-16

Branch `task/rename-niji-docs`, base `origin/next` 28740dd5e. Role mei `~/w/s1`. Brief: `~/state/briefs/RENAME-NIJI-DOCS.md`.
Follows RENAME-NIJI-CODE (handoff/rename-niji-code.md; decision D-X.114).

## State
Done. Commits: prose rename (de95c7598), old-name check (e5eef411b), stub regen and plants file (abe342e8d), this note.

## What changed
- **Prose** (140 files: README, CLAUDE.md, CONTRIBUTING, SECURITY, AI_POLICY, CODE_OF_CONDUCT, docs/, plan/ outside
  the generated views, spec/ prose and the two `*.glibc.txt` probe records, glibc-functional-spec.md,
  implementation-plan.md). Mapping as in tools/rename_niji.py: `Liella` -> `libniji` (every time, owner Q1),
  `LIELLA` -> `NIJI`, `liella` -> `niji` in identifiers and paths, mangled lengths kept, `libliella_native` -> `libniji`.
  Kept: author identity, `liella-ops`, `liella-historic`, story ids (`LIELLA-*`, `AQOURS-*`), `handoff/` paths, quoted
  rulings. Script: scratch `prose.py` (not committed; a one-off, the ratchet now guards the result).
- **Yohane Linux**: "Aqours" -> "Yohane Linux" in current text (articles fixed). No `ID=`, `+aqoursN`, apt suite,
  Origin or Label appears in current text, so nothing else needed renaming; `/aqours/` kept.
- **Repository URL**: no current text names `github.com/duskglow/liella` (only rename_niji.py's PROTECT list), so the
  rule is applied by the prose script for future runs but changed nothing.
- **Release notes**: header `# libniji: release notes`, the sentence "libniji was called Liella until November 2026.",
  a history marker comment, then the `snapshot-2026-11-16` entry from liella-ops `release/snapshot-2026-11-16-notes.md`
  (headings demoted one level, "(now libniji)" on its heading), then the past entries unchanged except "(now Yohane
  Linux)" once in the 11-15 entry.
- **Snapshot findings**: getent and ldd (UTIL-LDD has landed: csys/util/ldd.c, libc-bin `/usr/bin/ldd`) added in
  README, docs/toolchain.md section 3, docs/install-packages.md libc-bin row; the libstdc++.so.6 / unique-symbol sentence
  in README (limitations bullet and the phase-14 row); lr1 hash in docs/toolchain.md updated to spec/abi/rust-abi.toml's
  `b4a17b0ef811ee2c` (3 places); stale paths: loadtime-entry -> `hot/mem.rs`, saturation 22 -> `hot/invtrig.rs`,
  24 -> `hot/` (`exp_log.rs`, `hyperbolic.rs`), 00 appendmerge sentence -> one file per section, 10/13 bench-refresh
  logs -> "not kept in the tree" (the directory is in no tracked path).
- **Ratchet**: `xtask/src/oldname.rs`, `cargo xtask old-name`, in `cargo xtask static` (39 checks) and in
  affected's XTASK_TOOLS. Allow-list documented in the module header.

## `git grep -i liella` by category (lines, at abe342e8d)
handoff/ 5572; results/ + validation/results/ 518; records/ 393; views generated from records/ (plan/07-decisions,
docs/compat-flags.md, docs/saturation/01-07) 348; release notes past entries + the one sentence 74;
tools/rename_niji.py 54; allowed tokens elsewhere 293 (`liella-historic` 255 in bench/perf-ledger.tsv and a few
docs, author identity in tools/deb, `liella-ops`, the two kept cache_write test names, the compat key in
schema.rs / records_import.py, story ids, one quoted ruling, README's sentence); 3 binary fuzz seeds (fuzz/seeds/ld_cache).

## Open / RULING NEEDED
- The generated views (plan/07-decisions/*.md etc.) keep the old name because their records do; the allow-list
  covers them by path. If the owner wants the views to say libniji, the records' prose needs a sweep (records are
  history per RENAME-NIJI-CODE). Recommend: leave as is.
- `results/` (dated measurement tables) is allowed beside `validation/results/` (the brief names only the latter);
  same reason: dated history.

## Things not to redo
The prose sweep and the stub regen (`cargo xtask stubs` changed only spec/clauses.tsv and s31_utilities_clauses.rs).

## Jobs (mei)
rename-niji-docs-stubs (stubs + old-name + unit tests, rc 0), rename-niji-docs-gate, rename-niji-docs-plants,
rename-niji-docs-lint. Results in handoff/reports/RENAME-NIJI-DOCS.json.

## Next step
Land. The owner renames the GitHub repos; nothing in the tree depends on it.
