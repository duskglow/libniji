# HEAP-FIND-DEPTH-NEED: handoff (leg 4: HEAP-FIND-DEPTH-NEED-2)

State: branch `task/heap-find-depth-need-2` (continues `task/heap-find-depth-need`; base origin/next `3e668ecce`).
Code fix `adb8aae13` (EXACT_BLOCKERS 9); test commits `f2528a345` (page-exact, phase sweep, bound 33576) and
`aa2ca2ff7` (clippy); later commits are prose (D-3.25, handoff, plants file, report). Role mei `~/w/s1`
(Skylake-SP). Decision D-3.25 (`records/decisions/D-3/D-3.25.toml`). Report: handoff/reports/HEAP-FIND-DEPTH-NEED-2.json.

## Cause of the +2560/+4720 (unchanged, leg 1-3)

`map_aligned_exactly` (crates/boundary/src/heap.rs) bounded blockers per round at 8; a later segment (2304 KiB,
D-3.24) needs 9. Under RLIMIT_AS with find's locale mappings taking the aligned neighbours, no later segment was
placed and the heap mapped per request. Fixed with 9 (`adb8aae13`).

## Why CI went red (train 967) and the fix (this leg)

CI on mei: `find needs 15136 KiB for 3 entries, 48480 for 200000: 33344 over (bound 33296)`. Two measurement causes:

1. **16 KiB grid.** Each need was bisected to 16 KiB; the bound is a difference of two needs. The branch alone on
   mei read 15280 / 48592 = 33312 against keke's 15296 / 48592 = 33296 (d3 one grid step apart). Page-exact on mei:
   15276 / 48580 = 33304, 3 of 3 runs (`-meas4k`).
2. **Layout phase.** The heap places segments on 16 MiB boundaries; where the mappings before it end within 16 MiB
   decides which aligned slots are free. Shifting the mmap base page by page (stack limit above 128 MiB moves it
   by the excess under ADDR_NO_RANDOMIZE; diagnostic `-shift`, `-shift2`, `-dense`) gives, over one period:
   d3 always 15276; over-d3 in bands 33048 / 33304 / 33560-33576 (shifts 0-6 MiB low, 8 MiB and 14-15 MiB middle,
   ~10.2-13.5 MiB high; period 16 MiB confirmed at 16384/17408/26624). 145 shifts 32 KiB apart over
   [9856, 14464]: max 33576 (15 shifts). A change in any library's size moves the phase: the train's composition
   (with NIJI-GXX-UNWINDER-2) read 33344 on the 16 KiB grid, i.e. 33329..33360 exact, between bands; not
   investigated (no access to that branch's tree).
   strace of the high band (`-trace`, shift 10368 vs 0): the second segment's aligned slot (0x7ffff6000000) has
   room for ONE 2 MiB growth step before an existing mapping (`MAP_FIXED_NOREPLACE` EEXIST at +0x440000), so the
   walk places a third segment (more blocker rounds, another 2304 KiB start). At shift 0 the second segment grows
   to its full size.

Test now (`find_over_200000_entries_bounds_its_address_space`, crates/core/tests/heap_meta_products.rs): page-exact
bisection ([0, 128 MiB], 15 steps), 8 phases 2 MiB apart (`FIND_PHASES`, the high band is over 3 MiB wide),
bound on the worst: `FIND_200K_OVER_3 = 33576` (the dense sweep's max; still 1208 under 22.3's one-layout 34784).
On mei the test reads 33048 x4, 33304, 33572, 33560, 33304 (worst 33572 at 10240), 12.7 s. Skips (printed) without
/usr/bin/find, /dev/shm, the locale archive, or a stack hard limit allowing 128 MiB + 14 MiB.

## Results (mei, final code head aa2ca2ff7)

- `heap-find-depth-need-2-lint` (step 0, inherited head 9f3057a0a): clippy + static green.
- `heap-find-depth-need-2-lint3`: clippy -D warnings, static, the find test: green (33572 <= 33576).
- `heap-find-depth-need-2-plants` (plants-1): blockers-8 CAUGHT (s03 layout test); blockers-8-find failed the
  bound's assertion at every phase (mutant 34340..34852 over 3 entries) but WARN-NO-MSG (regex used `\d`).
  Superseded by `heap-find-depth-need-2-plants2` (regex `[0-9]+`): see the report json.
- `heap-find-depth-need-2-plants2`: 2 of 2 CAUGHT (blockers-8-find by the find bound at every phase).
- `heap-find-depth-need-2-gate` (`affected --run`, direct tier, on aa2ca2ff7): 10450 passed, 8 failed = 4 locale /
  reference tests in both passes (s7_2_f_0 accepts `zh_HK.big5hkscs`; locale_products 3 lines; printf corpus 4;
  catalogs reload). Host environment, not this branch: mei's /usr/lib/locale gained 511 compiled locale dirs on
  11-23 (06:32 / 18:10); `ctype-objects-setlocale-2-gate` (role s4, 18:44) failed the same 4; CI on mei passed
  them all evening, including the train with this code (ba4f67797). The only library change is EXACT_BLOCKERS.
  The find bound and s03 passed in it. `cargo xtask test` and the full-profile checks did not run in its `&&`
  chain: `heap-find-depth-need-2-startup` (static ok; `cargo xtask test` 7153 passed, the same 4 failed, nothing
  else) and `heap-find-depth-need-2-fullprof` (full-profile `so` + hot_text_products: 33 passed, rc 0).
- keke (Zen 1) NOT re-measured this leg (role is mei). Expected to pass: its 16 KiB-grid needs (d3 15296, d200k
  48592 at the default layout) put its exact over-d3 at <= 33312 at that phase, and its d3 is the larger one.

## Open / next

- INFORMATIONAL (heap, not this story): the high band costs up to 528 KiB of address space over the low band at the
  same workload. A later segment is placed where its 2304 KiB start fits, not where it can grow; choosing an
  aligned slot with growth room (or skipping a slot whose growth is blocked) would flatten the phase dependence
  and let the bound tighten to ~33048. Candidate story: HEAP-SEGMENT-GROWTH-ROOM.
- Next step: the coordinator's landing (FULL gate). If the find bound goes red on another host or composition,
  read the per-phase lines first: a reading between bands is a layout effect of the composition (see 2.).
- Diagnostic job scripts: mei `~/work/heap-find-depth-need-2/{shift,trace,meas}.sh`.
