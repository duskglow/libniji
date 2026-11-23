# LD-PHDR-ONCE: decode each object's program headers once

**State (LD-PHDR-ONCE-2, 2026-11-22):** branch `task/ld-phdr-once-2` (LD-PHDR-ONCE's commits, then origin/next
5f07429d3 merged: 67c3ff429; bounds, record and plants list: 93231ffa0; this note and the report on top). Role keke
`~/w/n3` (Zen 1 host). Briefs: ~/state/briefs/LD-PHDR-ONCE.md ("## The change"), ~/state/briefs/LD-PHDR-ONCE-2.md.
Decision: D-27.81 (records/decisions/D-27/D-27.81.toml).

## What changed (LD-PHDR-ONCE, unchanged by the merge)
`Elf::table()` takes the program header table slice once per walk (`rd.b[e_phoff .. + e_phnum*56]`, checked when
the reader is made; image mode: `rd.b` is the table, `e_phoff` 0) and `ProgramHeader::decode` reads each 56-byte
entry at fixed offsets. `program_headers`, `read_phdr` (`program_header`) and `check_table` use it. No check moved
or dropped. Not stored in `Elf`: `Object` has a static size assert (0x128, the loader's start-up stack).
Test `elf_reader::program_header_fields_read_their_own_offsets` checks all eight fields at distinct values.

## Merge (this run)
The only conflict: `DYN_START_INSTR` in crates/core/tests/hot_text_products.rs. Resolved to next's bounds, then
re-set from this branch's merged level (bounds only tighten; every merged row is under next's bound).

## Numbers, merged head (release products, STARTUP-REPROFILE kit `meas.sh`, median of 101, ASLR off)
Kit: handoff/STARTUP-REPROFILE/kit/ on task/startup-reprofile (identical to ~/work/startup-reprofile on both hosts).
Products: next 5f07429d3 built on the Zen 1 role (job ld-phdr-once-2-base-count), the merged head 67c3ff429 (job
ld-phdr-once-2-levels; same loader, libc and products code as the final head); the same files counted on the
Skylake-SP host (mei jobs ld-phdr-once-2-base-sky, ld-phdr-once-2-head-sky).

| row (user instructions, exec to exit) | Zen 1 next | Zen 1 head | Skylake-SP next | Skylake-SP head | host glibc (Zen 1 / SKX) |
|---|---|---|---|---|---|
| n0 empty main | 331,733 | **272,134** (-59,599, -18.0%) | 331,768 | **272,251** (-59,517) | 96,040 / 101,855 |
| n3 three one-function libraries | 492,864 | **381,399** (-111,465) | 492,854 | **381,501** (-111,353) | 132,752 / 138,635 |
| t /bin/true | 393,199 | 332,509 (-60,690) | 393,245 | 332,635 | 122,349 |
| n200 200 GLOB_DAT imports | 620,843 | 558,043 (-62,800) | 623,789 | 558,156 | 230,916 |

Per extra object (n3-n0)/3: 53,710 -> 36,422 (Zen 1). Page faults (63 / 75) and system calls (28 / 58) unchanged
on the Zen 1 host. STARTUP-REPROFILE expected -40k..-48k on the empty main: measured -59.6k.
Alone (LD-PHDR-ONCE, before the merge, fast products, Zen 1): empty main 705,569 -> 634,146, 3 libraries
956,034 -> 819,961.

hot_text_products' own count (Zen 1, merged head): fast 292,531, release 272,311 -> DYN_START_INSTR Zen 1
[298_400, 277_800] (level + 2%), Skylake-SP [298_600, 277_900] (Zen 1 level + the kit's +117 cross-host offset, + 2%;
the test itself was not run on the Skylake-SP host).

Intel PT, Skylake-SP, n0 (kit `pt` mode, jobs ld-phdr-once-2-base-pt and ld-phdr-once-2-head-sky): `read_phdr`
exclusive 47,847 -> 0 (inlined into its callers); `image_span_kind` 12,882 -> 11,826, `check_table` 1,776 -> 2,132,
`link` 12,138 -> 10,953, `protect_relro` 3,276 -> 653, `image_view` 4,936 -> 1,306. The brief's
"`read_phdr`-class work <= 10k" is NOT VERIFIED as such: with the reads inlined the walks are not separable by
symbol; the 13 functions holding them went 95,700 -> 35,942 exclusive (that includes their non-walk work).

Layout (§9): libc.so.6 of next and the head (release): all 6,340 defined symbols at the same addresses
(nm, `.llvm.<hash>` stripped). ld.so `.text` 369,639 -> 363,751 bytes, `.rodata` 11,004 -> 10,884.

## Jobs (keke unless named) and results, head 93231ffa0
- clippy (`cargo xtask clippy`) and `cargo xtask static` clean on the inherited head e0e732881 (step 0) and on
  93231ffa0.
- ld-phdr-once-2-gate: direct tier, `affected: ok`, 11 suites, 932 passed / 0 failed, peak 1.9G, rc 0 (includes
  hot_text_products on fast products with the new bounds).
- ld-phdr-once-2-plants: 8 plants, 8 CAUGHT, no WARN (handoff/LD-PHDR-ONCE-2-plants.txt; phdr-table-at's msg now
  names the products build's `BadAlign`, which was LD-PHDR-ONCE's WARN-NO-MSG).
- ld-phdr-once-2-startup: `cargo xtask test`, release products, release `hot_text_products`: rc 0 (cargo xtask test: 7,077 + 4,870 + 128 + 46 passed, 0 failed; release hot_text_products: 33 passed, with the new release bounds).
- Counts: ld-phdr-once-2-base-count, ld-phdr-once-2-levels (keke); ld-phdr-once-2-base-sky, ld-phdr-once-2-head-sky,
  ld-phdr-once-2-base-pt (mei). The levels ran on 67c3ff429: same code as the head (93231ffa0 changes the bounds,
  the record and the plants list only).

## Exact next step
Land (the CI train runs the FULL gate).

## Not to redo
Storing a decoded array in `Elf` (size assert); `as_chunks` (MSRV 1.85); perf sampling on keke for the 10k bound
(throttled; Intel PT on the Skylake-SP host is the tool, above).
