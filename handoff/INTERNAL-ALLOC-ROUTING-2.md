# INTERNAL-ALLOC-ROUTING-2 (handoff; end of leg 2)

State: branch `task/internal-alloc-routing-2`, stacked on INTERNAL-ALLOC-ROUTING (code head fbbb0d309, base
origin/next 279a24ee4, not merged). Role keke:~/w/n3. Leg 2 code: 1b9ba980e + "Drop an unused routing helper"
(variant (b), atomics). Job `internal-alloc-routing-2-build2` (clippy + static green, then `cargo xtask so`) was
still linking at hand-over: `~/bin/jobwait internal-alloc-routing-2-build2` first; its product is the (b) build.

## Rulings in force
#7583, #7696, #7697, #7698; **#7707** (coordinator): libc-side exact test once in early_init, zero added per
allocation, report the exact per-start cost in the hand-back's first line, DYN_START_INSTR not loosened; printf
(step 2) STAYS in this story, size M (use `ops extend-budget` when needed). **#7708 (OWNER): option (a)**: the
`process-flag` unsafe category now covers the two write-once heap.rs words as well as OWNED_HERE. Switch from
atomics to plain reads, each with its `// UNSAFE(process-flag)` reason + SAFETY lines, list exactly those three
reads in xtask/src/policy.rs (PROCESS_FLAG_FILE / PROCESS_FLAG_READ are single values today: make them a list of
(file, body) pairs), cite #7708 there and in the commit. Report BOTH measurements: (b) counts and (a) counts, per
start and per internal allocation.

## What leg 2 did (design; do not redo)
* rt `Heap::alloc_below(sys, pages, size, align, end) -> Result<Option<Block>, Routed>` (new `Routed` type):
  `align < end` = alloc's path; `align > ALIGN` = class_for path; else `Err(Routed)`. Removed predecessor's
  `Heap::untouched` (unused now).
* boundary heap.rs: `Niji` methods back to next's bodies, except `dealloc` picks `rust_dealloc_private` when
  `layout.align() > ALIGN` (folds at compile time). Words `FAST_ALIGN_END` (ALIGN+1 / 0 routed) and `RESERVE_LAST`
  (RESERVE-1 / usize::MAX routed), set by `route_internal()`. `rust_alloc(_zeroed)` use `alloc_below` with
  FAST_ALIGN_END; `rust_dealloc` tests `reserve_or_routed(addr)` (`addr - base <= RESERVE_LAST`) then cold
  `release_cold` (reserve block -> reserve, else replacement's free); `rust_realloc` uses `reserve_or_routed`
  and its fallback frees by alignment. Over-aligned internal blocks stay private (D-X.156).
* caller_alloc.rs: ROUTE/ROUTED/ROUTED_CHECKED removed; `family_replaced` = one C call
  `niji_link_alloc_family(w[6])` (runtime.c, adapter: no logic) + branch-free compares; `decide_route` calls
  `heap::route_internal`. startup-state.list: ROUTE replaced by the two words.
* Why not route frees through the misuse path: a forwarding replacement (wrapper calling the real malloc) hands
  back heap blocks, whose frees would then bypass its `free`.

## Next steps (leg 3)
1. jobwait build2. Bench (b): base vs head as the predecessor did (handoff/INTERNAL-ALLOC-ROUTING-report.md:
   RAM copies, `cargo xtask bench --compare --counters` startup_instr_setloc/_wctype/_printf/_c, STARTUP-REPROFILE
   kit rows, DYN_START_INSTR). Record (b) numbers in the report.
2. Switch to (a) (plain reads, policy.rs list, #7708), build, disassemble `rust_alloc`, `rust_dealloc`,
   `rust_alloc_zeroed`, `rust_realloc` on head vs base (same instruction count on the ordinary path), bench (a).
   Rerun `mint` and `x_internal_alloc` (conformance; the predecessor's plants file names the command).
3. Step 2 printf (#7698): probe reference with counting and refusing replacements over the brief's corpus;
   no allocation where the reference makes none (D-12.12 tables static/stack for common conversions).
4. Tests + plants (docs/plants.md form, `cargo xtask plants`): a per-allocation route test put back on the
   ordinary path (caught by a counts/hot-text test or disassembly-based test), a heap table for a plain %d.
   The predecessor's `no-route` plant text no longer matches (now `if family_replaced() {`).
5. Start-up checks (§9): `cargo xtask test`, `NIJI_FULL_PROFILE=1 cargo xtask so && ... hot_text_products`;
   direct gate; precheck; D-X.156 text update; report json.
