# WCTYPE-TRIE-FILE(-2..-9): lazily resolved class tables for compiled LC_CTYPE + the thread-block table word (D-5.10, D-5.11)

**State:** branch `task/wctype-trie-file-9` (continues `-8`..`-2`; origin/next merged in runs 3, 5 and 7). Product code
is run 8's (f8da29874); run 9 added only a test and a `#[cfg(test)]` hook (2e0c76915). Decisions in force: **D-5.10** as amended by the pending record
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
* Not run: the EPYC instruction rows on keke (my role is kanon only); the counter bench ran on kanon. Product code is
  unchanged since run 8, whose counts stand.

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
