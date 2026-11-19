# DYN-START-PARITY (handoff, leg 1 -> leg 2)

State: branch `task/dyn-start-parity`, HEAD = the commit "Cut the loader's per-relocation table walks"
(525af12ad) on origin/next 255a6901d. Role: mei `~/w/s1` (Skylake-SP). Scratch on the host:
`/mnt/zram/claude/DYN-START-PARITY/` (base-rel = origin/next release products; prog-* = test programs),
scripts in `~/work/DYN-START-PARITY/` (parity.sh, prof.sh).

## Measurements (execcount, ASLR off, median of 101, release products `NIJI_FULL_PROFILE=1 cargo xtask so`, Skylake-SP)

| build | empty main instr | 3 libs instr | clock_gettime instr | faults (pie0/pie3/time) |
|---|---|---|---|---|
| glibc 2.39 (host) | 106,501 | 132,267 | 108,600 | 49 / 62 / 50 |
| origin/next 255a6901d | 8,151,153 | 8,374,348 | 8,194,191 | 75 / 91 / 80 |
| 525af12ad (A+B) | 2,049,314 | 2,271,815 | 2,083,381 | 75 / 91 / 80 |

Measure: `sh ~/work/DYN-START-PARITY/parity.sh <products dir> <tag> 101` (add `GLIBC=1` for the reference
rows). Profile: perf record on the exec'd program only, e.g. the loop in prof.sh with `--comm pie0`.

## Profile before (origin/next, release, by symbol)
read_phdr 31% (26% from the per-relocation range check `reloc::inside`), Verdefs::next 18-20% + memmove
13-21% (version_name walks copying a reader view per entry), Dynamic::once 7.6% (finding DT_VERDEF/VERNEED
per lookup), GnuHash::symbol_count 3.5%, reloc::one/pass1 ~9%, image_span_kind 2.8%.
Phases: nearly all is relocation of libc.so.6 (1777 RELATIVE, 285 GLOB_DAT/JUMP_SLOT, BIND_NOW) and the
symbol/version lookups it does; mapping, search and init are each under 3%.

## Done (commit 1)
A. `reloc::Loads`: PT_LOAD ranges read once per pass; `one` takes `&Loads`.
B. `Object.vers: Option<Versions>` (elf.rs `Dynamic::versions`, `Versions::def_name/need_name`), only when
   DT_VERSYM is present (versym's walk validated every entry). GnuHash now one slice `tab` + `be` +
   `bloom_size`/`chain_at` (keeps `Object` at 0x128: assert in symbols.rs).
Tests added: `reloc_field_must_lie_in_a_writable_segment` (crates/ld/tests/reloc.rs; no test covered
`Outside` before), `version_names_by_index_match_the_table_walk` (crates/ld/tests/symbols.rs).
NOT YET RUN: job `dyn-start-parity-ldtest1` (mei; clippy -p niji-ld --all-targets + cargo test -p niji-ld) was
started at hand-over: read `~/logs/dyn-start-parity-ldtest1.log` first.

## Profile after A+B (2.05M)
pass1 17% (RELATIVE loop), one 12.8%, GnuHash::symbol_count 11.5% (called by symtab_count, versym,
link.rs per object: compute once), Loads::inside 11%, read_phdr 9% (image_span_kind, check_table,
Elf::dynamic), version_name 8.7%, Verdefs::next 2.8%, Object::new 2.7%, fill_sse2 2.7%.

## Next steps (in order)
1. Read the ldtest1 log; fix anything red.
2. Cheap cuts: symbol_count once per object (symtab_count + versym + link.rs all call it); a RELATIVE
   fast path in pass1 (skip `one`'s generic match; check range with a cached last-hit segment); the
   deferred pass re-resolves every symbolic entry (target() before the `deferred` test) - record which
   entries pass 1 deferred, or stop after the last deferred one.
3. Version index O(1) if still hot (Object has no room left; see the size assert).
4. Faults (75 vs 49) not looked at yet: by page and cause.
5. Ratchet row: empty-main PIE release instr on keke (Zen 1) and mei, glibc counts beside it
   (bench/instr_limits.tsv is static-only: needs a new bound file or column; check DYN_TRUE_BOUND).
6. Plants (docs/plants.md format; `cargo xtask plants`): Loads::inside (ignore W flag; `<` for `<=` on
   the end), Versions::def_name (skip the first match / return the next entry), need_name, GnuHash
   bucket offset; spill path of Loads (>8 PT_LOADs) has no test: add one or mark the plant equivalent
   with a reason.
7. Start-up checks before hand-back: `cargo xtask test` and `NIJI_FULL_PROFILE=1 cargo xtask so &&
   NIJI_FULL_PROFILE=1 cargo test -p niji --test hot_text_products`; affected gate; precheck; D-row
   (reserve with `ops reserve --story DYN-START-PARITY D-27`) for the GnuHash contiguity refusal.

## Process notes
- Leg 1 used one heredoc (a python edit script) by mistake: report it.
- A release `cargo xtask so` takes ~14 min on mei; batch changes per build.
