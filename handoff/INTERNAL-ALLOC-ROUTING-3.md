# INTERNAL-ALLOC-ROUTING-3 (handoff; end of run 1, leg 3)

State: branch `task/internal-alloc-routing-3` (base origin/next 279a24ee4, not merged). Code head 7d62b9016 =
option 1b (one routing word) on top of option 1 (01b3bdc9a) and the static probe GOT fix. Option 2 (DO NOT LAND)
is on `task/internal-alloc-routing-3-opt2` ab2286af6. Role keke:~/w/n1. printf is PRINTF-NO-HEAP's: left alone.
Rulings in force: #7583, #7707, #7708 (process-flag heap reads), #7724 (land option 1 only if 0 added per allocation
and release and start-up no worse than run 2's (a)), #7725.

## Counts (Zen 1 host, Release products as `cargo xtask bench --compare` builds them; hit path = small class,
thread cache hit, counted on the disassembly with ~/w/.scratch/INTERNAL-ALLOC-ROUTING-3/hitpath.py; base, (a) and
option 2 add `heap_cache_word`'s owned path, 4)

| | base 279a24ee4 | run 2 (a) | option 1 01b3bdc9a | **option 1b 7d62b9016** | option 2 ab2286af6 |
|---|---|---|---|---|---|
| per allocation (rust_alloc) | 54+4 = 58 | 56+4 = 60 | 42 | **42** | 57+4 = 61 |
| per release (rust_dealloc) | 80+4 = 84 | 72+4 = 76 | 76 | **76** | 76 |
| startup_instr_c | 2105 | 2105 | 2108 | **2106** | 2105 |
| startup_instr_setloc | 62056 | 62089 | 62029 | **62028** | 62129 |
| startup_instr_wctype | 62547 | 62567 | 62507 | **62519** | 62608 |
| startup_instr_printf | 65627 | 65631 | 65596 | **65612** | 65762 |

Jobs: o1bbench (1b), o2bench3 (option 2), o1bench (option 1, leg 2). rust_dealloc's body is byte-identical (addresses
aside) in (a), option 1, 1b and option 2.
* Option 1b: rust_alloc is frameless, `cmpq $0xfff8,RESERVE_LAST(%rip); jne gated` then the size class; the
  start-up open is one `lock decq RESERVE_LAST(%rip)` in each `runtime::start` instance (the c row's +1). routed()
  in rust_alloc_gated is `cmpq $0x10000,RESERVE_LAST`.
* Option 2: worse than (a) per allocation (+3 vs base: the size compares against SIZE_ENDS words, the frame and
  the `heap_cache_word` call stay) and up to +135 at start-up. Not for landing.

## Verdict: RULING NEEDED (#7724's start-up condition)
Option 1b: -16 per allocation and -8 per release against base, other start rows -15..-28 against base and
-19..-61 against (a), but startup_instr_c is +1 against base and (a) (the one instruction that opens the fast
path; zero is not reachable with a word written only at start-up: the routing decision (`early_init`) comes
before `init_initial`, so the open depends on data). The other way to zero, a routed bit in the malloc DEBUG word
(cleared by `clear_foreign`, which init already runs), would send the C `malloc` entries onto their general path
in every routed process (forwarding LD_PRELOAD wrappers included): a behaviour change that needs its own ruling.
Recommendation: accept +1 instruction once per process for -16 per allocation and -8 per release, and land 1b.

## Not done / gaps
* No plant on RESERVE_OPEN's value: a value that keeps the fast path closed is perf-only (the counts show it), and
  one that cuts the reserve's last 8-byte slot needs an exhausted heap (no test reaches it).
* `lock decq` costs a locked RMW once per process; `RESERVE_LAST.store(reserve_last() - 1)` would be an unlocked
  `decq` (same instruction count). Not measured; kept the safe atomic.

## Exact next step
If the ruling accepts 1b: nothing (jobs below prove the head). Otherwise: drop 7d62b9016 and 01b3bdc9a's gate in a
new run, per the ruling.

## Jobs (keke, role n1, head 7d62b9016)
* internal-alloc-routing-3-gate4: green (clippy, static, direct tier: test -p niji / niji-boundary / native /
  exports interposed, 51 conformance programs incl. x_internal_alloc = the in-tree `mint`; `affected: ok`), peak 12G.
* internal-alloc-routing-3-plants2: 6 plants, 6 CAUGHT, 0 WARN (handoff/INTERNAL-ALLOC-ROUTING-3-plants.txt).
* internal-alloc-routing-3-o1bbench: Release cbench_startup + disassembly (table above).
* internal-alloc-routing-3-startup3: `cargo xtask test`, NIJI_FULL_PROFILE so + hot_text_products (started 02:38;
  re-attach `~/bin/jobwait internal-alloc-routing-3-startup3` on keke).
* Evidence on another head: internal-alloc-routing-3-o2bench3 (option 2, ab2286af6).
* Superseded: -gate3 (`affected` with no mode: usage, exit 2), -plants (head 01b3bdc9a), -o1bench (option 1),
  -so1, -so2, -opt2bench, -o2bench2 (leg 1-2 shapes).

## Notes
* PROFILE TRAP: compare Release dumps only (`target/niji/.profile` = release after `bench --compare`); a plain
  `cargo xtask so` is the fast profile (thin LTO) and not comparable.
* `cargo xtask affected` now needs `--list` or `--run` (no mode: usage, exit 2): gate3 died on that.
* Scratch: ~/w/.scratch/INTERNAL-ALLOC-ROUTING-3/ (opt2 worktree, hitpath.py, dumps).
