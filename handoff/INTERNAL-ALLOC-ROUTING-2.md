# INTERNAL-ALLOC-ROUTING-2 (handoff; end of leg 3)

State: branch `task/internal-alloc-routing-2`, stacked on INTERNAL-ALLOC-ROUTING (code head fbbb0d309, base
origin/next 279a24ee4, not merged). Role keke:~/w/n3. Code head 141c3095e = variant (a) (plain reads, #7708).
Leg 3's job `internal-alloc-routing-2-builda` (on 141c3095e: clippy, static, `cargo xtask so`, bench
cbench_startup, disassembly to `~/work/internal-alloc-routing-2/dis-a.txt` on keke) ended rc 0 (results below).
Leg 4 is the run's LAST leg (no refresh left): hand back at its end. Printf (step 2) is the bulk; if it cannot land
in leg 4 with tests, plants and gate, hand back INCOMPLETE with step 1 proven and the printf design below.

## Rulings in force
#7583, #7696, #7697, #7698; #7707 (libc-side test once in early_init, zero added per allocation, exact per-start
cost in the hand-back's first line, DYN_START_INSTR not loosened; printf step 2 STAYS in this story, size M: use
`ops extend-budget` when needed). #7708 (OWNER, option (a)): plain reads of the two heap words under `process-flag`.

## Done (do not redo)
* Legs 1-2: design (see commit 1b9ba980e body): `Heap::alloc_below`, `FAST_ALIGN_END`/`RESERVE_LAST`,
  `route_internal()`, `release_cold`, `family_replaced` = one C call `niji_link_alloc_family`.
* Leg 3 (141c3095e): heap.rs reads both words through `fast_align_end()` / `reserve_last()`, each
  `// UNSAFE(process-flag)` + SAFETY; xtask/src/policy.rs `PROCESS_FLAG_READS` = 3 (file, body) pairs (thread.rs
  OWNED_HERE; heap.rs FAST_ALIGN_END, RESERVE_LAST), `PROCESS_FLAG_FILES`; docs/architecture.md row updated.
* Base tree for comparisons: keke `/dev/shm/claude/internal-alloc-routing-2/base` (279a24ee4, built, products in
  its target/niji; local worktree `~/w/.scratch/INTERNAL-ALLOC-ROUTING-2/base`, detached; remove it at the end with
  `git worktree remove`).

## Start-up, user instructions (`cargo xtask bench --compare --counters cbench_startup`, keke, Zen 1 host class)
| row | base 279a24ee4 | head (b) atomics 134ceabd4 | head (a) 141c3095e |
|---|---|---|---|
| startup_instr_c | 2105 | 2105 (0) | 2105 (0) |
| startup_instr_setloc | 62056 | 62094 (+38) | 62089 (+33) |
| startup_instr_wctype | 62547 | 62551 (+4) | 62567 (+20) |
| startup_instr_printf | 65627 | 65670 (+43) | 65631 (+4) |
Jobs: internal-alloc-routing-2-basebench (base so + bench), internal-alloc-routing-2-benchb (head (b)). The
predecessor's +137..+187 were on mei; these are keke's own base. (b) still pays per allocation (setloc/printf
allocate more at start than wctype). (a) (job internal-alloc-routing-2-builda: clippy, static, so all rc 0) moves the
rows unevenly against (b) (wctype +4 -> +20, printf +43 -> +4): the deltas are not a per-allocation cost alone;
suspect address/layout-dependent counts. Isolate: per-start decision cost from the disassembly of early_init's
decision (count the instructions of decide_route/family_replaced on the ordinary path), and the STARTUP-REPROFILE
kit (empty main, three libraries, clock_gettime; both classes) and DYN_START_INSTR (hot_text_products) still to run.
Static function sizes (dis-base.txt / dis-b.txt in ~/work/internal-alloc-routing-2):
rust_alloc 172 -> 149 (b) -> 148 (a), rust_dealloc 226 -> 180 -> 180, __rust_realloc 256 -> 255 -> 254 instructions; compare the ORDINARY PATH
(fast path) instruction by instruction on base vs (a), not totals.

## printf probe (step 2), done on (b) products (printf code is the same in (a))
Probe: `~/w/.scratch/INTERNAL-ALLOC-ROUTING-2/pfprobe.c` + `pfprobe.sh` (keke copy in ~/work/internal-alloc-routing-2;
`sh pfprobe.sh ~/w/n3`: gcc dynamic, host reference vs `target/niji/ld-linux-x86-64.so.2 --library-path`), modes c
(count) and r (refuse). Rows: %d %s %x %f %e %g %a %Lf, star width/precision, %ls %lc %.3ls, widths/precisions
1e5, positional 2/12/24 and with stars, 64 sequential, %'d, truncation, snprintf(NULL,0), swprintf rows.
* Reference: 0 calls in every row except 24 positional arguments (1 call); refusing: every row succeeds.
* libniji: calls in f 1e308 (2), Lf 1e300 (3), Lf 1e4000 (6), %.400e (3), %.10000f (8), %.500d (1), %.100000d (9),
  12 positional (1), 24 positional (4). Refusing: ENOMEM for the float rows and %.100000d; and a WRONG RESULT
  for %.500d and 12 positional args: rc = full length but nothing written (a defect this branch exposes; fix it).
* Every other row: 0 calls on both, identical results. Not probed yet: a stream (fprintf to /dev/null repeated:
  the per-stream plan cache `make_plans`, boundary printf.rs ~l.725, allocates after `PlanSlot::AFTER` calls).
* INFORMATIONAL (not allocation-related, both modes): swprintf(buf, 256, L"%100000d") gives errno 7 on the
  reference, 75 on libniji; base 279a24ee4 gives 75 as well (pre-existing, not this branch): report, do not fix.
* Base libniji (not routed): 0 calls, every row succeeds with the reference's text (pfbase-c/r.txt on keke).

## Next steps (leg 4)
1. Read dis-a.txt against dis-base.txt: the ordinary path of rust_alloc,
   rust_dealloc, __rust_realloc must have the same instructions as base (a compare against a memory operand in
   place of an immediate is the expected shape). Per-start cost = the early_init decision (count it: c row).
   Rerun mint and x_internal_alloc (`cargo xtask affected --only conformance:x_internal_alloc --with-exports --run`).
2. printf: no allocation where the reference makes none. Recommended design: digit text for every double and
   long double in fixed stack/inline storage (bounded: exact decimal expansion lengths), precision zeros and width
   padding streamed to the sink in chunks instead of built in a Field, positional-argument table inline up to the
   reference's observed no-allocation size (>= 12) and routed beyond. Cheaper fallback if that runs over: printf's
   spill containers allocate over-aligned (> ALIGN), which D-X.156 keeps private in both directions by Layout (zero
   ordinary-path cost); that is a private exception, so RULING NEEDED if chosen. Fix the silent wrong-result path
   either way (find where the sink's ENOMEM is dropped for %.500d / positional).
3. Tests: a conformance/product test with the refusing replacement over the probe corpus (expected: success +
   reference text); a test that the ordinary path makes no routing test (hot-text/disassembly or counts).
   Plants (`cargo xtask plants`, docs/plants.md form): a per-allocation route test on the ordinary path; a heap
   table for a plain %d. The predecessor's `no-route` plant text no longer matches.
4. Start-up checks (§9): `cargo xtask test`, `NIJI_FULL_PROFILE=1 cargo xtask so && ... hot_text_products`;
   layout-bins diff vs base; direct gate; clippy+static; precheck; D-X.156 text; report json + report md.
