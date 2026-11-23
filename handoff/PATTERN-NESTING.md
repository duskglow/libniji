# PATTERN-NESTING handoff (HL-13 fnmatch, HL-14 wordexp nesting)

State: branch `task/pattern-nesting-3` (PATTERN-NESTING-3, continues `task/pattern-nesting-2`), role
shiki:~/w/sh2 (Haswell-EP build host, E5-2697 v3; role dir on /mnt/zram). Final code head 2a288a620
(leg 4), plants-file fix 8cd8fa107; report commit on top. Plants file: `handoff/PATTERN-NESTING-3-plants.txt`
(21; `no-memo` re-aimed in leg 4 at `return if nested { self.nested(`). Scratch:
`~/w/.scratch/PATTERN-NESTING-3/` (cmp.py BASE AFTER: count diff; cond.py LISTING: one line per executed
instruction with its source function; counters-{next,head4,head5}.tsv; c-*.txt condensed listings,
sum-*.txt per-function sums). Host `~/work/pattern-nesting-3/`: ck3.sh <tag> (clippy, static, the 7
wordexp/fnmatch targets, counts), trace_any.sh <role|next> <tag> <spec|utf8> <case> <function>, fn_utf8.c.
Full report: `handoff/PATTERN-NESTING-report.md`; json `handoff/reports/PATTERN-NESTING-3.json`.

## Ruling in force

#8034 (coordinator): inline-then-heap, no per-call cost accepted, no nesting limit; target: wordexp rows and
fnmatch_names_ch back to origin/next's counts. #7900/#7901: land neutrally; the arithmetic limit is in HL-14.

## Done (do not redo)

HL-13 code and tests; HL-14 parser/expander; evaluator rewrite (7d27069b3); tracer fix (1d554b1ee); leg-2/3
per-word fixes (e67086c87, 3f50f55c0). Leg 4, 2a288a620: fnmatch's compiled path (UTF-8 names): `item` was an
out-of-line call per item (0x398-byte frame); now `#[inline(always)]`, group evaluation in `group`
(`#[inline(never)]`), `nested` `#[inline(never)]`; `unwrap_or` -> `unwrap_or_else` for the empty `Prog`;
`seq` pushes in place onto a local vector. Trace (one call, instructions): `naïve.c`/`*.[ch]` next 4977,
3f50f55c0 5167, 2a288a620 5120; `Grüße.txt`/`????.txt` next 5061, 3f50f55c0 5309, 2a288a620 5039.

## Open (RULING NEEDED in the hand-back)

Rows still above next (Haswell-EP niji_instr, job pattern-nesting-3-check5): wordexp plain_8 +1.13%, quoted
+1.33%, split_var +1.20%, tilde_var +1.13% (x2 benches); fnmatch_names ch +0.85%, class +0.94%, period
+1.55%, star_c +0.73%, paths_leading_dir +0.48%. Below next: wordexp arith -4.2%, param_forms -4.6%,
fnmatch lib_so -2.8%, paths_pathname -1.7%. Remaining fnmatch gap (+143 on the ch case, per-function sums
next vs head5): `program` setup 72, `new` +48, `current_memory` +44 (drops of the empty `seqs`/`todo`/memo
vectors), one more free (`free_hint` +32), `run_prog` 25.

## Next step (if the ruling is "keep tuning")

fnmatch: a group-free pattern compiles straight to `items` (no `Todo`, no `seqs`), and `run_prog` builds no
`Memo` when `prog.groups == 0`; then recount (ck3.sh) and replant. wordexp: Parser `frames`/`nest`
InlineVec::new default fill (+19 per call, leg 2 trace). Plan rows: `plan/hardcoded-limits.md` HL-13, HL-14.
No D-row needed (no spec deviation).
