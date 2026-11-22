# LD-RELR-LOOP: RELR applied a run at a time; libc.so.6 packed

**State (leg 1 done):** branch `task/ld-relr-loop` from origin/next fd973fc44; code commit bdf1d6fa0 (pushed). Role keke
`~/w/n1` (Zen 1). Kit: keke `~/work/ld-relr-loop/` (build.sh TAG = release `NIJI_FULL_PROFILE=1 cargo xtask so` and
a copy under TAG; meas.sh PRODUCTS OUTDIR = STARTUP-REPROFILE's meas.sh with K and the tmp name changed). Local copy
of the kit: the scratch dir `kit/`.

## The change (commit bdf1d6fa0)
* `elf.rs`: `Relrs::runs(f(base, bits), err)`: one call per table entry (address entry = `(addr, 1)`, bitmap =
  63 bits), every named address representable (an overflowing entry: the fitting bits, then the iterator's error,
  cold `relr_overflow`). Same addresses, order and errors as the iterator (test `relr_runs_match_the_iterator`).
* `reloc.rs` `pass1`: RELR through `runs` + `relr_entry` (inline): lowest and highest words inside the
  `Loads::window` copy -> add bias to each set bit's word, no check; else cold `relr_checked` (word by word,
  `Loads::inside`, window follows), exactly the old semantics. Test `reloc_relr_entry_checks_its_whole_run`.
* `boot.rs` `relr`: `Writer::add_bits`: bitmap's lowest and highest words in one writable segment -> one view, set
  bits only; else cold `add_bits_checked` (the old bit loop). Test `relr_bitmap_across_segments`.
* `xtask/src/so.rs` `link_libc`: `-Wl,-z,pack-relative-relocs` (ruling #7402).
* Proved so far: clippy `-p niji-ld -p xtask --all-targets -D warnings` clean; `niji-ld --lib boot` 7 passed,
  `--test elf_tables` 18 passed (job keke ld-relr-loop-lint3, on the code before commit = same content).

## Baseline counts (keke Zen 1, release products of fd973fc44, job ld-relr-loop-meas-base, median of 101)
user instr / kernel / faults: empty main n0 268,837 / 773k / 58; three libraries n3 378,132 / 1,122k / 70;
`/bin/true` t 329,222 / 775k / 58. glibc 2.39: 96,040 / 132,771 / 122,349. libc.so.6: 1,441 RELATIVE, 55
symbolic, no RELR; `.rela.dyn` 0x8be0 = 35,808 B. ld.so `.relr.dyn` 0x58 (11 words), `.rela.dyn` empty.

## Next steps (exact)
1. `~/bin/jobwait ld-relr-loop-so-new` (release build of bdf1d6fa0, started 15:32Z; products to
   `~/work/ld-relr-loop/new`; its log tail prints .rela.dyn/.relr.dyn sizes). If `cargo xtask so` refuses the
   link (e.g. a check on libc.so.6's relocation shape or a GLIBC_ABI_DT_RELR version need), fix that check.
2. Counts: `jobrun ld-relr-loop-meas-new count -- bash ~/work/ld-relr-loop/meas.sh ~/work/ld-relr-loop/new
   ~/work/ld-relr-loop/out-new` (absolute paths). Compare n0/n3/t to the baseline above; target RELATIVE work +
   self-relocation <= 15k (expected about -50k empty main). Faults must not go up. Also `readelf -V` libc.so.6: no
   new version need. Skylake-SP counts optional (copy products to mei, no build there).
3. Then on keke n1, one at a time: direct gate (`cargo xtask affected --run`, kind gate, label
   ld-relr-loop-gate1), start-up checks (§9: `cargo xtask test`, then `NIJI_FULL_PROFILE=1 cargo xtask so &&
   NIJI_FULL_PROFILE=1 cargo test -p niji --test hot_text_products`, label ld-relr-loop-startup1; a tightened
   release bound in hot_text_products may be due if the empty main drops: ALWAYS FORWARD), plants (add rows to
   docs/plants.md format, file handoff/LD-RELR-LOOP-plants.txt: drop the `last <= window.1` test in relr_entry;
   drop `holds(self.last, last)` in add_bits; make relr_fitting always Ok; drop the link flag (counts only:
   equivalent or a test on libc.so.6 having DT_RELR)), label ld-relr-loop-plants1.
4. Decision record: `ops reserve --story LD-RELR-LOOP D-27` for a D-27 row (pattern: records/decisions/D-27/D-27.68.toml),
   numbers from step 2. Diff `target/product/fast/layout-bins.tsv` against base (§9).
5. Workspace clippy + `cargo xtask static`, precheck, report json, handback-check.

## Not to redo
Profile and design are done (STARTUP-REPROFILE phase table: RELATIVE + walk 47.6k, self-relocation 17.9k).
