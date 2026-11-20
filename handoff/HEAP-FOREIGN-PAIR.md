# HEAP-FOREIGN-PAIR: a foreign thread's malloc/free pair 395 -> 348 (step 1); step 2 measured, not built (ruling)

State: branch `task/heap-foreign-pair-2` (run 2, on `task/heap-foreign-pair` e8b1bbb89, base origin/next c1413a3c4);
code head c6d225666, this note and the report after it (handoff/ only). Role mei ~/w/s2.
Decisions in force: D-X.60, D-X.58, D-X.134, D-X.139. Rulings: #6958 (step 2: build PT_TLS only at zero start-up
cost), #6959 (tighten PAIR_BOUND only from the gate's own printed count).

## Run 2: the gate4 red, cause and fix (c6d225666)
`foreign_plain_route_zeroes_calloc_and_yields_to_perturb` failed its reuse assertion in gate4 (slot 1 returned
instead of the freed slot 0). Cause: the gate's runner, `cargo xtask nextest`, sets `NIJI_TEST_MALLOC_PERTURB=165` in
every test process (FREE-POISON-TESTS); test builds take it as `M_PERTURB` on their first heap call, so debugging was
on, the plain route was never taken, and perturbation held the freed block back. Plain `cargo test` (the plants and
run 1's own runs) does not set it, so it passed there. Reproduced on mei with `cargo xtask nextest -p niji-boundary
--test heap_foreign_plain` on the old test (deterministic). Fix: the test calls `mallopt(M_PERTURB, 0)` first (an
explicit setting clears the environment step too, `Debug::set_perturb`), so its checks run on the route under any
runner. The plants now run under `cargo xtask nextest` (handoff/HEAP-FOREIGN-PAIR-2-plants.txt: 4 of 4 CAUGHT, no
WARN). No library code changed in run 2.

## Step 2 (ruling #6958): measured, NOT built; numbers for the coordinator's ruling
Probe (readelf, black box): the reference libc.so.6 has a PT_TLS (file 0x10, memory 0x88, align 8); libniji's
libc.so.6 has none (no .tdata/.tbss). libniji keeps per-thread state in its thread block at the thread pointer
(`tb`, e.g. the heap cache word at 0x90, one `%fs` load) in processes it started; foreign threads (D-22.10) reach
theirs through the registry (`thread::foreign_heap_cache_slot`, by thread pointer) or `foreign_words()` (gettid).
Prototype (scratch only, `~/work/heap-foreign-pair-2/` on mei and keke): (b) = this head's libc.so.6 with its PT_NOTE
header rewritten to an 8-byte PT_TLS (.tbss, the size the cache word needs); static (b) = the program linked with
one object holding an 8-byte `__thread` word. execcount, ASLR off, median of 101, two rounds identical, min = median,
pa/pb paths of equal length, products of c6d225666 (`cargo xtask so`, fast profile), user instructions / faults:

| program | Skylake-SP a | Skylake-SP b | Zen 1 a | Zen 1 b | delta (both) |
|---|---|---|---|---|---|
| dynamic empty main | 832,239 / 65 | 832,788 / 65 | 832,032 / 65 | 832,581 / 65 | +549 instr, +0 faults |
| dynamic 3 libs | 1,060,877 / 75 | 1,061,426 / 75 | 1,060,679 / 75 | 1,061,228 / 75 | +549, +0 |
| dynamic clock_gettime | 867,319 / 69 | 867,868 / 69 | 867,141 / 70 | 867,690 / 70 | +549, +0 |
| static empty main | 2,096 / 8 | 2,166 / 9 | 2,084 / 8 | 2,154 / 9 | +70, +1 fault |
| static clock_gettime | 7,497 / 12 | 7,567 / 13 | 7,514 / 13 | 7,584 / 14 | +70, +1 fault |

The dynamic prototype has no access site, so a real build adds at least one TPOFF64 relocation more: +549 is a lower
bound. Every process pays it; a libniji-started process never uses the word (its threads have `tb`).
Saving side (estimate, not measured on a TLS build): the registry probe is about 18% of the pair in run 1's profile
(two `foreign_heap_cache_slot` calls, ~58 instructions); one TLS load instead saves ~55, so 348 -> ~290, not 183. The
rest of the gap is the call chain (export -> malloc_general -> debug_alloc -> begin -> foreign_alloc).
How often foreign threads occur: only in the coexist flavour (libniji's exports in a process another C library's
loader started: libniji's own test processes, a program run under another ld.so). On a system whose C library is
libniji, its ld.so or static start-up starts every process: no foreign threads.
Recommendation: do not build the PT_TLS (start-up is paid by every process, the saving is ~55 per foreign pair in a
rare configuration). If the foreign pair still matters, the cheaper lever is the call chain, with no start-up cost.

## Step 3 (finding #6812): measured, no change
`strace -f -c` of interposed-suite test binaries (every thread foreign; `~/work/heap-foreign-pair-2/gettid.sh`):
s03_memory_clauses 2,300 gettid of 27,346 system calls (0.084 s under strace, 4.9% of its system-call time);
s04_strings_clauses 628 of 2,508 (5 ms). It does not show in the tests' kernel time; left as is.

## Counts (run 1, keke, Zen 1; user instructions per pair) - not redone
foreign malloc(64)/free release 395 -> 348 (inlined 325 failed start-up windows); runtime-made 122.2.
PAIR_BOUND (2100) not tightened: nextest hides a passing test's printed count, so the gate printed none (#6959).

## Jobs (run 2)
mei: `heap-foreign-pair-2-gate` (clippy, static, affected --run --base origin/next, `cargo xtask test`, full-profile
so, hot_text_products), `-plants`, `-startup`, `-gettid`, `-precheck`; keke: `heap-foreign-pair-2-startup-zen`.
Diagnostic: `-lint0` (inherited head, green), `-repro`/`-repro2` (cause found). keke scratch:
/dev/shm/claude/heap-foreign-pair-2.

## Next step
The coordinator's ruling on step 2 with the table above. Nothing else is open on this story; do not redo the
prototype or the counts.
