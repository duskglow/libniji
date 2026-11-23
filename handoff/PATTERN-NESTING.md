# PATTERN-NESTING handoff (HL-13 fnmatch, HL-14 wordexp nesting)

State: branch `task/pattern-nesting-2` (PATTERN-NESTING-2, cut from `task/pattern-nesting`), role
keke:~/w/n1. Plants file: `handoff/PATTERN-NESTING-2-plants.txt`. Scratch: `~/w/.scratch/PATTERN-NESTING-2/`.

## Done

- HL-13 (fnmatch, FNM_EXTMATCH nesting): committed (6e92bc806) from the held patch, by the
  coordinator's decision on #7900. Plants for it: the six lines of the plants file (plant 4 now
  uses `perl:` with `\|`: the old `lit:` with ` | ` split the field and refused the run, which was
  why pattern-nesting-plants1 ended rc=3). Job `pattern-nesting-2-plants1` ran them on 1a1b2911b.
- HL-14 (wordexp) code written, **WIP commit, not yet compiled** (the role was busy with the
  baseline bench; nothing was synced after 1a1b2911b):
  - Parser: `MAX_DEPTH` gone. `Frame` stack (`Parser::frames`, InlineVec 2) for `"..."`,
    `${name<op>word}` and slow-path `$((...))`/`$[...]`; nested segments on `Parser::nest`
    (InlineVec 4), moved to `arena` by `close`. `push_text`/`push_input` take `first` (never merge
    into an enclosing context's segment). `word` keeps its direct calls; `put*` pick the list.
  - Expander: `segs` is a driver over `Job`s (InlineVec 4); nested words expanded into `fresh`
    pieces for Assign/Error/Trim/Arith, finished in `finish`; trim values kept on `vals`;
    `param`/`param_value`/`seg` return `Option<Nested>`; trimming moved to `fn trim`
    (body indentation is off: run rustfmt on the file).
  - Arithmetic evaluator: the 512 limit removed (coordinator's decision on #7901): `P::run` with a
    `Wait` stack (InlineVec 8), `Read`, and `combine` (indentation off too).
  - `InlineVec::truncate` and `pop` added in `crates/core/src/mem.rs` (additive).
  - Tests: `crates/core/tests/wordexp_deep_nesting.rs` (129/600/1000 levels of every kind, 64 KiB
    stack, allocation-failure sweep expecting `WRDE_NOSPACE`).

## Next steps, in order

1. `~/bin/jobwait pattern-nesting-2-counts-base pattern-nesting-2-plants1` on keke; keep the
   baseline numbers (bench `cbench_opt`/`cbench_patterns`, rows `wordexp_*,fnmatch*`, counters).
2. Sync, rustfmt `crates/core/src/pattern/wordexp.rs` (copy the result back to the worktree),
   `cargo xtask clippy`, fix compile errors; run `cargo test -p niji --test wordexp_deep_nesting
   --test wordexp_arith_lexer --test wordexp_budget_time --test s14_search_clauses` and the
   conformance/oracle wordexp tests that `cargo xtask affected --list` names.
3. Counts after (`pattern-nesting-2-counts-after`, same command); every wordexp/fnmatch row must
   not get slower; if one does, profile before tuning (InlineVec sizes, frame copies).
4. Add HL-14 plants (restore a cap alone: parser depth, expander, evaluator; `first` guard;
   short-circuit `eval` in `run`), run plants on the final head, gate (`cargo xtask affected
   --run`), precheck, D-row if needed (reserve an id via ops), report json, handback-check.

## Not to redo

HL-13 code and its tests; the recursion map of wordexp (above). `s14_7_f_0` already updated.
Plan rows: `plan/hardcoded-limits.md` HL-13, HL-14.
