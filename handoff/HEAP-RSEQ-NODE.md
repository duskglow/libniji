# HEAP-RSEQ-NODE(-2, -3): the heap's node read takes rseq `node_id` (D-3.21)

State: branch `task/heap-rseq-node-3` (stacked on the HEAP-FIRST-USE chain, tip 8b24b6f94) = HEAP-RSEQ-NODE's
cf4dcc5ec (change, tests, D-3.21) + -2's start-up state declarations (02751a80e, fd71dfaba) + this run's
789767ac8 (whole-process first-localtime_r bounds, pending record) + 6946d3dea (plants file) + daad1a7dc (clippy
fix) + this note. Code head daad1a7dc. Role mei:~/w/s1 (Skylake-SP, two NUMA nodes).

## Decisions in force
D-3.12, D-3.20, D-3.21; D-X.53 (start-up bounds), amended by `records/pending/HEAP-RSEQ-NODE-3-1.toml` (D-NEW,
series D-X: CI assigns the id); D-X.62 / D-X.64 (`.data1` cluster, startup-state.list).

## What -3 did (owner ruling #5735 (b), extended to every host class, #5769)
The vDSO lookup stays lazy (no eager lookup, no `clock_gettime` in the trace probe). Instead the two first-localtime_r
bounds count the whole process: `page_faults.c` modes `localtime` and `localtime_mktime` print getrusage minflt at
the end of the call(s) (start-up included), not the delta. Bounds per product profile (read from the
`.hot-lists` marker's `target/product/<profile>`), equal to origin/next's whole-process count measured the same way:
next 5bbefc645 with only the probe change, mei, every run identical: fast 40 / 40, release 42 / 42
(localtime / localtime_mktime); this branch reads the same (jobs heap-rseq-node-3-next-count, -head-count). A count
more than 4 under its bound fails too (`WHOLE_COUNT_SLACK`): catches a probe that stops counting start-up (call-only
reads about 20) and asks for a tighter bound after an improvement. Every other bound unchanged. HEAP-FIRST-USE-8 has
not landed on next yet (no merge done).

## Jobs (mei, code head daad1a7dc)
heap-rseq-node-3-lint0 (inherited head fd71dfaba..62fb5aa40: clippy + static green), heap-rseq-node-3-lint2 (clippy,
static, records check green), heap-rseq-node-3-gate (`cargo xtask so && cargo xtask affected --run --base
origin/next`), heap-rseq-node-3-xtest (`cargo xtask test`), heap-rseq-node-3-startup-rel (release so +
hot_text_products), heap-rseq-node-3-plants (handoff/HEAP-RSEQ-NODE-3-plants.txt). All rc 0: gate 32 suites ok,
11800 passed, 0 failed, conformance 137/0, peak 16G; xtask test rc 0 (products step green); release
hot_text_products 26 passed (whole process 42 / 42, bound 42); plants 2 CAUGHT with msg, 0 WARN. SUPERSEDED: heap-rseq-node-3-lint1 (clippy: unchecked addition, fixed in
daad1a7dc); measurement jobs -next-count / -head-count are evidence (their tests fail by design on the old bounds).

## Open
- Counts measured on the Skylake-SP two-node class only; Zen 1, Haswell-EP, Broadwell-EP not measured (the floor's
  slack of 4 covers a one-fault spread; an upper-bound red on another class would need next's count there).
- -2's percall (thread_first_malloc -21 instr, heap-rseq-node-2-percall2) was on fd71dfaba; product code unchanged since.

## Next step
If any job above is red: fix and rerun only that job. Otherwise land via CI (the record's id is assigned there).

## Not to redo
The layout check of `Rseq` (offset 20), the ownership analysis, -2's three rseq plants (CAUGHT on mei), the count,
next's whole-process measurement (scripts mei ~/work/HEAP-RSEQ-NODE-3/measure_next.sh, measure_head.sh).
crates/core is package `liella`. Plan rows: D-3.12, D-3.20, D-3.21, D-X.53, D-X.62, D-X.64, #4198.
