# LD-PHDR-ONCE: decode each object's program headers once

**State (leg 1, 2026-11-22):** branch `task/ld-phdr-once` from origin/next c9f54d512; code commit b709cab98
(`crates/ld/src/elf.rs`), then this note and D-27.81. Role keke `~/w/n3` (Zen 1 host). Brief:
~/state/briefs/LD-PHDR-ONCE.md. Decision: D-27.81 (records/decisions/D-27/D-27.81.toml, id reserved).

## What changed
`Elf::table()` takes the program header table slice once per walk (`rd.b[e_phoff .. + e_phnum*56]`, checked when
the reader is made; image mode: `rd.b` is the table, `e_phoff` 0) and `ProgramHeader::decode` reads each 56-byte
entry at fixed offsets. `program_headers`, `read_phdr` (`program_header`) and `check_table` use it. No check moved
or dropped. Not stored in `Elf`: `Object` has a static size assert (0x128, symbols.rs: the loader's start-up
stack), and adding a 16-byte slice to `Elf` broke it. `as_chunks` is refused by the MSRV (1.85):
`chunks_exact` + `first_chunk::<56>`.

## Numbers (Zen 1 host, fast products, exec to exit, ASLR off, median of 21; kit ~/work/ld-phdr-once/ on keke)
| | next c9f54d512 (job ld-phdr-once-base-count) | b709cab98 (job ld-phdr-once-t3) | host glibc 2.39 |
|---|---|---|---|
| empty dynamic main | 705,569 | **634,146** (-71,423) | 96,015 |
| 3 one-function libraries | 956,034 | **819,961** (-136,073) | 132,677 |
| marginal per object | 83,488 | **61,938** (-21,550) | 12,221 |

Brief's bounds: 3-library marginal -20k per object: met (-21.5k). `read_phdr`-class work <= 10k on the empty
main: NOT VERIFIED (no valgrind on keke; by subtraction from the review's 82,779 callgrind figure on another
snapshot, ~11k would remain: an estimate). Kit: `bc.sh TAG` = `build.sh TAG` (cargo xtask so on n3, copies
products to p-TAG) + `count.sh TAG` (exec_instr of m0/m3 and the reference). Copies in the scratch dir.

## Exact next step
1. Read `~/logs/ld-phdr-once-levels.log` (keke): `dynamic empty main (fast|release, ...)` lines. Both will be more
   than 10% under `DYN_START_INSTR` (719,100 / 679,100, crates/core/tests/hot_text_products.rs:494): set both CPU
   rows to the measured levels plus 2% (the rule in the doc comment above; add an LD-PHDR-ONCE (D-27.81) sentence).
   NOTE: that job left n3's products in the release profile; rebuild fast (`cargo xtask so`) before fast tests.
2. Commit, sync, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo xtask static`
   (records check for D-27.81).
3. Detached: gate (`cargo xtask affected --run`, direct tier; if FULL escalated: precheck + `cargo test -p niji-ld`),
   plants (`cargo xtask plants handoff/LD-PHDR-ONCE-plants.txt --dir /dev/shm/claude/ld-phdr-once/plants-1`),
   start-up checks (`cargo xtask test`; `NIJI_FULL_PROFILE=1 cargo xtask so && NIJI_FULL_PROFILE=1 cargo test -p niji
   --test hot_text_products`). One product build per role at a time: run them in sequence.
4. Layout: ld.so has no traced layout (§9); diff `target/product/fast/layout-bins.tsv` vs base (libc unchanged).
5. Report json, precheck, handback-check. First line names the start-up gain (coordinator tells Yohane).

## Not to redo
Storing a decoded array in `Elf` (size assert); `as_chunks` (MSRV).
