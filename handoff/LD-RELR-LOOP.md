# LD-RELR-LOOP: RELR applied a run at a time; libc.so.6 packed

**State (leg 2):** branch `task/ld-relr-loop` from origin/next fd973fc44. Commits: bdf1d6fa0 (code), bdbdf3c85 (start-up
bounds tightened, D-27.87, plants file), then handoff/report commits. Role keke `~/w/n1` (Zen 1). Kit: keke
`~/work/ld-relr-loop/` (build.sh, meas.sh = STARTUP-REPROFILE's kit; prof.sh = sampled perf, unusable: throttled;
relrstat.py = RELR table decoder). Decision in force: D-27.87 (this story), ruling #7402 (RELR for libc.so.6).

## The change
* `elf.rs` `Relrs::runs`: one `(base, bits)` call per table entry, same addresses/order/errors as the iterator,
  overflow cold (`relr_overflow`). Test `relr_runs_match_the_iterator`.
* `reloc.rs` `pass1` -> `relr_entry` (inline): entry's lowest and highest words inside the window -> add bias to
  each set bit's word (8 instructions per word, disassembly), else cold `relr_checked` (old semantics).
  Test `reloc_relr_entry_checks_its_whole_run`.
* `boot.rs` `Writer::add_bits`: lowest and highest words in one writable segment -> one view (13 instructions per
  word); else cold `add_bits_checked`. Test `relr_bitmap_across_segments`.
* `xtask/src/so.rs`: libc.so.6 linked with `-z pack-relative-relocs`. 1,441 RELATIVE -> 63 RELR words (7 address,
  56 bitmap); `.rela.dyn` 35,808 -> 1,224 B (+504 B `.relr.dyn`); no new version need (`readelf -V`: none).
* `hot_text_products` `DYN_START_INSTR` tightened (Zen 1 fast 255,972, release 243,825 + 2%; Skylake-SP +117 +2%).

## Counts (Zen 1, release, STARTUP-REPROFILE kit, median of 101; jobs ld-relr-loop-meas-base / -meas-new)
| row | base fd973fc44 | this branch | delta |
|---|---|---|---|
| empty main n0 | 268,837 | 243,688 | -25,149 |
| three libraries n3 | 378,132 | 352,952 | -25,180 |
| /bin/true t | 329,222 | 304,186 | -25,036 |
| faults n0 / n3 / t | 58 / 70 / 58 | 57 / 69 / 57 | -1 each |
glibc 2.39: 96,040 / 132,771 / 122,349. Syscalls unchanged. **Target (RELATIVE + self-relocation <= 15k) not
reached, by estimate (NOT measured; no PT on Zen 1):** libc RELR ~13.4k + self ~3.5k = ~17k. The brief's -50k
expectation counted pass1 parts that remain (STARTUP-REPROFILE's 47.6k phase included the table walk).

## Not done / open
* Skylake-SP counts and a PT phase profile: copying the products to mei was REFUSED (command in the report).
* `layout-bins.tsv` diff against base: no base copy on the host; the file lists libc's hot group only (this
  change is ld.so code and libc's relocation sections); the gate's layout tests passed.

## Next step (a follow-up story)
A word-add method on `reloc::Image` (no null-address branch per word; niji-boundary `ProcImage`) and a
self-relocation loop without the per-word slice check: ~3-4k more, to reach <= 15k. Then measure with PT on
Skylake-SP.

Full report: handoff/LD-RELR-LOOP-report.md; plants: handoff/LD-RELR-LOOP-plants.txt.
