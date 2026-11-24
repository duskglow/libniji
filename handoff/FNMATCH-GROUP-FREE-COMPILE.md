# FNMATCH-GROUP-FREE-COMPILE handoff (follow-up of PATTERN-NESTING-3, ruling #8175)

State: branch `task/fnmatch-group-free-compile`, cut from `task/pattern-nesting-3` (ea69dbf20, not yet on next);
role keke:~/w/n2 (Zen 1 build host). Code commit b40e94bba; 2a9dc93fb (optional `Units.offs`) measured worse and
was undone by d65cf7638 (source identical to b40e94bba). Plants: `handoff/FNMATCH-GROUP-FREE-COMPILE-plants.txt`
(2, unit test) and `handoff/FNMATCH-GROUP-FREE-COMPILE-cost-plants.txt` (the brief's cost plant, bench limit).
Scratch: `~/w/.scratch/FNMATCH-GROUP-FREE-COMPILE/` (cmp.py BASE AFTER; sums.py BASE_LIST AFTER_LIST: per-function
split of one traced call; counters-*.tsv). Host `~/work/fnmatch-group-free-compile/`: ck.sh <tag> [tests] (counts),
tr.sh <base|role> <tag> (one-call listings; base = primed 0495c4684 products), it.sh <tag> (clippy, tests, both).

## Done

- `Prog::seqs` is `Option` (None until a group is queued); `program` parses a pattern without `FNM_EXTMATCH`
  straight onto its items (no `Todo`); `run_prog` steps a group-free pattern's items in a plain loop (no `Memo`);
  the plain step is `Matcher::step`, shared by `item`. Groups keep PATTERN-NESTING-3's path, no nesting limit.
- Unit test `a_group_free_pattern_matches_as_the_group_machinery_does` (~1M pattern/string/flag cases, C and
  C.UTF-8): same reached positions as the memo path, no group structures without `FNM_EXTMATCH`.

## Counts (niji_instr per call; base = next before PATTERN-NESTING-3, 0495c4684)

Base numbers are PATTERN-NESTING-3's Haswell-EP counts; on keke the PN3 head counts within +-0.1% of them and the
one-call traces of the base equal theirs exactly (4977, 5061), so they transfer.

| row | base | PN3 head | this branch | vs base |
|---|---|---|---|---|
| fnmatch_names_ch | 755.9 | 763.0 | 760.3 | +0.58% |
| fnmatch_names_class | 892.3 | 900.6 | 898.3 | +0.67% |
| fnmatch_names_period | 644.5 | 655.0 | 652.4 | +1.23% |
| fnmatch_names_star_c | 564.1 | 568.6 | 565.7 | +0.28% |
| fnmatch_paths_leading_dir | 438.6 | 441.0 | 437.9 | -0.16% |
| fnmatch_names_lib_so | 433.1 | 421.2 | 417.5 | -3.60% |
| fnmatch_paths_pathname | 684.0 | 672.7 | 668.3 | -2.30% |
| fnmatch_names_question / casefold | 518.5 / 1428.5 | 518.0 / 1428.8 | 514.8 / 1427.2 | -0.71% / -0.09% |
| wordexp plain_8 / quoted / split_var / tilde_var | 3778.9 / 3782.0 / 3795.4 / 4100.9 | unchanged | 3824.3 / 3833.8 / 3842.7 / 4149.1 | +1.20 / +1.37 / +1.25 / +1.18% |
| wordexp arith / param_forms | 4137.3 / 6240.6 | unchanged | 3965.3 / 5956.3 | -4.16% / -4.56% |
| cbench_patterns fnmatch_* (ASCII byte path) | | | | +0.03..+0.06% (unchanged by this branch) |

The rows average over many names; only non-ASCII names reach the compiled matcher. One call of it (trace):
`naïve.c`/`*.[ch]` base 4977, PN3 5120, this branch 5055; `Grüße.txt`/`????.txt` base 5061, PN3 5039, now 4962.

## What bounds the rest (per-function split, `naïve.c`, base vs this branch, +71)

Compile is now below base (seq/program/push/write -126). Above base: the item loop's code (`step`/`run_prog` vs
the base's `item`/`run`: +34, in the `ones` iterator `next<u64>` +49, `get<u32>` +26, `period_at` +19), four more
empty-vector drops (`current_memory` +40), drop glue of `Prog`/`Compiled`/`Option<Prog>` (+37), `new` +41. None is
group machinery. wordexp plain_8 (+42 per call) is wordexp's own parser, not pattern code: `push_input<12>` with
its `get_mut` +56, `eq<u8>` +24, InlineVec default fill +18, against the arithmetic helpers -74.

## Plants

plain-loop-skips-sets, compile-queue-always: CAUGHT by the unit test. group-path-for-sets (cost plants file):
CAUGHT by INSTR-REGRESS, fnmatch_names_ch 762.5 against the Zen 1 limit 761 (this branch 760.3): a thin margin,
because the row averages over names that mostly take the direct (ASCII) path.

## Next step

fnmatch: the item loop's per-position code (`Bits::ones`, `period_at` per position for `*`) and the drop glue of
the compiled pattern (one owner for both item lists). wordexp: `push_input<12>` not inlined (generic over N) and
`Parser` InlineVec default fill. Both are RUT RULE territory after PATTERN-NESTING-3's four passes: a rethink, not
a fifth tweak. Plan rows: `plan/hardcoded-limits.md` HL-13, HL-14. No D-row (no spec deviation).
