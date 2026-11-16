# RENAME-NIJI-CODE: Liella -> libniji (code). State after RENAME-NIJI-CODE-3, 2026-11-16

Branch `task/rename-niji-code-3` (continues `task/rename-niji-code-2` and `task/rename-niji-code`), base `origin/next`
e1afc4e47. Role keke `~/w/r5`. Brief: `~/state/briefs/RENAME-NIJI-CODE.md` (spec) and its continuations -2, -3;
mapping and reasons: liella-ops `state/RENAME-LIBNIJI-PLAN.md`. Full report: handoff/rename-niji-code-report.md.

## State
Done. The rename is the script's output plus the hand fixes listed below. -3 fixed CI's red (train 756, native_static
"pwrite was short") and the same class of defect elsewhere: numbers that were a renamed literal's length or offset.

## Decisions in force
- Pending record `records/pending/RENAME-NIJI-CODE-1.toml` (D-NEW, series D-X; ids assigned in 3abfa7769 as D-X.114):
  the rename, the mapping, no aliases; lr1's hash recorded in place; records keep their history. No differences row.
- Ruling #6086: the train regenerates the codemap (and its other views) before its gate; this branch leaves
  docs/codemap/ alone. The -3 gate ran the train's three generators `--in-place` in the role only, then `cargo xtask test`.

## The script (tools/rename_niji.py) and how to reproduce
Copy the head's script into a checkout of e1afc4e47 and run it; the result equals the branch except the hand commits:
8edbe7557 (sizes of test strings and note owners), 36faffcf0 / 8e5551fea (`__NIJI__` test and record), ee0a5e923
(native-gen rewrap), 43846bf53 (test fix), 03db94b18 (lr1 hash), ee350824a (rust-dylib), 30766aacb (three fixtures),
and -3's 953c0081d / 609ba6252 (lengths: native example, its expected line, s22_sync's PID offset and check).

## Things not to redo
- The length sweep (report, "Length sweep"): every hunk of the rename diff where a literal holding the old name changed
  within 14 lines of an unchanged number, plus slices/offsets, fixed arrays and expected-output counts, was checked.
- Records (`records/compat/*.toml` key `liella`, `liella.` tunable text, D-27.43's test names) stay as history.
- docs/ (incl. toolchain.md's lr1 hash and `LIELLA_RUST_ABI` lines, docs/plants.md examples), README, plan/, spec
  prose: RENAME-NIJI-DOCS.

## Open
None for this story. Follow-ups: RENAME-NIJI-DOCS (prose, compat-flags wording, spec quotes the s31 stubs regenerate
from), DL-TRACE-INTERP (the 7 swap-root ldd cases, unchanged by the rename).
