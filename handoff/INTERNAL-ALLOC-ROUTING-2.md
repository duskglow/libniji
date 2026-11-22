# INTERNAL-ALLOC-ROUTING-2 (handoff; leg 1 = analysis only, no code yet)

State: branch `task/internal-alloc-routing-2`, stacked on INTERNAL-ALLOC-ROUTING (code head fbbb0d309, base
origin/next 279a24ee4; origin/next is now b69d50e78, not merged). Role keke:~/w/n3 (synced at leg 1, tree
4277ae0c, nothing built there yet). Read first: handoff/INTERNAL-ALLOC-ROUTING.md and -report.md (start-up table,
mint table, plants file), then this page. Rulings in force: #7583, #7696, #7697 (dlopen exception accepted), #7698.

## Where the predecessor's start-up cost comes from (read from its diff and an older product's disassembly)
* `heap::Niji`'s GlobalAlloc methods are inlined at every call site (an older product: 162 calls of
  `heap::rust_alloc`, 1863 of `heap::rust_dealloc`, 4 of `rust_realloc`, all out of line). The predecessor's
  `caller_alloc::route() != PRIVATE` test sits in those inlined bodies: a load and a branch per internal
  allocation, release and resize (about two thirds of +137..+187), plus `decide_route` (six C wrapper calls and
  the compares, about a third).

## Design for leg 2 (no instruction added on the ordinary path)
1. Put `Niji`'s four methods back to next's bodies exactly (no route test at the call sites).
2. Turn two compares the ordinary path ALREADY makes into compares against start-set plain words. Probe (rustc
   on the build host, -O, PIC cdylib): a Relaxed atomic load is NOT folded (`mov` + `cmp`, one extra instruction);
   a plain read of an `UnsafeCell<usize>` in a `Sync` wrapper that some function writes IS folded:
   `cmpq W(%rip), %rax` - the same count as `cmpq $65536, %rax`. So (niji-boundary heap.rs, UNSAFE comments; written
   once in early_init, before any thread and before the library's first allocation; read-only afterwards):
   * `FAST_ALIGN_END` (ordinary `ALIGN + 1`, routed 0) replaces the `align <= ALIGN` test of the fast path in
     `rust_alloc` / `rust_alloc_zeroed` (needs a niji_rt `Heap` fast-path variant taking the bound; `malloc` keeps
     the constant, so its code does not change). Shape: `align < end` -> as today (small, then class_for);
     `align > ALIGN` -> today's over-aligned path (stays private, as D-X.156 already says; +2 instructions only for
     an over-aligned internal allocation); else (only possible when routed) -> cold `routed_alloc`.
   * `RESERVE_SPAN` (ordinary `RESERVE`, routed `usize::MAX`) in `in_reserve`: when routed, `rust_dealloc` and
     `rust_realloc` take the reserve branch, which checks the route FIRST (cold): ROUTED -> `caller_alloc::free` /
     `realloc`; ROUTED_CHECKED keeps the predecessor's reserve / `HEAP.usable` test. Check every other user of
     `in_reserve` (reserve_alloc, the stdio slots) keeps its meaning (maybe keep a constant-bound twin for them).
   * Verify by disassembly of target/niji/libc.so.6 on head and base: `heap10rust_alloc`, `heap12rust_dealloc`,
     `rust_alloc_zeroed`, `rust_realloc` bodies the same length; call sites with no `ROUTE` load.
3. The decision, once, in `early_init`. The brief says StartInfo; finding (challenge flagged at leg 1): libniji's
   ld.so keeps no "replaced" state. Deriving it there costs either a name check on every relocation bound outside
   its own object (copy relocations do that in every ordinary program) or four bloom/hash lookups in the objects
   ahead of libc (about 8+ instructions each). The exact and cheapest test is libc's own: its four GOT entries
   (`&malloc` .. `&free` in csys runtime.c) against hidden local aliases of its own definitions, one expression
   (~14 instructions, no loop, no six calls). Use StartInfo only if the coordinator answers the challenge so.
   Either way the start keeps a few instructions (the read and the branch): report the exact delta, never "equal".
4. Bench as the predecessor did (base and head each in a RAM copy, `cargo xtask bench --compare --counters`
   startup_instr_setloc / _wctype / _printf / _c, the STARTUP-REPROFILE kit rows, DYN_START_INSTR), and rerun `mint`
   (the predecessor's probe: its report names it) and `x_internal_alloc`.

## Step 2 (printf, #7698): not started
Probe the reference with a counting and a refusing replacement over the brief's corpus; where it makes no call,
libniji must make none (D-12.12 format tables: keep them on the stack or in static storage for the common
conversions). The brief sizes the story S; with step 1 it is two changes (challenge recommends a separate story).

## Next step
Leg 2: implement step 1 (items 1-3), build (`cargo xtask so` on keke n3 under jobrun), disassemble, bench base vs head.
