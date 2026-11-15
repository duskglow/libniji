# WCTYPE-TRIE-FILE(-2..-9): lazily resolved class tables for compiled LC_CTYPE + the thread-block table word (D-5.10, D-5.11)

**State:** branch `task/wctype-trie-file-11` (continues `-10`..`-2`; origin/next merged in runs 3, 5 and 7). Product code
is run 10's (eb46f66a4); run 11 changed no code (its red was the job's length, below). Decisions in force: **D-5.10** as amended by the pending record
`records/pending/WCTYPE-TRIE-FILE-8-1.toml` (D-NEW, series D-5, amends D-5.10), **D-5.11** (owned() gate, ruling
#4281), D-5.9, D-5.5, D-7.13, D-X.58.

## What the story does (runs 1-2)
`ClassTable` (crates/core/src/locale/classes.rs): stage 1 of 4352 32-bit cells, built-in leaves (D-5.9) or pool leaves,
resolved per 256-character block; `ThreadBlock::wclass` (0x180) stored with `view` by every writer (only when
`owned()`); `isw*`/`iswctype` read it with one `%fs` load and `& owned_mask()`.

## Run 8: CI red on startup_instr_wctype (228,598 vs limit 70,900) and startup_instr_c (2,156 vs 2,150)
* Step 0 on the inherited head cb557945b: clippy + static green (shiki `wctype-trie-file-8-lint0`).
* **Profile (shiki, wl_trace, spec ~/work/WCTYPE-TRIE-FILE-8/wct8.c on shiki; copy in this note's numbers):** first
  `iswalpha(U+00E9)` under en_US.UTF-8 = **160,256** instructions, 160,065 (99.9%) in `ClassTable::resolve`: 256
  characters x 12 class-table walks (`w_classes_file` per character), + 102 in the leaf compare. next: under 200
  (cold row 0.0 at 200-instruction resolution).
* **Fix 1, miss count:** an unresolved block counts its misses in its stage-1 cell (values from `COUNT_BASE` 0x10000,
  which name no leaf, so `hit`/`test` are unchanged); misses 1..64 (`RESOLVE_AFTER`) answer by one walk of the asked
  class (`ClassTable::test_miss`, called by `LocaleView::isw_wide`); miss 65 resolves. Publishing CAS loops over a
  changed count.
* **Fix 2, block build:** `Table3::bits32` (one walk per 32 characters when both shifts >= 5, else 32 bit walks) and
  `block_words` (mask ORed into each member's word): resolve 160,065 -> **10,342** (call 65: 10,609 total).
* **Fix 3, start-up C row:** `store_words` no longer tests `owned()` before `store_view` (which tests it itself).
* Counts (wl_percall, instructions/call, glibc / next f6cab48d7 / head): cold_iswalpha_e9 0.0/0.0/0.0 (was 8.0 =
  160k/process), cold_isw3_e9 0.0/0.1/0.0, cold_iswupper_3a9 0.0/0.0/0.0, warm_iswalpha_e9 53.0/32.3/32.9 (was 40.0),
  warm_iswspace_mix (28 blocks, warm-up) 47.7/50.0/58.5 (was 237.0). First miss = 128 instructions; resolve = 10.6k;
  64 is the ski-rental point (10.6k / ~110 surplus per miss ~ 96), so a block's total cost is <= 2x the best choice.
* Tests: `class_table_resolves_at_its_miss_threshold` (per word and per class; the slots' count), 
  `table3::tests::bits32_is_bit_of_each_character_with_small_shifts`. Plants: handoff/wctype-trie-file-8-plants.txt
  (10) + handoff/wctype-trie-file-plants.txt (2 stale patterns repointed: resolved-test, publish-order).
* Jobs (shiki): `wctype-trie-file-8-check3` (clippy, static, tests, counts, traces all green; rc 1 only because the
  bench step had the wrong syntax), `wctype-trie-file-8-startup` (bench --compare --counters cbench_startup, cargo
  xtask test, full-profile so + hot_text_products), `wctype-trie-file-8-gate` (direct tier, after startup),
  `wctype-trie-file-8-plants`. Results: see the hand-back / report json.

## Run 9 (kanon kn1, Broadwell-EP): the surviving walk-marker plant
* Step 0 on the inherited head 7c5c0ff1d: clippy + static green (`wctype-trie-file-9-lint0`).
* Why walk-marker survived: it mutates the early return of `ClassTable::resolve` for a block already resolved without a
  leaf (WALK). `word()`/`test_miss()` never reach it: `due()` declines a resolved block, so only a thread that passes
  `due()` and then loses the race to a full-pool resolution takes that branch. (The run-8 note calling it the "32
  characters per walk" marker was a misreading: the block build is covered by `file_class_tables_equal_the_file_walk`,
  every block of both locales.) Fix: `ClassTable::resolve_late_for_test` (cfg(test)) runs `resolve` on an already
  resolved block, making that race deterministic; test `late_block_resolution_answers_by_the_winner`: blocks 0x00,
  0x03, 0x1f, 0x4e of en_US.UTF-8 and the system C.utf8, shared and pool leaves, winner left WALK or published a leaf:
  every character equals `w_classes_file`, and the late resolution writes nothing (census unchanged).
* publish-order stays EQUIVALENT, now with evidence: libc.so.6 built with and without the mutation in a RAM copy
  (`wctype-trie-file-9-ordercheck`, script ~/work/WCTYPE-TRIE-FILE-9/ordercheck.sh on kanon) has identical disassembly
  apart from `.llvm.<hash>` suffixes of local names. Memory-model argument in the plants file.
* Plants (handoff/wctype-trie-file-plants.txt, 14): 13 CAUGHT, 1 EQUIVALENT, 0 WARN. Jobs: see the hand-back / report json.
* **startup_instr_c regression found and fixed.** cbench_startup counters on kanon (Broadwell-EP): merge base
  f6cab48d7 `startup_instr_c` 2,148 / `startup_instr_wctype` 68,489; head 463071ca0 2,160 / 67,961 (jobs
  `wctype-trie-file-9-nextcounts`, `-counts`). gdb single-step of `cbench_startup true`, per instruction, head vs base
  (script ~/work/WCTYPE-TRIE-FILE-9/stepboth2.sh, stepcount.py on kanon): +4 in `locale::refresh_words` (the D-5.11
  `wclass` store, inherent) and +8 in `runtime::start::<false>`, whose code is identical: its program-header loop ran 9
  times, not 8. Cause: `Mode::File { share, after }` made every `FILE_TABLES` slot non-zero, so the 16 slots (278 KiB)
  went to `.data` instead of `.bss` and the static binary gained a loadable segment (`readelf -l`). Fix (commit after
  463071ca0): `Mode` is a fieldless `repr(u32)` enum with `File = 0`; the slots' (share, after) = (true,
  RESOLVE_AFTER) come from `ClassTable::file()`, tests set theirs in a `#[cfg(test)]` field. **Result
  (`wctype-trie-file-9-fix-counts`): FILE_TABLES and POOL are now NOBITS (.lbss); .data 0x70a02-0x7000 -> 0x2597a
  (base 0x18d6a: +52 KiB = C_TABLE + C_UTF8_TABLE, non-zero by design), but the binary still has 6 LOAD segments
  (base 5) and startup_instr_c still reads 2,160 (+12).** The split now is .data1/.bss/.lbss in their own RW LOAD;
  why the linker splits at this .data size and not at the base's is NOT found (budget).
* **Next step (first):** find what makes the 6th LOAD (`readelf -lW` head vs base; the link script / layout tool that
  places `.data1` at 0x4000 alignment; whether .data crossing 0x20000 triggers it). Options: keep C_TABLE/C_UTF8_TABLE's
  stage 1 out of `.data` (they are never written: a `.rodata` stage 1 behind the same type), or fix the split. Then
  re-measure startup_instr_c on base and head on one host. CI's EPYC limit (2,150) decides whether this lands red.
* The EPYC instruction rows are checked on keke by CI; my role is kanon only, so the counter bench ran on kanon.

## Run 10 (keke n4, Zen 1; branch `task/wctype-trie-file-10`): CI red STARTUP-DATA-WINDOWS 9 > 7 (C_TABLE)
* Step 0 on d98e7aac8: clippy + static green (`wctype-trie-file-10-lint0`). This head alone (fast products, keke,
  `wctype-trie-file-10-so0`) holds STARTUP-DATA-WINDOWS at 7 pages: only `LocaleView::new` and the legacy C view
  (`.data1`) reference `C_TABLE`. CI built a merge with next (255 commits ahead of the merge base f6cab48d7), where
  something in start-up addressed `C_TABLE` (17 KiB in `.data`, head over pages 0x863000/0x864000). I could not
  reproduce that tree (no merge allowed in this brief).
* **Fix (eb46f66a4):** no table has a non-zero byte any more. `Mode` is not stored; `ClassTable::mode()` (miss path
  only) works it out from the address: `C_TABLE`/`C_UTF8_TABLE` -> `Builtin(stage 1 bytes)`, `WALK_TABLE` -> `Walk`,
  else a slot (`File { share: true, after: RESOLVE_AFTER }`); tests set `test_mode`. A built-in block resolves at its
  first miss (`resolve_builtin`: one Relaxed store of `s1[block] + 1`; every thread stores the same value, leaves are
  immutable statics). So C, C.UTF-8 and the walking table (52 KiB) leave `.data` for `.bss`/`.lbss`, which is
  what kept the 6th LOAD segment (run 9's +8 on `startup_instr_c`); start-up neither reads nor writes them, and
  a start-up reference to them can only be an address. Hit path unchanged.
* Test `locale::tests::builtin_class_tables_resolve_a_block_at_its_first_miss`; plants
  handoff/wctype-trie-file-10-plants.txt (6). Results: hand-back / report json.

## Run 11 (mei s3; branch `task/wctype-trie-file-11`): verify red, `wctype-trie-file-10-final` rc 124 on keke
* **Not a hang: the job used up its 9,000 s cap.** Its script (keke ~/work/WCTYPE-TRIE-FILE-10/final.sh) chained
  `affected --run` (escalated to FULL by `crates/libc-so/startup-state.list`, which runs `cargo xtask test` itself:
  6,092 s, green), a second `cargo xtask test` (2,711 s, green), then `LIELLA_FULL_PROFILE=1 cargo xtask so`. The
  release trace started 05:38:26 and the cap ended the job at 05:40:58, 152 s into it. The fast-profile trace of the
  same job had finished (startup 103, hot 511, probe 154 keys).
* Reproduced alone on this head (mei, load ~38, `wctype-trie-file-11-startup`): release `so` with its trace 6 min 18 s
  (trace ~77 s: startup 104, hot 456, probe 156 keys, 80 start-up data objects), then `hot_text_products` 26 passed.
* Code review of the miss path: `resolve_builtin` takes no lock, makes one Relaxed store and calls nothing that
  classifies a character. The only loop is the publishing CAS in `resolve`, which ends once the block is published.
  No fix and no hang test: there is no hang to guard.
* Jobs on this head, each on its own so none can starve another of the cap: `wctype-trie-file-11-lint0` (clippy +
  static), `-startup` (full-profile so + hot_text_products), `-plants` (run 10's 6 plants), `-gate` (the gate,
  FULL by escalation, which includes `cargo xtask test`). Results: hand-back / report json.

## Still open
* The EPYC instruction limits are checked on keke by CI; this run measured on shiki (Haswell-EP) only. startup_instr_c's
  -6 is not measured per instruction here (no next build of cbench_startup on shiki): CI's keke row decides.
* `_l` rows (ruling #4173: astral `_l` 20/21 vs 12; profile `view_of(loc)` first); period-cache own-process test; timing.
* warm_iswspace_mix stays +8.5/call over next while 28 blocks warm up (one-off per block; steady rows are at parity).

## Next step
1. If CI's keke instruction rows are red: read the cbench_startup ctr lines of the CI log; startup_instr_c: disassemble
   `store_words`' caller in libc.so.6 on head vs next. 2. `_l` rows. 3. timing.
Do not redo: the stage-1 encoding, the shared pool, `.bss` slots, the 32-bit cells, the Latin alignment experiment, the
cold out-of-line resolve-only path, `cold_path`; the run-7 merge; the run-8 profile (numbers above); run 9's
publish-order codegen check (identical code; re-run only if the publishing CAS or `pool_add` changes).
