# PATTERN-NESTING handoff (HL-13 fnmatch, HL-14 wordexp nesting)

State: branch `task/pattern-nesting-2` (PATTERN-NESTING-2, cut from `task/pattern-nesting`), role
keke:~/w/n1 (Zen 1 build host). Plants file: `handoff/PATTERN-NESTING-2-plants.txt` (12 plants:
6 HL-13, 6 HL-14). Scratch: `~/w/.scratch/PATTERN-NESTING-2/`; on the host `~/work/pattern-nesting-2/`.
Code head: 11b46b216 (leg 2).

## Done

- HL-13 (fnmatch, FNM_EXTMATCH nesting): 6e92bc806. Plants: `pattern-nesting-2-plants1` on
  1a1b2911b, 6/6 CAUGHT (superseded by plants2: the head moved).
- HL-14 (wordexp): parser frames + `nest`, expander job stack, evaluator `Wait` stack; 128 and
  512 limits removed (5b5f2904d WIP, 792e6f763 compiled and tested). New test assertions and the
  6 HL-14 plants: 15f98b743.
- Counts, rows `wordexp_*`/`fnmatch*` of cbench_opt and cbench_patterns
  (`cargo xtask bench --compare --counters cbench_opt cbench_patterns --rows
  "cbench_opt/wordexp_*,cbench_opt/fnmatch*,cbench_patterns/wordexp_*,cbench_patterns/fnmatch*"`):
  base job `pattern-nesting-2-counts-base` (on 1a1b2911b: HL-13 in, wordexp as on next),
  saved `~/work/pattern-nesting-2/counters-base.tsv`. After 792e6f763 (`-counts-after`,
  `counters-after.tsv`): fnmatch rows unchanged (-0.04..+0.03%), wordexp rows SLOWER in niji_instr:
  arith 4141->4799 (+15.9%), param_forms 6246->7066 (+13.1%), quoted +9.9%, tilde_var +8.8%,
  split_var +5.1%, plain_8 +2.7%. Compare script: `~/work/pattern-nesting-2/cmp.py BASE AFTER`.
- Profile (`pattern-nesting-2-trace-new`, wl_trace per-function split, new head only; the base
  cannot be traced outside a role dir): cost spread over the job stack set up for every word
  (push<Job,4>, new<Job,4>, segs), the frame loop for every `${x:-word}` (braced, nested,
  as_slice<Frame>, close, put_text), and the evaluator (run 448, push/pop<Wait>).
- Fix 11b46b216: `segs` fast path (direct walk; a trailing nested word expanded in place; job stack
  only in `segs_deep`), `braced` fast path (plain word up to `}`: one text segment, no frame),
  cheap `InlineVec::pop`. Tests green (t3 job). NOT YET COUNTED.

## Running at the end of leg 2 (started 13:12Z on keke)

- `pattern-nesting-2-counts-after2` (same bench command, on 11b46b216).
- `pattern-nesting-2-plants2` (all 12 plants, 2 copies, /dev/shm/claude/pattern-nesting-2/plants-2,
  on 11b46b216; about 75 min).

## Next steps, in order

1. `~/bin/jobwait pattern-nesting-2-counts-after2`; copy counters.tsv to
   `~/work/pattern-nesting-2/counters-after2.tsv`; `python3 cmp.py counters-base.tsv counters-after2.tsv`.
   Every row must be <= base. If wordexp_arith is still up: the evaluator (`P::run`): fewer
   pushes (merge `Wait::Test` into the level-0 `Binary`), smaller `Wait`. Any code change
   supersedes plants2 (rerun it). If time runs short, hand back with the numbers (brief: owner
   decision when a removal cannot meet a bound).
2. When counts are done (role free): gate `pattern-nesting-2-gate` =
   `cargo xtask affected --run` under ~/bin/gate 16G; clippy (`cargo clippy --workspace
   --all-targets --all-features -- -D warnings`) and `cargo xtask static` on the role.
3. jobwait plants2: all CAUGHT? (msg for we-nospace-panic is `Result::unwrap\(\)`.) Commit
   results.txt as the plants file if the tool says so.
4. precheck `--jobs <labels> --head <sha>`, report json `handoff/reports/PATTERN-NESTING-2.json`
   (impact: counts before/after), handback-check, hand back.

## Not to redo

HL-13 code and tests; HL-14 code; the profile. Plan rows: `plan/hardcoded-limits.md` HL-13, HL-14
(no status column to update). No D-row needed so far (no spec deviation; limits removed per the
owner's 11-17 ruling and the coordinator's #7901 decision).
INFORMATIONAL for the report: a pattern removal/assignment/arithmetic text with expansions now
allocates the nested word's pieces on the heap (`fresh`, one malloc per call that has one); no
bench row covers it.
