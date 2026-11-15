# UTIL-LDCONFIG (UTIL-3, SL-LDCONFIG): ldconfig

**State.** Branch `task/util-ldconfig-3` (run 3; stacked on run 2's `task/util-ldconfig-2`, itself on run 1's
`task/util-ldconfig`), role mei `~/w/s1`. Head: see the report `handoff/reports/UTIL-LDCONFIG-3.json`. Engine,
front end, transcripts and unit tests are green; the shippable ldconfig program and its packaging are NOT done
(next steps), so the engine has no user yet outside `cargo xtask transcript`.

## Done (runs 1 to 3)

* Engine `crates/ld/src/ldconfig/{mod,conf,scan}.rs`; front end `xtask/src/transcript/ldconfig.rs` (host `Fs`
  over std: `-r` resolved inside ROOT, atomic link replacement, cache through `<cache>~` + fsync + rename).
* Cache writer (`crates/ld/src/cache/writer.rs`): a hwcaps entry's path may end in any file name; since run 3 a
  key that is not the tail of its path gets its own string (was stored as the path's tail: key `bdd.so.4.0`).
  Test `write_hwcaps_key_other_than_the_file_name` (cache_write.rs).
* Harness items `elf:` and `as-cache:`, documented in docs/transcripts.md (run 3).
* 12 transcripts in validation/transcripts/ldconfig/: **all 12 ok against the reference** (mei, job
  util-ldconfig-3-check1). `crates/ld/tests/ldconfig.rs`: **8 of 8 pass** (run 3 fixed the two test bugs: the
  soname test lists after a second run; the fuzz test compares two runs of the same tree, 3000 seeds).
* Pending decision `records/pending/UTIL-LDCONFIG-3-1.toml` (D-NEW, series D-31): atomic links, sorted listings,
  positioned reads, native class only, include cycles refused with a message, separate hwcaps key, `-l` refused
  (a gap). D-31.9 already holds determinism, the atomic cache and untrusted input.
* Plants `handoff/UTIL-LDCONFIG-3-plants.txt` (11: conf cycle/glob order/relative base, scan version direction,
  development-link rule, dangling removal x2, hwcaps no-link and file, writer key x2). Results in the report.

## Probe-settled (runs 1-2; probe files keke `~/work/UTIL-LDCONFIG-2/probe/`; do not redo)

Directories stat'ed and de-duplicated as named; `-N -X -v` still scans and prints; hwcaps subdirs scanned after
their parent, no links there, entry = soname -> real file; dangling `lib*.so*` links removed unless `-X` or name
ends in `.so` (`Cannot stat` only with `-v`); a fake `ld-linux-x86-64.so.2` with e_entry 0 draws no "is the
dynamic linker" message (rule unknown, open).

## Differential coverage (how the brief's "both loaders" proof stands)

* Liella's reader: every transcript with `as-cache:` decodes the reference's cache and Liella's with Liella's reader
  and they agree (r-conf-cache, incl. a hwcaps entry).
* Reference loader: `reference_loader_resolves_through_liella_cache` (cache_write.rs, namespace method) resolves
  through a cache made by the same writer the engine calls. NOT done: that test over a cache written end to end by
  the engine (a loader run over the r-conf-cache tree).

## Open questions

1. Native class only: other-class ELF files (i386, x32) are skipped, so on a multilib system their cache entries
   would be missing; plan/utilities.md §5 lists "foreign-class" files in the differential. Not probed (the `elf:`
   item makes only x86-64 ELF64). RULING NEEDED in the hand-back.
2. `-l` is refused ("not supported yet"): implement (needs probes of its messages) or keep refused with a
   differences row.
3. Include cycles: Liella's message line has no reference counterpart recorded; a differences row needs the
   reference's behaviour, which was not probed (and should be probed with care: a self-include).
4. The "is the dynamic linker" rule (probe with a nonzero e_entry / PT_INTERP).

## Next steps (run 4), in order

1. The shippable program: a freestanding static `ldconfig` binary (D-31.2) with an `Fs` over liella-boundary
   syscalls (`openat2(RESOLVE_IN_ROOT)` for `-r`, D-31.9), running the same transcripts with `--liella`.
2. Packaging (`tools/deb/system.sh`): ship it in libc-bin as `/sbin/ldconfig`; `tools/deb/system/libc-bin.postinst`
   `triggered)` runs `ldconfig` (with `-r "$DPKG_ROOT"` when set). The triggers files are already there.
3. The end-to-end loader differential above; an ELF32 `elf:` option and a foreign-class transcript (question 1).
4. plan/utilities.md §5 counters (instructions, syscalls) of a full run against the reference.

Read: this note, plan/utilities.md §2.3/§4/§5, spec §31.2 and D.3, docs/transcripts.md, D-31.2/D-31.9.
