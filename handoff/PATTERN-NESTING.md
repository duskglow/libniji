# PATTERN-NESTING handoff (HL-13 fnmatch, HL-14 wordexp nesting)

State: branch `task/pattern-nesting`, cut from origin/next 0495c4684; this note and the report are
the only commits pushed. Run 1 (2026-11-23) stopped at the context checkpoint.

## Done (run 1)

- HL-13 (`fnmatch`, `FNM_EXTMATCH` nesting): implemented and tested on the build role, but **held
  off this branch by the coordinator's decision** (see the run's hand-back). Do not re-implement
  it. Ask the coordinator for the held change before touching `crates/core/src/pattern/fnmatch.rs`.
  On the role it was clippy and static clean, and the new tests plus the existing §14.7 tests passed.
- `s14_7_f_0` asserts the old cap (100 nested groups are an error). It changes with HL-13, not before.

## Not started: HL-14 (`wordexp`, `MAX_DEPTH` 128 at `crates/core/src/pattern/wordexp.rs:221`)

The recursion to replace with a heap stack (read by range: the file is 83 KB):

- Parser: `dollar` (enter/leave) -> `dollar_inner` -> `braced` -> `brace_word` (enter/leave), whose
  loop calls `dquote`, `dollar` and `backquote`; `dollar_inner` -> `arith` (`$((`, `$[`) whose loop
  reaches `dollar` again. `dquote` writes into its caller's segment list; `brace_word` and `arith`
  each build their own list and `stash` it into `arena` (a `Range`), so the result is already flat.
  Suggested shape: one loop with a stack of frames (BraceWord {header, depth, first, quoted,
  segs}, Arith {square, segs, ...}, DQuote {}) where a `$`/`"` pushes a frame instead of recursing,
  and a closing byte pops it and hands the stashed range to the parent.
- Expander: `segs` -> `param` -> `param_value` -> `segs` on the word's arena range
  (`${x:-${y:-...}}` expands inner words recursively). Needs the same treatment (or proof that it is
  bounded otherwise).
- Not in HL-14: the arithmetic evaluator's own nesting limit (`depth > 512` in `unary` and
  `primary`, lines ~1896 and ~2002), parentheses within one `$((...))`. It is not an audit row;
  raise it with the coordinator if it should go too.

Tests the brief asks for HL-14: 1000 nested `${x:-...}` (the reference accepts 300, audit probe),
a deep word on a 64 KiB thread stack (model: `crates/core/tests/regex_deep_nesting.rs`), and an
allocation-failure sweep (model: the `failinject` hook as in `crates/core/tests/enomem_fs_walk.rs`).
Update any test that asserts `WRDE_SYNTAX` for 129+ levels.

## Next step

Coordinator: rule on the held HL-13 change (hand-back). Then a run for HL-14 as above.
Plan rows: `plan/hardcoded-limits.md` HL-13, HL-14 (section 8: owner 2026-11-17 "remove").
