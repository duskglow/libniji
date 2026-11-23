# LD-RELR-LOOP: RELR applied a run at a time, words added in place; libc.so.6 packed

**State (LD-RELR-LOOP-4):** branch `task/ld-relr-loop-4` = LD-RELR-LOOP-3's head (1a5ef34d1) with origin/next
3e668ecce merged in (conflict only in `hot_text_products.rs`' `DYN_START_INSTR`: both records kept, the bound
re-set from the merged level). Role mei `~/w/s4`. Decisions in force: D-27.87 (RELR), D-27.85 (LD-OBJECT-ONCE,
landed), ruling #7402, ruling #7740 (the train-932 loader failures are not RELR's; LD-TABLES-LAYOUT is separate).
Full report: handoff/LD-RELR-LOOP-report.md ("LD-RELR-LOOP-4"); plants: handoff/LD-RELR-LOOP-4-plants.txt.

## The change (unchanged since LD-RELR-LOOP-2)
* `elf.rs` `Relrs::runs`; `reloc.rs` `pass1` -> `relr_entry` / cold `relr_checked`; `Image::add_bits` (default
  `add_bits_by_word`, niji-boundary `ProcImage` adds in place); `boot.rs` `Writer::add_bits` -> `Memory::add_bits`
  (`bootstrap::Mapped` adds in place); `xtask/src/so.rs` links libc.so.6 with `-z pack-relative-relocs`.

## Measured on the merged head (Skylake-SP, release, STARTUP-REPROFILE kit, median of 101)
* empty main 159,047 (next 3e668ecce 189,818: -30,771; next fd973fc44, before LD-OBJECT-ONCE and RELR,
  268,959: -109,912). Three libraries 250,528 (-30,853 / -127,681), `/bin/true` 216,900 (-30,811 / -112,435),
  200 imports 448,709 (-30,971 / -106,159). glibc 2.39: 101,855 / 138,761 / 128,175 / 236,773.
* hot_text_products' own count: fast 170,176, release 159,226; bounds now Skylake-SP [173,600, 162,500], Zen 1
  [173,500, 162,300] (Zen 1 derived as Skylake-SP - 117, not measured).

## Do not redo
* The train-932 loader failures (ruling #7740: LD-TABLES-LAYOUT's, not this branch's).
* The earlier legs' plants: all 8 re-run on the merged code this leg.

## Next step
Land (CI train). If the train conflicts on `DYN_START_INSTR` again: merge next, keep both records, re-measure
with the kit (handoff/STARTUP-REPROFILE/kit, `meas.sh`) and this test on the merged head, set level + 2%.
Remaining cost after RELR (estimate in the report's first section): ld.so self-relocation ~3.5k, libc RELR ~10k.
