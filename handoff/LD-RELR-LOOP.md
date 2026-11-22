# LD-RELR-LOOP: RELR applied a run at a time, words added in place; libc.so.6 packed

**State (LD-RELR-LOOP-2):** branch `task/ld-relr-loop-2` (continues `task/ld-relr-loop`, base origin/next
fd973fc44; next has moved 44 commits since, `crates/ld/src/elf.rs` touched on both: the landing's merge). Code
commits: bdf1d6fa0, bdbdf3c85 (leg 1-2), 9a96824e1 and bb4b03995 (this run). Role keke `~/w/n1` (Zen 1). Kits:
keke `~/work/ld-relr-loop/` (meas.sh = STARTUP-REPROFILE's, relrstat.py) and `~/work/ld-relr-loop-2/` (build.sh,
products `new2`, counts `out-prev` / `out-new2`). Decision in force: D-27.87 (both runs), ruling #7402.

## The change
* `elf.rs` `Relrs::runs`: one `(base, bits)` call per table entry (test `relr_runs_match_the_iterator`).
* `reloc.rs` `pass1` -> `relr_entry`: entry's lowest and highest words inside the window -> `Image::add_bits`
  for the whole bitmap, else cold `relr_checked` (test `reloc_relr_entry_checks_its_whole_run`).
* `Image::add_bits` (new trait method): default `add_bits_by_word` (load/store, either byte order); niji-boundary
  `ProcImage` adds in place (unaligned read + write, native order): 6 instructions per word (was 8).
* `boot.rs` `Writer::add_bits` -> `Memory::add_bits` (new trait method): default = bounds-checked view loop;
  `bootstrap::Mapped` adds in place: 6 instructions per word (was 13). Test `relr_bitmap_across_segments`.
* `xtask/src/so.rs`: libc.so.6 linked with `-z pack-relative-relocs` (1,441 RELATIVE -> 63 RELR words).
* `hot_text_products` `DYN_START_INSTR` tightened again (Zen 1 fast 248,446, release 238,792, + 2%).

## Counts (Zen 1, release, STARTUP-REPROFILE kit, median of 101; user instructions)
| row | next fd973fc44 | leg 2 (bdbdf3c85) | this run (9a96824e1) | glibc 2.39 |
|---|---|---|---|---|
| empty main n0 | 268,837 | 243,688 | 238,655 | 96,040 |
| three libraries n3 | 378,132 | 352,952 | 347,868 | 132,771 |
| /bin/true t | 329,222 | 304,186 | 299,000 | 122,349 |
Total vs next: -30,182 / -30,264 / -30,222. Faults 58/70/58 -> 57/69/57; syscalls unchanged. Target (RELATIVE +
self-relocation <= 15k): ~12k by estimate (libc RELR 1,441 x 6 + 63 entries x ~25; self 233 x 6 + 11 x ~40), NOT
measured: no Intel PT or valgrind on the Zen 1 host.

## Jobs (keke, this run) and open items
See handoff/LD-RELR-LOOP-report.md ("LD-RELR-LOOP-2") for the job list and results.
* Skylake-SP not measured (no product copy between hosts: refused in leg 2; this run did not build there).
* `layout-bins.tsv`: not diffed against base (no base copy; the change is ld.so code + libc's relocation sections).

## Next step
Land. A follow-up, if start-up wants more: measure the RELR share exactly with PT on Skylake-SP (build there),
and consider the per-entry overhead (~25 instructions x 63 entries) only if a profile ranks it.
