# PATTERN-NESTING-3 report (HL-13 fnmatch, HL-14 wordexp: no nesting limit)

Branch `task/pattern-nesting-3`, base origin/next 0495c4684. Counts: niji_instr per call, Haswell-EP build
host (`cargo xtask bench --compare --counters cbench_opt cbench_patterns`); traces: `validation/wl_trace.py`
listings of one call.

## Commits on this story (all legs)

- 7d27069b3 wordexp arithmetic by operator precedence on a small stack (6 inline entries, heap past it).
- 1d554b1ee wl_trace entry stack pointer from the caller frame (tooling fix).
- e67086c87 wordexp's usual double quotes read without a frame.
- 3f50f55c0 wordexp plain-word loop and segment expansion kept lean (inlining boundaries, one arena range for
  plain arithmetic text, evaluator `Stack` with one total length).
- 2a288a620 fnmatch's per-item step kept inline in the compiled matcher (group evaluation out of line,
  `unwrap_or_else`, `seq` pushes in place onto a local vector).
- 8cd8fa107 plant `no-memo` re-aimed at the new group dispatch.

## Results on the final head

- check5 (8cd8fa107): clippy rc 0, static clean, wordexp_deep_nesting, wordexp_arith_lexer,
  wordexp_budget_time, s14_search_clauses, fnmatch_deep_nesting, fnmatch_differential, patterns_products green.
- Gate `pattern-nesting-3-gate` (direct tier) and plants `pattern-nesting-3-plants4` (21 plants): see the
  hand-back and the json (`gate`, `jobs`).

| row | next | 3f50f55c0 | 2a288a620 | vs next |
|---|---|---|---|---|
| cbench_opt wordexp_arith | 4137.3 | 3962.8 | 3963.0 | -4.21% |
| wordexp_param_forms | 6240.6 | 5953.0 | 5953.2 | -4.61% |
| wordexp_plain_8 | 3778.9 | 3821.4 | 3821.7 | +1.13% |
| wordexp_quoted | 3782.0 | 3832.1 | 3832.4 | +1.33% |
| wordexp_split_var | 3795.4 | 3841.1 | 3840.8 | +1.20% |
| wordexp_tilde_var | 4100.9 | 4147.3 | 4147.2 | +1.13% |
| fnmatch_names_casefold | 1428.5 | 1439.0 | 1428.0 | -0.04% |
| fnmatch_names_ch | 755.9 | 763.3 | 762.3 | +0.85% |
| fnmatch_names_class | 892.3 | 897.0 | 900.7 | +0.94% |
| fnmatch_names_lib_so | 433.1 | 434.5 | 420.9 | -2.82% |
| fnmatch_names_period | 644.5 | 650.6 | 654.5 | +1.55% |
| fnmatch_names_question | 518.5 | 528.8 | 517.6 | -0.17% |
| fnmatch_names_star_c | 564.1 | 572.6 | 568.2 | +0.73% |
| fnmatch_paths_leading_dir | 438.6 | | 440.7 | +0.48% |
| fnmatch_paths_pathname | 684.0 | | 672.1 | -1.74% |
| cbench_patterns fnmatch_* (ASCII byte path) | | | | -0.03..+0.00% |
| cbench_patterns wordexp_arith | 4168.4 | | 3993.7 | -4.19% |
| cbench_patterns wordexp_tilde_var | 4087.0 | | 4132.8 | +1.12% |

Leg 4 against 3f50f55c0 is mixed: lib_so -3.1%, question -2.1%, casefold -0.8%, star_c -0.8%, ch -0.1%;
class +0.4%, period +0.6% (sum over the fnmatch_names rows -33.6 instructions). Kept: it removes a per-item
call from every compiled-path match and the one-call trace of `Grüße.txt`/`????.txt` is now below next
(5039 vs 5061).

## Where the remaining fnmatch cost is

One call, `naïve.c` / `*.[ch]`: next 4977, head 5120 (+143). Per-function sums, next vs head:
`program` +72 (new: the queue-based compile's setup), `new` +48, `current_memory` +44 (drops of the empty
`seqs`, `todo` and memo vectors), `free_hint` +32 (one more free), `run_prog` +25; the item loop itself is
now at next's level. Fix direction: a group-free pattern compiles straight to `items` with no queue and no
`seqs`, and `run_prog` makes no `Memo` when the pattern has no groups.

## RULING NEEDED

Ruling #8034 asked for wordexp rows and fnmatch_names_ch back to base. Four tuning passes later (legs 1-4)
the plain wordexp rows stay +1.1..1.3% (~45 instr/call) and five fnmatch rows +0.5..1.6%, against -4.2%
(arith) and -4.6% (param_forms), and fnmatch lib_so -2.8%, paths_pathname -1.7%. Recommendation: land as it
stands (no nesting limit, net gain over the wordexp rows), with the fnmatch group-free fast compile above as
a follow-up story; or rule a fifth pass (estimate: fnmatch rows to base is one small change plus plants;
wordexp's last ~1.2% has no single identified cause, RUT RULE).

## INFORMATIONAL

A pattern removal/assignment/arithmetic text with expansions allocates the nested word's pieces on the
heap (`fresh`, one malloc per call that has one); no bench row covers it.
