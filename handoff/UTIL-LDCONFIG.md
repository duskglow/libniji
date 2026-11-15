# UTIL-LDCONFIG (UTIL-3, SL-LDCONFIG): ldconfig

**State.** Branch `task/util-ldconfig-4` (run 4; stacked on run 3's `task/util-ldconfig-3`), role mei `~/w/s1`.
Head: see `handoff/reports/UTIL-LDCONFIG-4.json`. Engine green (runs 1-3). Since run 4 the shippable program and its
packaging exist and pass the transcripts. NOT done: `-l`, the foreign-class probe, the end-to-end loader
differential, plan §5 counters.

## Done

* Runs 1-3: engine `crates/ld/src/ldconfig/{mod,conf,scan}.rs`, hosted front end `xtask/src/transcript/ldconfig.rs`,
  cache writer hwcaps key, 12 transcripts (all ok against the reference), `crates/ld/tests/ldconfig.rs` 8/8, plants
  `handoff/UTIL-LDCONFIG-3-plants.txt` 11/11, pending record `records/pending/UTIL-LDCONFIG-3-1.toml`.
* Run 4, the shipped program (`target/liella/ldconfig`, built by `cargo xtask so`, `run_ldconfig` in xtask/src/so.rs):
  * `crates/ldconfig-bin` (product workspace, staticlib) + boundary feature `ldconfig` (`crates/boundary/src/ldconfig.rs`:
    `liella_ldconfig_start`, `Arena` bump allocator, panic handler) + `csys/src/machine/entry.S` `-DLIELLA_LDCONFIG`
    entry `liella_machine_ldconfig_entry`. Static PIE, no PT_INTERP (`check_no_interp`), L14-1 (`check_interp`).
    Shares the `ld-pie` target directory with ld.so (same flags).
  * `crates/ld/src/ldconfig/sysfs.rs`: `SysFs<S: Sys>`; `-r` via `openat2(RESOLVE_IN_ROOT)`, parent-fd ops,
    one cached read fd. `RESOLVE_IN_ROOT` added to kernel uapi.rs. `ldconfig::GENERATOR` now shared with xtask.
  * **12/12 transcripts ok through the binary** against the reference (mei job util-ldconfig-4-tr2); new test
    `the_shipped_ldconfig_passes_the_transcripts` (xtask/tests/transcripts.rs; SKIP when the product is not built).
  * Packaging: libc-bin ships `/usr/sbin/ldconfig`; `libc-bin.postinst triggered)` runs it (`-r "$DPKG_ROOT"` when
    set); check-system.sh allows the path. The trigger files were already right. root-system.sh NOT re-run.
  * unsafe audit: `src/ldconfig.rs` added to policy.rs for export/allocator/slice (as ld.rs).
  * Pending record `records/pending/UTIL-LDCONFIG-4-1.toml` (D-NEW, D-31): the program's shape.
  * Plants `handoff/UTIL-LDCONFIG-4-plants.txt` (3, on sysfs.rs): results in the report.

## Probe-settled (runs 1-2; do not redo)

See run 3's note (git history of this file on this branch): directory dedup, `-N -X -v`, hwcaps, dangling links.

## Open questions

1. Foreign class (i386/x32 ELF skipped, native class only): not probed. RULING NEEDED stays.
2. `-l` still refused ("not supported yet"): implement (probe its messages) or differences row.
3. Include-cycle message has no reference counterpart recorded.
4. The arena keeps freed non-last blocks: measure peak RSS on a full system's /usr/lib before claiming "a few MB".

## Next steps (run 5), in order

1. Read the gate and plants results of run 4 (labels in the report); fix any red.
2. `tools/deb/root-system.sh` over a built system flavour: the trigger must now write `$r/etc/ld.so.cache`
   (add a pass line for it).
3. `-l` (#5916) with probes; the foreign-class probe (#5915) with an ELF32 `elf:` item.
4. The end-to-end loader differential over a cache the binary wrote; plan §5 counters (instructions, syscalls).

Read: this note, plan/utilities.md §2.3/§4/§5, spec §31.2 and D.3, docs/transcripts.md, D-31.2/D-31.9.
