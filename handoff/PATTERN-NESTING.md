# PATTERN-NESTING handoff (HL-13 fnmatch, HL-14 wordexp nesting)

State: branch `task/pattern-nesting-2` (PATTERN-NESTING-2, cut from `task/pattern-nesting`), role
keke:~/w/n1 (Zen 1 build host). Plants file: `handoff/PATTERN-NESTING-2-plants.txt` (12 plants:
6 HL-13, 6 HL-14). Scratch: `~/w/.scratch/PATTERN-NESTING-2/`; on the host `~/work/pattern-nesting-2/`.
Code head: 11b46b216 (unchanged in leg 3; leg 3 only measured and wrote this note and the report).

## Verdict at the end of run PATTERN-NESTING-2: INCOMPLETE (do not land yet)

Both limits are removed and tested, but wordexp bench rows are still slower than base, and one fnmatch
row is above its Zen 1 instruction limit (owner rule: rows must not move, bounds not loosened).

## Done

- HL-13 (fnmatch, FNM_EXTMATCH nesting): 6e92bc806; plants 1a1b2911b.
- HL-14 (wordexp): parser frames + `nest`, expander job stack, evaluator `Wait` stack; 128 and
  512 limits removed (792e6f763); tests + 6 plants 15f98b743; fast paths 11b46b216 (`segs` direct walk,
  `braced` plain word, cheap `InlineVec::pop`).
- Counts (cbench_opt/cbench_patterns, rows `wordexp_*`, `fnmatch*`; niji_instr per call). Base
  = 1a1b2911b (HL-13 in, wordexp as on next) `counters-base.tsv`; after2 = 11b46b216 (job
  `pattern-nesting-2-counts-after2`, `counters-after2.tsv`; compare `python3 cmp.py BASE AFTER` in
  `~/work/pattern-nesting-2/`):
  | row (cbench_opt) | base | after (792e6f763) | after2 (11b46b216) | glibc |
  | wordexp_arith | 4141.1 | 4799 | 4556.5 (+10.0%) | 3538.8 |
  | wordexp_param_forms | 6245.6 | 7066 | 5940.9 (-4.9%) | 5248.4 |
  | wordexp_quoted | 3784.4 | +9.9% | 4043.8 (+6.9%) | 2560.7 |
  | wordexp_plain_8 | 3782.9 | +2.7% | 3921.9 (+3.7%) | 6386.9 |
  | wordexp_tilde_var | 4104.1 | +8.8% | 4199.9 (+2.3%) | 3056.0 |
  | wordexp_split_var | 3798.5 | +5.1% | 3847.2 (+1.3%) | 3504.4 |
  cbench_patterns: wordexp_arith 4170.8 -> 4588.4 (+10.0%), wordexp_tilde_var 4089.6 -> 4186.9 (+2.4%).
  fnmatch rows: base -> after2 within -1.8..+0.04% (wordexp work does not touch them).
- LIMIT FINDING: `bench/instr_limits.tsv` (AMD EPYC 7551) `cbench_opt fnmatch_names_ch` max 761;
  measured 784.4 (base, HL-13 in) and 784.2 (after2). Not yet known whether origin/next is also over
  (no next count of that row exists on the host: the CI instr counts do not include fnmatch rows), i.e.
  whether HL-13 (6e92bc806, it changes `Pattern` and the matcher) moved it. OBSERVED: the counts jobs
  (rows chosen with `--rows "cbench_opt/fnmatch*,..."`) printed no instruction-limit verdict and ended
  rc 0 although 784.2 > 761. Cause NOT investigated (xtask bench.rs `check_instr_limits` checks only
  rows in its `selected` set, built from the bench table's rows; whether `--rows` selections reach it
  is unverified). Do not treat a green counts job as "within limits": compare against the tsv.

## Running at hand-back

- `pattern-nesting-2-plants2` (all 12 plants on 11b46b216, /dev/shm/claude/pattern-nesting-2/plants-2):
  5 of 12 CAUGHT at 14:10Z (no-memo, depth-cap, ...), ends about 14:50Z. Read verdicts:
  `grep -E ": CAUGHT|SURVIV|WARN-RC|WARN-NO" ~/logs/pattern-nesting-2-plants2.log`.
  Not run: the direct-tier gate on 11b46b216 (the role was held by plants2; code will change anyway).

## Next steps, in order

1. Count origin/next's rows on the role, exact rows so the limit is checked:
   `cargo xtask bench --compare --counters cbench_opt cbench_patterns --rows "cbench_opt/fnmatch_names_ch,..."`
   on an origin/next tree (git worktree of origin/next synced with sync_role.sh --story; then re-sync the
   branch). If next is <= 761 and HL-13 is 784: profile fnmatch_names_ch (a non-extmatch pattern) and
   keep the non-EXTMATCH path as it is on next (the flat program only for extended patterns).
2. wordexp rows (profile of 792e6f763 in `~/logs/pattern-nesting-2-trace-new.log`, arith split):
   evaluator `run` 448, `push<Wait,8>` 93 (an out-of-line call per push), `copied/last/as_slice<Wait>`
   ~146, `default` 56 + `new<Seg,12>` 60 (`InlineVec::new` fills all N slots with `T::default()`:
   8 x 24-byte `Wait` and 12 `Seg` per call), `checked_sub` 158, `saturating_add` 105.
   Candidates: a `MaybeUninit` inline area or a smaller N for the evaluator stack (only the used slots
   are written); `#[inline]` on `InlineVec::push` (measure: shared type); merge `Wait::Test` into the
   level-0 `Binary` (one push fewer per expression); for plain_8/quoted the per-word cost (`Seg` list
   init, job stack set-up): trace after2 first. RUT RULE: this is the second tuning pass; if it gains
   little, RULING NEEDED (accept the cost for the removed limits, or keep a cheap common path).
3. Then plants (all 12, new dir plants-3), gate (`cargo xtask affected --run` under ~/bin/gate 16G),
   clippy + static, precheck, report, handback-check.

## Not to redo

HL-13 code and tests; HL-14 code; the leg-2 profile. Plan rows: `plan/hardcoded-limits.md` HL-13,
HL-14 (no status column). No D-row needed so far (no spec deviation; limits removed per the owner's
11-17 ruling and the coordinator's #7901 decision).
INFORMATIONAL: a pattern removal/assignment/arithmetic text with expansions now allocates the nested
word's pieces on the heap (`fresh`, one malloc per call that has one); no bench row covers it.
Full report: `handoff/reports/PATTERN-NESTING-2.json`.
