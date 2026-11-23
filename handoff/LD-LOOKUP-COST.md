# LD-LOOKUP-COST: one symbol lookup at the reference's cost or better

**State (leg 2, 2026-11-22):** branch `task/ld-lookup-cost` from origin/next 0299fd6e1, code head 538b0970d
(handoff/report commits on top). Role mei `~/w/s2` (Skylake-SP). Decision D-27.77 (records/decisions/D-27/).
Brief: ~/state/briefs/LD-LOOKUP-COST.md. Result: a gain short of the brief's bound (600 per lookup): see below.

## Numbers (Skylake-SP host, fast products, user instructions exec to exit, median of 21; kit below)
| | next 0299fd6e1 | this branch | host C library (glibc 2.39) |
|---|---|---|---|
| empty dynamic main | 763,388 | **704,633** (-58,755) | 101,856 |
| `/bin/true` under the loader | 841,599 | **768,496** (-73,103) | 128,175 |
| 200 imports (204 GLOB_DAT) | 1,152,970 | 1,006,096 | 236,773 |
| **marginal per lookup** | **1,947** | **1,507** (-22.6%) | **674** |

Per step (same kit): s1 (chain loop, in-place names, origin by a second probe) 2,159 per lookup: WORSE, the
second definition-side probe walked the program's requirements and evicted memo slots; s2 (origin from the memo)
1,954, empty main 712,693; s3 (64-slot memo, one definition walk per requirement entry) 1,530, 707,929; s4 (memo hit
inline + fill cold, word-wise name hash) 1,507, 704,633. Every step after s1 improved every row.
`hot_text_products`' own count: fast 704,934, release 665,711 (both went red, more than 10% under):
`DYN_START_INSTR` is now 719,100 / 719,100 fast and 679,100 / 679,100 release. **Zen 1 host: NOT MEASURED**
(no role there this run); it takes the Skylake-SP bounds as its precedent rows did (levels within 0.02%).
`/bin/true -45k after LIBC-SELF-BIND`: not measurable here (LIBC-SELF-BIND not on next); -73k measured without it.

## What changed (D-27.77; every answer and error unchanged, no check dropped)
* elf.rs: `Symtab::get` one 24-byte chunk; `GnuHash::candidates` (Bloom + chain words in one loop, the iterator's
  errors); `Dynamic::hashed_string` (end + GNU hash, 8 bytes at a time while no NUL); `Dynamic::string_is`
  (in-place compare when the table ends in NUL, else `string`).
* symbols.rs: `Request::of_table` (plain name, hash, origin); origin = the reference's version came from its own
  object's definitions (memo key bit `MEMO_OWN`), so a definition of that object with that index matches without a
  compare; `VersionMemo` 64 slots of 16 bytes (len in the key), hit inline, `memo_fill` cold.
* reloc.rs: `target` uses the above; `check_versions_live` walks a provider's definitions once per requirement
  entry, keeping the first 64 names (`DEF_NAMES`), cold `version_defined` past them.
* Tests: symbols.rs `lookup_fast_paths_answer_as_the_plain_ones` (every symbol of the corpus + host libc: hash,
  string_is incl. prefix/extension, fast request vs plain in self and libv x4); reloc.rs
  `reloc_version_check_beyond_the_kept_definitions` (70 versions, reverse order, one missing).
* Plants: handoff/LD-LOOKUP-COST-plants.txt (7). plants-1 (6 of them, head 6bbe45561): 6 CAUGHT, no WARN;
  plants-2 (all 7, final code head): see the report.

## Remaining cost (Intel PT, n200 minus n0, per lookup, at s3)
`reloc::target` ~325 (name hash ~130 of it, now halved), `Object::lookup` ~220, its closure ~220, memo ~140 (now
inline), `symbols::lookup` ~120, pass1/one ~180, `Loads::inside` 44, `niji_machine_equal` 43, resolve 36. The
profile is flat straight-line code per call: the next step is structural (decode a relocation + its symbol once,
validate an object's tables once at `Object::new` and index inside the ranges, versions by index through a
per-object table; a (definer, index) answer for requirements computed in check_versions, which already pairs
each requirement with its provider's definition). Design B (per-start global name index) stays rejected.

## Final jobs (mei, code head 538b0970d)
* ld-lookup-cost-gate: direct tier, 9 suites, 849 passed, 0 failed, peak 2.9G.
* ld-lookup-cost-plants-2: 7 plants, 7 CAUGHT, no WARN (results in handoff/LD-LOOKUP-COST-plants.txt).
* ld-lookup-cost-hot: fast and release hot_text_products, 33 passed each (704,934 / 665,711).
* ld-lookup-cost-xtest: cargo xtask test, 7017 passed, **1 timed out**: dns_products resolv_equals_the_reference
  (1800 s; 0 s CPU, test thread in poll on the child's pipes, the timeout wrapper exited and unreaped, no claude
  process holding the pipe). Rerun alone (ld-lookup-cost-dns): passed in 98 s (1 passed, 0 failed): the timeout is intermittent, not chased (outside this change)
* SUPERSEDED: ld-lookup-cost-xtest (one intermittent timeout, its test rerun green as ld-lookup-cost-dns), ld-lookup-cost-s4 (rc=1 by design: found the bounds to tighten), plants-1 (older head), hot-1 (stopped,
  stale code).

## Kit (mei `~/work/ld-lookup-cost/`; copies in the scratch dir)
`count.sh PRODUCTS SRC [RUNS] [pt]`, `build_count.sh TAG`, `step.sh TAG` (ld tests + build + counts + PT),
`pt2.sh TAG [N]` (PT of n0 and n200, per-symbol difference = marginal), `pt3.sh TAG SYM...` (hottest instructions,
perf annotate on PT), `hot.sh TAG` (fast + release products and hot_text_products), `ptdiff.py`.

## Next step
Coordinator: land as a gain short of the bound; cut the structural follow-up above (LD-LOOKUP-COST-2) and a Zen 1
count (copy a products dir + kit to the Zen 1 host, run count.sh; no build).
