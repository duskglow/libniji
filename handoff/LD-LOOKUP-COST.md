# LD-LOOKUP-COST: one symbol lookup at the reference's cost or better

**State (leg 1 ending, 2026-11-21 ~23:20Z):** branch `task/ld-lookup-cost` from origin/next 0299fd6e1; WIP commit
06ff9f2ad (step 1 below, not yet tested: no local cargo). Role mei `~/w/s2` (Skylake-SP). Job running:
`ld-lookup-cost-s1` (ld tests symbols/reloc/unique_table, then `cargo xtask so`, products kept as
`~/work/ld-lookup-cost/s1`, counts + Intel PT profile): `~/bin/jobwait ld-lookup-cost-s1`, read
`~/logs/ld-lookup-cost-s1.log`. Brief: ~/state/briefs/LD-LOOKUP-COST.md; review (sections 2, 5, 6):
`git show origin/task/startup-structure-review:plan/reviews/STARTUP-STRUCTURE-REVIEW.md`.

## Measurement kit (mei `~/work/ld-lookup-cost/`; copies in the scratch dir)
* `gen.py NIJI_LIBC GLIBC_LIBC N`: C program taking N libc function addresses through the GOT (GLOB_DAT), names
  exported at a default version by both libraries.
* `count.sh PRODUCTS SRC [RUNS] [pt]`: links n0/n200 like hot_text_products' dyn_link, g0/g200 with the host
  compiler, `/bin/true` with PT_INTERP rewritten (same length, `/tmp/q` symlink); exec_instr median of RUNS,
  ASLR off; `pt`: Intel PT per-symbol table of n200 (perf intel_pt, exact).
* `build_count.sh TAG [RUNS] [pt]`: `cargo xtask so` on the role, products kept in `~/work/ld-lookup-cost/TAG`,
  then count.sh. `step.sh TAG`: ld tests, then build_count.sh TAG 21 pt.
* Zen 1 host (AMD EPYC 7551, keke) not measured yet: copy a TAG dir + kit there and run count.sh (no build).

## Baseline (job ld-lookup-cost-base, next 0299fd6e1, fast products, Skylake-SP, median of 21)
| | libniji | host C library (glibc 2.39) |
|---|---|---|
| empty main | 763,388 | 101,856 |
| 200 imports (204 GLOB_DAT) | 1,152,970 | 236,773 |
| **marginal per lookup** | **1,947** | **674** |
| /bin/true | 841,599 | 128,175 |

PT profile of n200 (ld.so, whole run, ~488 symbolic lookups): `reloc::target` 182,338 excl; `Object::lookup`
closure 145,304; `Object::lookup` 113,684; `pass1` 90,360; `read_phdr` 62,620; `memo_name` 61,519;
`symbols::lookup` 57,936; `Verdefs::next` 51,349 (who calls it per lookup: NOT VERIFIED, maybe check_versions or
version_name's walk); `one` 42,804; `GnuHash::symbol_count` 30,935; `check_versions` 29,570; `Versions::def_name`
28,925; `niji_machine_equal` 20,999; `resolve` 17,652.

## Design (RETHINK §11)
* **A (chosen): validated-once layout, tight hot path.** Keep every answer and error identical; remove the layers
  per step: one-chunk symbol read, a single loop for Bloom + chain bytes, the name's end and hash in one pass,
  in-place name compare when the string table ends in NUL, versions by index (an object's own definition: origin
  shortcut; other objects: a (definer, index, wanted name) match cache next). Estimate: ~300-500 per lookup.
* **B (rejected): a per-start global name index over the scope** (build a hash of every exported definition once,
  then O(1) per lookup). ~45k per start to build for libc's ~3,000 symbols plus allocation and dlopen
  invalidation; wins only above ~100 lookups and loses after LIBC-SELF-BIND (~30 lookups per start).

## Step 1 (commit 06ff9f2ad): what changed
* elf.rs: `Symtab::get` one 24-byte chunk (`Sym::decode`); `GnuHash::candidates` (Bloom + chain loop, the
  iterator's errors incl. the u32::MAX overflow case); `Dynamic::hashed_string` (name + GNU hash, `string`'s
  errors); `Dynamic::string_is` (in-place compare, needs a NUL-free name; falls back to `string` when the table
  does not end in NUL).
* symbols.rs: `Request` gains `plain` (name NUL-free; `new` checks, `of_table` asserts by construction) and
  `origin` (referencing object's strtab address + version index when `ptr::eq(def-side name, wanted name)`);
  `Object::reference_want_origin` (an error on the def-side probe only gives no origin);
  `for_each_candidate` GNU arm -> `candidates`; the closure uses `string_is`; Want::Reference exact via origin.
* reloc.rs `target`: `hashed_string` + `reference_want_origin` + `Request::of_table`.

## Next steps (in order)
1. `jobwait ld-lookup-cost-s1`; fix compile/test errors; read counts vs baseline. A change that measures worse:
   profile (PT section of the log) before another design.
2. Next costs by profile: def-side version names for other objects (match cache keyed by definer strtab, index,
   wanted name ptr/len; only Ok answers cached), `memo_name`'s own cost (16 slots collide for libc's ~45
   versions), `symbols::lookup`/`resolve` per-object overhead (scope iter, `unique.bound` dyn call), `target`.
3. Tests: a new test that the fast paths answer as the slow ones (string_is vs string on corpus + host libc with
   a table not ending in NUL; candidates vs chain; origin). Plants (docs/plants.md lines, `cargo xtask plants`) on
   the Bloom reject (`word & mask != mask`), `string_is` NUL check, the origin index compare.
4. The empty-main bound `DYN_START_INSTR` (crates/core/tests/hot_text_products.rs:489) has a 10%-under check:
   a big gain makes it red -> tighten to measured fast level + 2% on both CPU models (Zen 1: keke counts).
5. Decision record: reserve `python3 ~/src/liella-ops/bin/ops reserve --story LD-LOOKUP-COST D-27` (adopt A,
   B rejected, numbers). Saturation row if a step measured worse.
6. Hand-back checks: clippy (`cargo clippy --workspace --all-targets --all-features -- -D warnings`), `cargo xtask
   static`, direct gate (`cargo xtask affected --run` under `~/bin/gate 16G`), plants, `cargo xtask test`,
   `NIJI_FULL_PROFILE=1 cargo xtask so && NIJI_FULL_PROFILE=1 cargo test -p niji --test hot_text_products`,
   layout-bins diff vs base, precheck, report json, handback-check. Wall deadline 2026-11-22T02:32Z.
