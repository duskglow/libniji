# MATCHER-WORDEXP-RETHINK handoff (RETHINK of fnmatch's item loop and wordexp's buffering; #8237, #8238)

State: branch `task/matcher-wordexp-rethink`, stacked on `task/fnmatch-group-free-compile` (aa4d907d9, not yet
on next; next's merge base ea69dbf20 = PATTERN-NESTING-3). Role mei:~/w/s2 (Skylake-SP build host). Code head
cdeb7ccca (commits b76ac55a2 percall spec, 95c8715df bench row, 630dbda14 fnmatch, cf4ca8ad3 wordexp, cdeb7ccca
test fix + plants); records, limits and this note on top. Decision: **D-14.25** (adopt). Plants:
`handoff/MATCHER-WORDEXP-RETHINK-plants.txt` (5). Scratch: `~/w/.scratch/MATCHER-WORDEXP-RETHINK/` (syms.py and
phase.py split a `--list` trace by symbol and by instruction range; pc.sh, ck2.sh, tr.sh, trl.sh as run on mei
under `~/work/matcher-wordexp-rethink/`).

## Tried before (not repeated)

PATTERN-NESTING-3 legs 1-4: arithmetic evaluator on a small stack; double quotes without a frame; plain-word loop
inlining boundaries, one arena range for plain arithmetic text, evaluator `Stack` with one total length; per-item
`step` inline, group evaluation out of line, `unwrap_or_else`, `seq` pushes in place. FNMATCH-GROUP-FREE-COMPILE:
group-free patterns compile straight to items, `run_prog` without `Memo`; optional `Units.offs` (measured worse,
undone). PERF-FNMATCH-BRACKET: plain comparisons in `bracket_has`. All five tuned the compiled matcher or the
parser's inner loop; none removed a stage.

## Profile (one call, Skylake-SP, C.UTF-8; wl_percall --trace, --list)

fnmatch `*.[ch]` / `naïve.c` 5055: allocator ~1000 (pattern copy, two decodes, two programs, the string's
characters), decoding ~380, compiling ~560, bit-set item loop ~600. Only non-ASCII names took it (the direct
matcher took ASCII strings only): 4 of the 97 bench names, ~27% of each fnmatch_names row. wordexp plain_8 3720:
setup ~200 (inline-buffer fills: the three nesting stacks, Segs, Collected), parse ~1500, per word ~100 expand +
66 collect, result block ~560 (calloc ~95, then one memmove call per word).

## Change (D-14.25)

fnmatch: approaches A (direct last-star matcher reads valid UTF-8 a character at a time), B (bit-parallel item loop,
compile once; bound ~2000, allocations stay), C (shape classifier; covers few shapes). A adopted. wordexp: the
stacks `frames`/`nest`/`arena` are `LazyInlineVec` (mem.rs; storage made on first push), one copy of the
collected strings into the result block.

## Results (instructions per call)

Per call, validation/percall/fnmatch_utf8.c, C.UTF-8, mei (base = pre-PN3 0495c4684, next = 80d1f809b):

| case | glibc | base | next | before (aa4d907) | after |
|---|---|---|---|---|---|
| fnmatch_names_utf8 (new row's calls) | 1476.0 | 4971.0 | 5022.8 | 4955.6 | 886.4 |
| fnmatch_utf8_ch / class / period | 1236 / 1507 / 950 | 5048 / 5638 / 3534 | 5191 / 5905 / 3772 | 5126 / 5842 / 3710 | 788 / 1332 / 793 |
| wordexp_plain_8 / quoted | 6197 / 2823 | 3779 / 4040 | 3821 / 4090 | 3821 / 4090 | 3427 / 3936 |
| wordexp_split_var / tilde_var | 4029 / 3174 | 4245 / 4407 | 4290 / 4453 | 4290 / 4453 | 4075 / 4296 |

Bench rows (`cargo xtask bench --compare --counters`, mei; base = PN3's Haswell-EP counts, which transfer
within 0.1%): fnmatch_names star_c 564.1 -> 374.5, ch 755.9 -> 574.6, lib_so 433.1 -> 234.6, period 644.5 ->
483.5, question 518.5 -> 300.5, class 892.3 -> 723.2, casefold 1428.5 -> 1222.1, paths_pathname 684.0 -> 418.6,
paths_leading_dir 438.6 -> 259.4, fnmatch_names_utf8 (new) 962.6 (glibc 1474.0); wordexp plain_8 3778.9 ->
3426.0, quoted 3782.0 -> 3676.7, split_var 3795.4 -> 3624.4, tilde_var 4100.9 -> 3989.8, arith 4137.3 ->
3824.8, param_forms 6240.6 -> 5803.6; cbench_patterns fnmatch_* unchanged (ASCII path), wordexp_tilde_var
4087.0 -> 3974.3, wordexp_arith 4168.4 -> 3856.1. Every listed row is below base; the improved rows improved again.

## Jobs (mei)

Final head 534cae783: matcher-wordexp-rethink-gate2 (direct tier, 231 targets: mem.rs has 60 one-hop users) rc 1,
and the only failures are four niji tests that fail the same way in other stories' gates on this host
(heap-find-depth-need-2-gate, gate-honest-metrics-*): s7_2_f_0 (zh_HK.big5hkscs), printf_corpus_equals_the_reference,
locale_behaviour_equals_the_reference, catalogs_equal_the_reference; matcher-wordexp-rethink-plants2 5/5 caught;
precheck red only on gate2 (33 other checks ok). Superseded: -gate (code head cdeb7ccca, same four), -plants
(before the limits commit), -ck2 (tests, clippy, static, counts, percall; rc 1 only from the first version of the
stack test, fixed in cdeb7ccca; the counts above are from it). Limits: Skylake-SP rows added in
bench/instr_limits.tsv (2% headroom); the Zen 1 rows were not tightened (no Zen 1 count in this run).

## Next step

1. wordexp plain-text lane: a text with no quote, `$`, backquote, backslash, `~`, glob or operator byte splits on
   blanks straight to the sink (no Parser, Segs or Expander): bound ~1300 for plain_8 (now 3427); differential test
   against the full pipeline. 2. Tighten the Zen 1 fnmatch limits from a keke count. 3. The general matcher still
   compiles the byte program next to the character one for every non-ASCII pattern; compile it only for invalid text.
