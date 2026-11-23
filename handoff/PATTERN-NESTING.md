# PATTERN-NESTING handoff (HL-13 fnmatch, HL-14 wordexp nesting)

State: branch `task/pattern-nesting-3` (PATTERN-NESTING-3, continues `task/pattern-nesting-2`), role
shiki:~/w/sh2 (Haswell-EP build host, E5-2697 v3; role dir on /mnt/zram). Code head 7d27069b3.
Plants file: `handoff/PATTERN-NESTING-3-plants.txt` (17: the 12 of PATTERN-NESTING-2, `we-short-circuit` and
`we-eval-cap` re-aimed at the new evaluator, plus 5 new). Scratch: `~/w/.scratch/PATTERN-NESTING-3/` (cmp.py,
cmp_trace.py, trace.sh, counters-*.tsv, traces); on the host `~/work/pattern-nesting-3/`.

## Ruling in force

#8034 (coordinator): inline-then-heap, no per-call cost accepted, no nesting limit. Target: wordexp rows and
fnmatch_names_ch back to origin/next's counts. #7900/#7901: land neutrally; the arithmetic limit is in HL-14.

## Done (run PATTERN-NESTING-3, leg 1)

- Step 0: clippy was red on the inherited head (`derivable_impls` on `Frame`'s Default): fixed (derive + `#[default]`).
- 7d27069b3: arithmetic evaluator rewritten as operator precedence (shunting-yard): one `Wait` per binary or
  unary operator, `(`, `?`, `:` (was: a Test plus ten Binary-level entries per operand); a local `Stack` with
  6 inline entries that spills to a `TryVec` (no limit). `Wait` is 16 bytes. New test
  `wordexp_arith_lexer::precedence_conditionals_and_short_circuit` (precedence, `?:` right-assoc, short-circuit,
  errors, 40-deep mixes past the inline part). fnmatch: `Prog.root` (the pattern's own sequence first, straight onto
  `items`); `seqs` and the parse queue only allocate once there is a group (2 allocations fewer per plain compile).
- Unit tests green on shiki (job `pattern-nesting-3-t1` on the pre-test tree; the new test passed in
  `pattern-nesting-3-trace-head`). clippy + static clean (`pattern-nesting-3-lint2`, before the test was added).
- Counts, shiki, niji_instr per call, `cargo xtask bench --compare --counters cbench_opt cbench_patterns --rows
  "wordexp_*,fnmatch_*"` (origin/next 0495c4684 = job `pattern-nesting-3-counts-next`; head 7d27069b3 =
  `pattern-nesting-3-counts-head`; compare `python3 cmp.py counters-next.tsv counters-head.tsv` in the scratch):
  | row (cbench_opt) | next | head | delta | (PN-2 after2 on Zen 1, vs its base) |
  | wordexp_arith | 4137.3 | 4305.5 | +4.07% | +10.0% |
  | wordexp_quoted | 3782.0 | 4037.8 | +6.76% | +6.9% |
  | wordexp_plain_8 | 3778.9 | 3919.2 | +3.71% | +3.7% |
  | wordexp_tilde_var | 4100.9 | 4193.5 | +2.26% | +2.3% |
  | wordexp_split_var | 3795.4 | 3846.1 | +1.34% | +1.3% |
  | wordexp_param_forms | 6240.6 | 5933.1 | -4.93% | -4.9% |
  | fnmatch_names_ch | 755.9 | 763.3 | +0.98% | (784 vs 761 limit on Zen 1) |
  Other fnmatch cbench_opt rows +0.35..+1.99%; cbench_patterns fnmatch rows +-0.03%.
  The Zen 1 limit 761 is a Zen 1 number: shiki's next is 755.9, so recount on Zen 1 or compare deltas only.

## Running at the end of leg 1

- `pattern-nesting-3-plants` (17 plants on 7d27069b3, `/mnt/zram/claude/pattern-nesting-3/plants-1`), started ~16:05Z.
  Verdicts: `grep -E ": CAUGHT|SURVIV|WARN-RC|WARN-NO|EQUIV" ~/logs/pattern-nesting-3-plants.log` on shiki.
  If later code changes touch a planted line, re-run plants on the final head (new dir plants-2).

## Next steps, in order

1. Find the remaining per-word cost (plain_8, quoted, tilde_var, split_var: the same deltas as PN-2, so it is the
   parser/expander, not the evaluator; and the ~7 instructions left on plain fnmatch rows). The head trace
   (`pattern-nesting-3-trace-head2`, `~/work/pattern-nesting-3/trace.sh ~/w/sh2 head` under jobrun) shows only
   27-32 instructions in `wordexp`, so it traced an early exit or a different build than the next trace (3672..4264):
   find out why first (wl_trace on a role synced clean at a commit; the role had just run `bench --compare`).
   Then `python3 cmp_trace.py pattern-nesting-3-trace-next.log <head trace>` in the scratch.
   Suspects from the diff: `InlineVec::new` filling all slots with `T::default()` for `frames` (2 x Frame),
   `nest` (4 x Seg), `jobs` (4 x Job) per call; `nest_first()` on every `put_*`; `push<Seg,12>` out of line.
   Apply the #8034 design to them: keep the per-word path as cheap as next (no new stacks initialised for a
   word with no nesting; create them lazily or reuse one buffer).
2. fnmatch plain rows: compare `Matcher::run` / `run_prog` / `Memo::new` against next in a trace of
   fnmatch_names_ch (the extra parameters and the `nested` branch).
3. Recount (same command, job `pattern-nesting-3-counts-head2`), plants on the final head if code moved,
   direct-tier gate (`cargo xtask affected --run` under `~/bin/gate 16G`), clippy + static, precheck, report
   `handoff/reports/PATTERN-NESTING-3.json`, handback-check.

## Not to redo

HL-13 code and tests; HL-14 parser/expander code; the evaluator rewrite (7d27069b3). Plan rows:
`plan/hardcoded-limits.md` HL-13, HL-14. No D-row needed (no spec deviation).
INFORMATIONAL (from PN-2): a pattern removal/assignment/arithmetic text with expansions allocates the nested
word's pieces on the heap (`fresh`, one malloc per call that has one); no bench row covers it.
