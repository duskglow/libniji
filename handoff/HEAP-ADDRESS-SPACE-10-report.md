# HEAP-ADDRESS-SPACE-10 report (issue #21)

Branch `task/heap-address-space-10`; code head `88506b099` (later commits are handoff/ only). Build role keke `~/w/r1`
(Zen 1, 60 vCPU). Verdict: INCOMPLETE: the fast-profile WINDOW-BIN-BYTES contract is red (`tool 3/2`); the cause is
settled and is neither this run's fix nor the branch's planner change (RULING NEEDED below).

## Commits (run -10)

- `e76808c9a` Give ForeignPages this branch's map_room and grow (fuzz-check red after the merge with next).
- `39acf3bd3` Place aligned heap mappings past a gap with no aligned range (the RLIMIT_AS xtest red of run -9: a
  library defect, malloc failing with room left under `ulimit -v`).
- `88506b099` Make the gap test's layout the one the heap meets (deterministic test of that gap).
- `5abea6080`, `8f57c92ba`, this commit: plants record, handoff, report.

## Jobs (keke)

| label | what | rc |
|---|---|---|
| heap-address-space-10-v2 | xtask clippy, cargo clippy --all-features -D warnings, xtask static, both RLIMIT/gap tests 20/20 native + 20/20 interposed | 0 |
| heap-address-space-10-plants | `cargo xtask plants`: 6 of 6 CAUGHT with messages (HEAP-ADDRESS-SPACE-10-plants.txt) | 0 |
| heap-address-space-10-xtest | `cargo xtask test`: products step, `layout: 1 of 13 contracts broken: WINDOW-BIN-BYTES` | 1 |
| heap-address-space-10-fullhot | NIJI_FULL_PROFILE so + hot_text_products: 31 passed; release windows within budget | 0 |
| heap-address-space-10-cbhead / -cbnext | cbench_startup release counts, head / next | 0 / 0 |
| -sonext, -somerge, -somerge-oldbins | fast `cargo xtask so` of next, of the merge, of the merge with next's bins.rs (diagnostic) | 0 / 1 / 1 |
| -kcmp2, -kcmp-setloc, -kcmp-user | perf stat -r 300 and strace of the static cbench_startup children, next vs head (diagnostic) | 0 |
| -kcmp (superseded) | wrong invocation (ran the whole harness, not the child); stopped with kill-label | - |

## WINDOW-BIN-BYTES (fast profile) split

| tree | bins 0 / 1 / 2 bytes | tool windows (budget 2) |
|---|---|---|
| next `d4cbe1667` | 64300 / 58666 / 64897 | 2 (bins 0, 2) |
| merge `e76808c9a` (no gap fix) | 63980 / 58090 / 60451 | 3 |
| merge + next's `xtask/src/bins.rs` | 63980 / 58090 / 60451 | 3 |
| head (r1) | 63980 / 58090 / 60451 | 3 |

The `next` tree equals the merge base apart from the branch's own files (md5 of 1193 sources). Per-bin bytes by
signature: bin 0 is unchanged; in bin 2 the `locale,rebind,rebind.pre,tool` group grew 43125 -> 44158 B
(`thread::each_block<locale::publish_global>` 696 -> 2083 B, `global_locale::setlocale_env_kept` newly outlined,
1208 B; `.llvm.<hash>` suffixes changed throughout), so the `rebind,rebind.pre,tool` (1547 B) and `tool` (3256 B)
groups no longer fit bin 2 and open bin 1, the third window for `tool`. The heap's own hot functions net -23 B
(`first_cache` -50, `refill` +32, `take_held` -5). Fast = dev profile (niji-boundary opt-level 2, many codegen
units, no LTO): the branch's edits to `crates/boundary/src/heap.rs` re-partition that crate's codegen units and move
inlining in unrelated locale functions. hot-text.signatures: no traced mode changed; the differences are function
boundaries only (bodies now compiled into `setlocale_env_kept`, `intl::bind` / `textdomain`, `Conv::to_words`,
`locale::setlocale` instead of `catalog::bind` / `textdomain`, `iconv::codec`, `store_words`), plus the branch's
`grow_segment` with signature `-` (not run at start-up). The release product (fat LTO, one codegen unit), which
ships and which the counts measure: head = next on every mode (`tool 3/3`, all within budget), and the full profile's
hot_text_products passes.

## Start-up counts (cbench_startup, static release, head vs next)

LOAD count equal (5 / 5, two read-only each), so the pad method of ruling #7061 is not needed. Bench rows
(instr/op, libniji): startup_instr_c 2160 / 2160; setloc 62087 -> 62129; printf 65722 -> 65725; wctype 62586 ->
62571; startup_kinstr_c 146779 -> 165349. Direct check of the same binaries' children (perf stat -r 300, two
alternating passes, means):

| child | next | head | delta |
|---|---|---|---|
| true, user | 3029 / 3029 | 3029 / 3029 | 0 |
| true, kernel | 200699 / 183955 | 183942 / 181839 | noise (next alone moves 16.7k) |
| true, faults; system calls | 8; 11 lines | 8; 11 lines (strace equal but addresses/pids) | 0 |
| setlocale, user | 62253, 62256, 62256, 62255 | 62262, 62263, 62262, 62262 | +6..+9 |
| printf, user | 65870, 65870, 65869, 65869 | 65873, 65874, 65873, 65871 | +2..+4 |
| wctype, user | 62725, 62725 | 62729, 62730 | +4..+5 |

The kernel +18.6k of the bench job is noise; the user deltas are the heap's own first-reservation cost (run -9
measured +4..+6 on the same three children).

## RULING NEEDED

1. Fast-profile WINDOW-BIN-BYTES `tool 3/2` from dev codegen-unit drift of locale functions (above). Recommend:
   treat as layout jitter (as #7028 did for the static row; no heap text trimming to dodge it) and cut a tooling
   story that makes the fast window gate stable under codegen-unit re-partitioning (for example one codegen unit
   for the crates in the start-up group in the products build, with its build-time cost measured), or judge
   windows on the release product only.
2. The heap's start-up cost on malloc'ing children: setlocale +6..+9, printf +2..+4, wctype +4..+5 user
   instructions (0.01 %). Recommend: accept as the cost of issue #21's fix (malloc no longer fails early under an
   address-space limit); follow-up HEAP-SPILL-COLD to look for it.

## INFORMATIONAL

- origin/next moved to `305065b07` after the merged `d4cbe1667`; not merged in this run (the landing merges).
- `cargo xtask affected --list --base origin/next`: FULL: the train's (`xtask/src/bins.rs`, run -6's change).
