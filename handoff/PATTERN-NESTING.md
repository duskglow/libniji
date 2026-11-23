# PATTERN-NESTING handoff (HL-13 fnmatch, HL-14 wordexp nesting)

State: branch `task/pattern-nesting-3` (PATTERN-NESTING-3, continues `task/pattern-nesting-2`), role
shiki:~/w/sh2 (Haswell-EP build host, E5-2697 v3; role dir on /mnt/zram). Code head 3f50f55c0 (leg 3).
Plants file: `handoff/PATTERN-NESTING-3-plants.txt` (21: the 18 of leg 2, `we-stack-top` re-aimed at the new
`Stack::top`, plus `we-stack-pop`, `we-arith-range`, `we-square-end` from leg 3). Scratch:
`~/w/.scratch/PATTERN-NESTING-3/` (cmp.py BASE AFTER: count diff; cond.py LISTING: one line per executed
instruction with its source function; counters-next.tsv / counters-head4.tsv; c-list-*.txt condensed listings).
On the host `~/work/pattern-nesting-3/`: ck3.sh <tag> (clippy, static, the 7 wordexp/fnmatch test targets,
counts to counters-<tag>.tsv); trace_any.sh <role|next> <tag> <spec|utf8> <case> <function> (full listing to
`list-<tag>-<case>.txt`; `next` = `nextsrc/`, a copy of origin/next's cached source with a .synced naming
0495c4684, so wl_percall uses next's cached products and never mislabels the role); fn_utf8.c (scratch spec:
fnmatch on UTF-8 names in en_US.UTF-8).

## Ruling in force

#8034 (coordinator): inline-then-heap, no per-call cost accepted, no nesting limit. Target: wordexp rows and
fnmatch_names_ch back to origin/next's counts. #7900/#7901: land neutrally; the arithmetic limit is in HL-14.

## Done

- Legs 1-2 (7d27069b3, 1d554b1ee, e67086c87): see the commits; evaluator on a 6-inline stack, tracer fix,
  per-word fixes.
- Leg 3, 3f50f55c0 (from listings of one call, head vs next, `c-list-*.txt`): `nested` and `dquote_word`
  `#[inline(never)]` (inlined into `word` they cost the plain loop spills/reloads); `seg` `#[inline(always)]`,
  `segs_deep` `#[inline(never)]` (seg had become a call per segment); `arith_plain` pushes one input range to
  the arena (was a default-filled 12-entry Segs + a byte copy; `stash` removed); evaluator `Stack` keeps one
  total `len` (top/pop never read the heap vector below INLINE). Job `pattern-nesting-3-check4` (on 3f50f55c0):
  clippy rc 0, static tail clean, 7 test targets green, counts (niji_instr, Haswell-EP):
  | row | next | 3f50f55c0 |
  | cbench_opt wordexp_arith | 4137.3 | 3962.8 -4.22% |
  | wordexp_param_forms | 6240.6 | 5953.0 -4.61% |
  | wordexp_plain_8 | 3778.9 | 3821.4 +1.12% |
  | wordexp_quoted | 3782.0 | 3832.1 +1.32% |
  | wordexp_split_var | 3795.4 | 3841.1 +1.20% |
  | wordexp_tilde_var | 4100.9 | 4147.3 +1.13% |
  | fnmatch_names_* (cbench_opt) | | +0.32..+2.07% (names_ch 755.9 -> 763.3) |
  | cbench_patterns fnmatch_* | | +0.00..+0.02% |
- Finding: cbench_patterns' fnmatch rows (ASCII, byte path `direct`) are unchanged; the cbench_opt fnmatch
  rows rise because 4 of the ~100 names are UTF-8 and take the compiled matcher (`Pattern::new` + `matches`),
  which this branch changed (Prog/Memo/run_prog): ~+185 instructions per such call.

## Running at the end of leg 3 (started ~17:23Z)

- `pattern-nesting-3-fntrace`: listings of fnmatch on `na\xc3\xafve.c` / `*.[ch]` and `Gr\xc3\xbc\xc3\x9fe.txt` /
  `????.txt`, next and head: `~/work/pattern-nesting-3/list-fnnext-fn_utf8_{ch,q}.txt`,
  `list-fnhead-fn_utf8_{ch,q}.txt` (head builds 3f50f55c0's products first, ~7 min).
- `pattern-nesting-3-plants3`: 21 plants on 3f50f55c0, `/mnt/zram/claude/pattern-nesting-3/plants-3`
  (~60 min). Verdicts: `grep -E ": CAUGHT|SURVIV|WARN-RC|WARN-NO|EQUIV" ~/logs/pattern-nesting-3-plants3.log`.
  It holds the role (sync_role rc 4 while it runs).
- SUPERSEDED: `pattern-nesting-3-plants2` (on e67086c87; stopped at 6/18, all CAUGHT, when 3f50f55c0 changed
  planted lines), `pattern-nesting-3-check3` (clippy red: unused `stash`, fixed in 3f50f55c0).

## Next steps, in order

1. fnmatch compiled path: scp the 4 listings, `python3 cond.py <listing>`, per-function sums
   (`awk '{print $3}' | sed 's/@.*//' | sort | uniq -c | sort -rn`), find the ~185 extra (suspects:
   `Memo::new` per match, `Prog` root/seqs setup in `Pattern::new`, `run` taking prog/memo/nested). If a fix
   is small: kill plants3 (`~/bin/kill-label pattern-nesting-3-plants3 --kill`), commit, sync, recount with
   `ck3.sh head5` under jobrun (label pattern-nesting-3-check5), plants on the final head (plants-4).
   If none: keep 3f50f55c0 and plants3.
2. wordexp plain/quoted/tilde/split still +1.1..1.3% (~45 instr/call; ~5/word plus setup: the Parser's
   `frames`/`nest` InlineVec::new default fill, +19 per call in leg 2's trace). RUT RULE applies: three passes
   done; hand back with RULING NEEDED (accept ~1.2% on these 4 rows for unlimited nesting, with arith -4.2% and
   param_forms -4.6%, or keep tuning) unless step 1 leaves time.
3. Direct-tier gate (`cargo xtask affected --run` under `~/bin/gate 16G`, label pattern-nesting-3-gate) after
   plants, foreground clippy + static on the role, precheck --jobs, report
   `handoff/reports/PATTERN-NESTING-3.json`, handback-check. Wall-time deadline 19:20Z: if ~45 min remain with
   only jobs left, push and hand back PENDING JOBS.

## Not to redo

HL-13 code and tests; HL-14 parser/expander code; the evaluator rewrite (7d27069b3); the tracer fix; the leg-2
and leg-3 per-word fixes. Plan rows: `plan/hardcoded-limits.md` HL-13, HL-14. No D-row needed (no spec deviation).
INFORMATIONAL (from PN-2): a pattern removal/assignment/arithmetic text with expansions allocates the nested
word's pieces on the heap (`fresh`, one malloc per call that has one); no bench row covers it.
