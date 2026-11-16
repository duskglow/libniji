# RENAME-NIJI-CODE: Liella -> libniji (code). State after RENAME-NIJI-CODE-2, 2026-11-16

Branch `task/rename-niji-code-2` (continues `task/rename-niji-code`), base `origin/next` e1afc4e47, code head
30766aacb. Role mei `~/w/s1`; proof records in mei `~/work/RENAME-NIJI-CODE-2/{before,after}` (compare with
`bash ~/work/RENAME-NIJI-CODE-2/compare.sh`). Brief: `~/state/briefs/RENAME-NIJI-CODE.md`; mapping and reasons:
liella-ops `state/RENAME-LIBNIJI-PLAN.md`. Full report: handoff/rename-niji-code-report.md.

## State
Done. The rename is the script's output plus the fixes listed below; every job named in the report ran on 30766aacb (proof records: see the report).

## Decisions in force
- Pending record `records/pending/RENAME-NIJI-CODE-1.toml` (D-NEW, series D-X): the rename, the mapping, no aliases;
  lr1's hash recorded in place; records keep their history. No differences row (no glibc-visible name changes).

## The script (tools/rename_niji.py) and how to reproduce
Copy the head's script into a checkout of e1afc4e47 and run it; the result equals the branch except the commits that
are not the script's output: 8edbe7557 (sizes of test strings and note owners), 36faffcf0 / 8e5551fea (the
`__NIJI__` test and record), ee0a5e923 (native-gen's rewrap of include/niji/native.h), 43846bf53 (the test
fix), 03db94b18 (lr1 hash), ee350824a (rust-dylib builds
rustc's `libniji_native.so` and links it as `libniji.so`, Q2). `--self-test` covers every rule; `--check` on the head reports 0 changes.
Rules added in this run: RUST_IDENT (a bare `Liella` as a Rust type, variant or `starts_with("Liella")` prefix
becomes `Niji`, .rs only); PROTECT the compat records' key list and the two test names D-27.43 cites; skip
stub files `cargo xtask stubs` still owns; source files (by suffix) are text even with a NUL byte.

## Things not to redo
- Records (`records/compat/*.toml` key `liella`, `liella.` tunable text, D-27.43's test names) stay as history;
  the reader (xtask records schema, tools/records_import.py) keeps the key `liella`. A records correction or
  RENAME-NIJI-DOCS may rename them together (reader + all records in one change).
- docs/ (incl. toolchain.md's lr1 hash and `LIELLA_RUST_ABI` lines), README, plan/, spec prose: RENAME-NIJI-DOCS.

## Open
- The landing must regenerate docs/codemap/symbols.tsv (train-owned) before its gate: 2 affected:: tests read
  it (RULING NEEDED in the report). Otherwise none for this story. Follow-ups: RENAME-NIJI-DOCS (prose, compat-flags wording, spec quotes that the s31
  stubs regenerate from), DL-TRACE-INTERP (the 7 swap-root ldd cases, unchanged by the rename).
