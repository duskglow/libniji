# LD-PHDR-ONCE: decode each object's program headers once

**State (leg 2, 2026-11-22):** branch `task/ld-phdr-once` from origin/next c9f54d512. Role keke `~/w/n3`
(Zen 1 host). Brief: ~/state/briefs/LD-PHDR-ONCE.md. Decision: D-27.81 (records/decisions/D-27/D-27.81.toml).
Commits: b709cab98 (code, `crates/ld/src/elf.rs`), 82423d536 (D-27.81, note), dd475a111 (DYN_START_INSTR
tightened), 177964875 (plants file: one test-name filter), b910b7baa (new field-offset test), a959f7b65 (record
list spacing); this note and the report on top.

## What changed
`Elf::table()` takes the program header table slice once per walk (`rd.b[e_phoff .. + e_phnum*56]`, checked when
the reader is made; image mode: `rd.b` is the table, `e_phoff` 0) and `ProgramHeader::decode` reads each 56-byte
entry at fixed offsets. `program_headers`, `read_phdr` (`program_header`) and `check_table` use it. No check moved
or dropped. Not stored in `Elf`: `Object` has a static size assert (0x128, symbols.rs: the loader's start-up
stack), and adding a 16-byte slice to `Elf` broke it. `as_chunks` is refused by the MSRV (1.85):
`chunks_exact` + `first_chunk::<56>`.

New test `elf_reader::program_header_fields_read_their_own_offsets`: the synthetic image repeats p_offset in
p_vaddr and p_paddr, so plant phdr-vaddr-offset (p_vaddr read from p_paddr's bytes) survived plants-1; the test
gives every p_paddr a distinct value and checks all eight fields, indexed read and walk, both byte orders.

## Numbers (Zen 1 host, fast products, exec to exit, ASLR off, median of 21; kit ~/work/ld-phdr-once/ on keke)
| | next c9f54d512 (job ld-phdr-once-base-count) | b709cab98 (job ld-phdr-once-t3) | host glibc 2.39 |
|---|---|---|---|
| empty dynamic main | 705,569 | **634,146** (-71,423) | 96,015 |
| 3 one-function libraries | 956,034 | **819,961** (-136,073) | 132,677 |
| marginal per object | 83,488 | **61,938** (-21,550) | 12,221 |

`hot_text_products` itself (job ld-phdr-once-levels, Zen 1): fast 634,044, release 602,883; DYN_START_INSTR is
now 646,800 / 615,000 on both rows (level + 2%; Skylake-SP not measured, same-bound precedent of the doc comment).

Brief's bounds: 3-library marginal -20k per object: met (-21.5k). `read_phdr`-class work <= 10k on the empty
main: NOT VERIFIED. No valgrind on keke; `perf record` is capped at 2,000 samples/s there (a few hundred samples
over 200 runs), so only a rough share: phdr-walk symbols 33 of 485 kit-ld.so samples on next, 18 of 458 on the
branch (`image_span_kind` now inlines its walk). Script: ~/work/ld-phdr-once/prof.sh TAG.

Layout (§9): ld.so has no traced layout; it shrank 512,560 -> 508,688 bytes. libc.so.6: every defined symbol at
the same address in next's and the branch's fast build (nm, `.llvm.<hash>` suffixes stripped); the 48-byte size
difference is those suffixes' lengths in the string table. No base layout-bins.tsv was kept, so this symbol
comparison replaces the layout-bins diff.

## Jobs (keke) and results, head a959f7b65 (code and tests; prose commits on top)
- ld-phdr-once-gate2: direct tier, `affected: ok`, 11 suites, 920 passed / 0 failed, peak 1.7G, rc 0.
- ld-phdr-once-plants2: 8 plants, 7 CAUGHT, 1 WARN-NO-MSG (phdr-table-at: the products build's own read of the
  mutant-linked ld.so fails `BadAlign(8)` before tests run); results in handoff/LD-PHDR-ONCE-plants.txt.
- ld-phdr-once-startup2: `cargo xtask test` + release `hot_text_products`, started 08:1x after the gate, left
  running (PENDING JOBS; about 80 min on this host).
- Counts: ld-phdr-once-base-count (next), ld-phdr-once-t3 (b709cab98), ld-phdr-once-levels (82423d536): same
  loader and libc code as the head.
- clippy (`--workspace --all-targets --all-features -D warnings`) clean on b910b7baa; `cargo xtask static`
  clean on a959f7b65.
- Superseded: ld-phdr-once-gate, ld-phdr-once-plants (stopped, rc 124, to rerun on the head with the new test;
  plants-1 found the phdr-vaddr-offset survivor), ld-phdr-once-startup (stopped, rc 124, to run the gate first).

## Exact next step
If ld-phdr-once-startup2 ends red: fix it and rerun only that suite. Otherwise land.

## Not to redo
Storing a decoded array in `Elf` (size assert); `as_chunks` (MSRV); perf sampling for the 10k bound (throttled).
