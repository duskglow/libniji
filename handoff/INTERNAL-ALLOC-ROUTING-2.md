# INTERNAL-ALLOC-ROUTING-2 (handoff and report; end of run 1, leg 4)

State: branch `task/internal-alloc-routing-2`, stacked on INTERNAL-ALLOC-ROUTING (base origin/next 279a24ee4, not
merged). Code head 141c3095e = variant (a) (plain reads, ruling #7708); 6beb1da01 adds the printf check to
x_internal_alloc and the plants file. Verdict: INCOMPLETE (step 2, printf, not done; step 1 adds 2 instructions per
allocation, see RULING NEEDED). Role keke:~/w/n3.

## Rulings in force
#7583, #7696, #7697, #7698, #7707 (zero added per allocation; per-start cost in the first line; DYN_START_INSTR not
loosened; printf stays in this story), #7708 (OWNER: plain reads of the two heap words under `process-flag`).

## Step 1: the ordinary path (disassembly of libc.so.6, base 279a24ee4 vs head (a) 141c3095e)
Script `fastpath.py` (scratch) over `disasm.sh` output (keke ~/work/internal-alloc-routing-2/dis-base.txt, dis-a.txt).
* `rust_alloc` (172 -> 148 instructions in all): base's fast path has NO alignment test: LTO proved every call
  site's alignment is at most 16 and folded the constant `align <= ALIGN` away. Head reads `FAST_ALIGN_END` from
  memory, which cannot fold: `cmp FAST_ALIGN_END(%rip),%rsi; jae` = **+2 instructions per allocation** (the rest of
  the fast path is the same instructions, reordered). #7707's "zero added per allocation" is not met on this side.
* `rust_dealloc` (226 -> 180): base tests the reserve range with two compares and a cmov behind three pushes; head
  is `lea; mov; sub; cmp RESERVE_LAST(%rip); jbe` behind one push: about **-5 instructions per release** (and two
  fewer pushes/pops).
* `__rust_realloc` (256 -> 254): same entry, the reserve test as in dealloc.
* Per start: `decide_route` = `family_replaced` (one C call, `niji_link_alloc_family`) in `early_init`; on a
  program that does not replace malloc nothing more runs. Start-up user instructions (bench cbench_startup, Zen 1
  host, leg 3): c 2105 -> 2105 (0), setloc 62056 -> 62089 (+33), wctype 62547 -> 62567 (+20), printf 65627 ->
  65631 (+4). The mix of +2 per allocation and -5 per release explains why the rows move unevenly (setlocale
  allocates and keeps more at start than the others release).
* Zero-cost design not tried (needs a ruling: more `process-flag` words than #7708's two): put the routing bound on
  the SIZE compares the paths already make (fast-path size cap, `class_for`'s MAX_SMALL, `alloc_slow`'s
  MAX_REQUEST become words, 0 when routed) and decide in the cold fallback after the heap answers None (routed and
  align <= 16 -> replacement; routed and align > 16 -> the heap with constant bounds; else the reserve). Each
  compare then reads a memory operand where it read an immediate: 0 added. Alternative: route by withholding the
  thread's heap cache word (the fast path already tests it), with the routing test inside cache creation (once per
  thread).

## Step 2: printf (probe `pfprobe.c`/`pfprobe.sh` in scratch; keke copy ~/work/internal-alloc-routing-2)
* CORRECTION to leg 3: the "silent wrong result" (rc = full length, nothing written, for %.500d and 12 positional
  arguments) was a PROBE ARTIFACT: gcc folds snprintf's known return value even with -fno-builtin. Rebuilt with
  `-fno-printf-return-value` (pfprobe2): libniji returns -1/ENOMEM for those rows under refusal (no wrong result);
  the reference gives -1/ENOMEM for 24 positional arguments (its one allocating row) the same way.
* Rows where the reference makes no call but libniji (head) does (calls): %f 1e308 (2), %Lf 1e300 (3), %Lf 1e4000
  (6), %.400e (3), %.10000f (8), %.500d (1), %.100000d (9), 12 positional (1). Under refusal they fail with ENOMEM
  where the reference succeeds. Every other row (plain conversions, widths, stars, %ls/%lc, 64 sequential, %'d,
  truncation, snprintf(NULL,0), swprintf rows): 0 calls on both, identical text. 24 positional: 1 call on both.
* NOT DONE (needs its own run): design for the remaining rows: (1) integer precision zeros counted, not built in
  the Field: emit prefix, then the zeros in chunks from a static block, then the digits (%.500d, %.100000d);
  (2) long float text in a cold path: a counting pass for the length, then a second pass that writes straight to
  the sink (no Text/Field spill) for %f/%e/%Lf beyond INLINE_TEXT; (3) positional pieces: measure INLINE_PIECES
  16 -> 32 (the inline array is initialised per call: count printf_short first) or route only beyond the
  reference's size. Cheaper alternative (RULING NEEDED): printf's spill blocks private (over-aligned, as D-X.156's
  > 16-byte exception) - needs a safe over-aligned container in core (core has no unsafe).
* Not probed: a stream's plan cache (`make_plans`, boundary printf.rs, after `PlanSlot::AFTER` calls) allocates.
* Test added: x_internal_alloc runs snprintf over common conversions (14 pieces, inside INLINE_PIECES) with every
  allocation failing: success, full text, no call (format through a volatile pointer: no folding).

## Jobs (keke, role n3; head 6beb1da01)
See the report json. internal-alloc-routing-2-gate4 (clippy, static, direct affected incl. x_internal_alloc = the
in-tree `mint`), internal-alloc-routing-2-plants4 (6 plants, handoff/INTERNAL-ALLOC-ROUTING-2-plants.txt),
internal-alloc-routing-2-startup4 (`cargo xtask test`, NIJI_FULL_PROFILE so + hot_text_products = DYN_START_INSTR).
Earlier: internal-alloc-routing-2-basebench, -benchb, -builda (leg 3, start-up table above).

## INFORMATIONAL
* swprintf(buf, 256, L"%100000d"): errno 7 (E2BIG) on the reference, 75 (EOVERFLOW) on libniji; base 279a24ee4 the
  same: pre-existing, not this branch.
* The STARTUP-REPROFILE kit (empty main, three libraries, clock_gettime; both classes) was not run in this run;
  cbench_startup rows are above.

## Next step
Owner/coordinator rulings on the +2 per allocation (accept net, or the size-bound design with more process-flag
words) and on splitting printf step 2 into its own story; then a run that does the chosen step-1 change and the
printf design above (tests: extend the x_internal_alloc printf block with the eight rows; plants: per row).
Scratch: remove the detached base worktree (`git worktree remove ~/w/.scratch/INTERNAL-ALLOC-ROUTING-2/base`).
