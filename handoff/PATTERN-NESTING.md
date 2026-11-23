# PATTERN-NESTING handoff (HL-13 fnmatch, HL-14 wordexp nesting)

State: branch `task/pattern-nesting-3` (PATTERN-NESTING-3, continues `task/pattern-nesting-2`), role
shiki:~/w/sh2 (Haswell-EP build host, E5-2697 v3; role dir on /mnt/zram). Code head e67086c87 (leg 2).
Plants file: `handoff/PATTERN-NESTING-3-plants.txt` (18: the 12 of PATTERN-NESTING-2, `we-short-circuit` and
`we-eval-cap` re-aimed at the evaluator, 5 from leg 1, `we-dquote-handover` from leg 2; `we-first-guard`
re-aimed at the new `first == 0 ||` check). Scratch: `~/w/.scratch/PATTERN-NESTING-3/` (cmp.py, cmp_trace.py,
counters-*.tsv, traces head3.log/head4.log, cmp3.txt/cmp4.txt); on the host `~/work/pattern-nesting-3/`
(trace3.sh: per-function traces with the fixed tracer; trace4.sh: full instruction listing to
`list-<tag>-<case>.txt`; ck.sh: clippy, static, tests, counts).

## Ruling in force

#8034 (coordinator): inline-then-heap, no per-call cost accepted, no nesting limit. Target: wordexp rows and
fnmatch_names_ch back to origin/next's counts. #7900/#7901: land neutrally; the arithmetic limit is in HL-14.

## Done

- Leg 1 (7d27069b3): clippy fix; arithmetic evaluator as operator precedence on a 6-inline stack; fnmatch
  `Prog.root`, lazy `seqs`/parse queue. Test `wordexp_arith_lexer::precedence_conditionals_and_short_circuit`.
- Leg 2, 1d554b1ee: `validation/wl_trace.py` entry stack pointer from the caller frame. In a dynamic link
  gdb broke after the prologue, so the first epilogue pop before a tail jump ended the trace (the 32-instruction
  "early exit" of leg 1). Traces are whole calls now. NOTE for trace runs: wl_percall prints only the last 60
  lines unless `WLP_TAIL=0` (a `--list` listing needs it; trace4.sh sets it).
- Leg 2, e67086c87: `push_text`/`push_input` skip the length read when `first == 0`; boundary `patterns::wordexp`
  `#[inline]` (the extra call level was +30/call); double quotes in a word's own context read by
  `Parser::dquote_word` without a frame (an expansion opening a context gets the DQuote frame put under it, then
  `nested`). Job `pattern-nesting-3-check2` (on e67086c87): clippy clean, static tail clean, the 7 wordexp/fnmatch
  test targets green, counts:
  | row (cbench_opt, niji_instr) | next | 7d27069b3 | e67086c87 |
  | wordexp_arith | 4137.3 | +4.07% | 4269.3 +3.19% |
  | wordexp_quoted | 3782.0 | +6.76% | 3835.2 +1.41% |
  | wordexp_plain_8 | 3778.9 | +3.71% | 3850.2 +1.89% |
  | wordexp_tilde_var | 4100.9 | +2.26% | 4162.8 +1.51% |
  | wordexp_split_var | 3795.4 | +1.34% | 3825.0 +0.78% |
  | wordexp_param_forms | 6240.6 | -4.93% | 5911.6 -5.27% |
  | fnmatch_names_ch | 755.9 | +0.98% | 763.3 +0.98% (fnmatch unchanged; others +0.3..+2.1%) |
  cbench_patterns fnmatch rows +-0.02%.

## Running at the end of leg 2

- `pattern-nesting-3-plants2`: 18 plants on e67086c87, `/mnt/zram/claude/pattern-nesting-3/plants-2`, started
  ~16:45Z (~45-60 min). Verdicts: `grep -E ": CAUGHT|SURVIV|WARN-RC|WARN-NO|EQUIV" ~/logs/pattern-nesting-3-plants2.log`.
  It holds the role (sync_role rc 4 while it runs). If the code moves again, re-run on the final head (plants-3).
- SUPERSEDED: `pattern-nesting-3-plants` (leg 1, on 7d27069b3; stopped at 7 of 17 caught, all CAUGHT so far, when
  e67086c87 changed a planted line).

## Next steps, in order

1. Remaining wordexp cost (trace head4 vs next, `cmp4.txt`): plain_8 3672 -> 3748 instructions, all in the
   `word` + `push_input` + `push<Seg,12>` frames (next 338, head 409: ~9/word); quoted +59 (`seg` in the
   expander +69); tilde_var +67 (`seg` +105, `segs` -104, `word` +64); arith +137 (evaluator `run`
   +262, `arith_plain` +155). The frame names are inline attributions, so diff the instruction LISTINGS of
   one word: `trace4.sh ~/w/sh2 head we_plain_8` (role at head) and the same with the role synced to
   origin/next (as leg 1 did: a checkout of next in the scratch synced with sync_role). Suspects: register
   pressure or spills in `word` since `dollar`/`nested`/`dquote_word` are reachable from it (try
   `#[inline(never)]` on the rare arms: `dquote_word`, the `$` arm's `nested`); `InlineVec::new` filling
   `frames`/`nest` once per call (create them only when a word nests).
2. fnmatch plain rows (+0.3..2%): trace `fnmatch_names_ch` the same way (`Matcher::run`/`run_prog`/`Memo::new`).
3. Recount (ck.sh, new label), plants on the final head, direct-tier gate (`cargo xtask affected --run` under
   `~/bin/gate 16G`), foreground clippy + static, precheck, report `handoff/reports/PATTERN-NESTING-3.json`,
   handback-check. RUT RULE: if step 1 gains little, hand back with RULING NEEDED (accept the remaining
   ~1.5-3% on these rows for the unlimited nesting, or keep tuning).

## Not to redo

HL-13 code and tests; HL-14 parser/expander code; the evaluator rewrite (7d27069b3); the tracer fix and the
leg-2 per-word fixes. Plan rows: `plan/hardcoded-limits.md` HL-13, HL-14. No D-row needed (no spec deviation).
INFORMATIONAL (from PN-2): a pattern removal/assignment/arithmetic text with expansions allocates the nested
word's pieces on the heap (`fresh`, one malloc per call that has one); no bench row covers it.
