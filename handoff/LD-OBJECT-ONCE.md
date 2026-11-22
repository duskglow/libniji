# LD-OBJECT-ONCE: one validating pass per loaded object (LD-VERSIONS-ONCE merged in)

**State:** branch `task/ld-object-once` from origin/next fd973fc44. Role keke `~/w/n3` (Zen 1 host). Briefs:
~/state/briefs/LD-OBJECT-ONCE.md, ~/state/briefs/LD-VERSIONS-ONCE.md. Decision: D-27.85
(records/decisions/D-27/D-27.85.toml). Plants: handoff/LD-OBJECT-ONCE-plants.txt.

## What changed (no check dropped; every error the same, in the same order)
- `crates/ld/src/elf.rs`: `Tags` (the nine tags a lookup view reads, one pass over the entries, `once`'s
  answers incl. `DuplicateDynamic`); `Hashes` (`sysv_hash`/`gnu_hash` read once, results incl. errors kept);
  `sysv_hash_at`/`gnu_hash_at`/`pair_t`; `symtab_with`/`versym_with_versions` take them. `GnuHash::symbol_count`
  scans the bucket words as one slice (`bucket_range`, branch-free max and lowest non-empty); a bucket below
  `symoffset` falls back to the old checked walk (`bucket_top_checked`, names the first). `check_verdefs` /
  `check_verneeds`: the iterator walks' checks from fixed record offsets, no entry structs; `check_string`
  skips the NUL search when the string table ends in NUL. The same walk yields the `Versions` index
  (`versym_versions`). `Verdefs::has_hashed`: hash-first lookup that answers only "found" when the walk by
  name would find it with no error first. `image_tables` / `image_span_kind_with` (`ImageTables`, 16 slots,
  fallback walk past that).
- `crates/ld/src/symbols.rs`: `Object::new`/`with_style` -> `build` with one `Tags` and one `Hashes`.
- `crates/ld/src/reloc.rs`: `check_versions_live` tries `has_hashed` before its walk.
- `crates/boundary/src/ld/image.rs`: `image_view` reads `image_tables` once.
- Tests: elf_tables `relocation_tags_checked` (+2 duplicate view tags), `verdef_malformed`/`verneed_malformed`
  now also assert `versym()` alone (`versym_err`), new `image_span_kind_finds_each_table`; reloc new
  `reloc_version_check_hashes_never_change_the_answer`. hot_text_products `DYN_START_INSTR` tightened.

## Numbers (release products, STARTUP-REPROFILE kit meas.sh, median of 101, ASLR off, Zen 1 host)
| row (user instructions) | next fd973fc44 | WIP head1 (view only) | this branch | glibc 2.39 |
|---|---|---|---|---|
| n0 empty main | 268,797 | 203,300 | **196,486** (-72,311, -26.9%) | 96,040 |
| n3 three one-function libraries | 378,047 | 299,965 | **290,575** (-87,472) | 132,752 |
| per extra object (n3-n0)/3 | 36,417 | 32,222 | **31,363** (target 25,000: NOT reached) | 12,237 |
| t /bin/true | 329,173 | 263,306 | 254,366 (-74,807) | 122,349 |
| n200 200 GLOB_DAT imports | 554,706 | 493,132 | 486,317 (-68,389) | 230,916 |
Faults 58/70, system calls 28/58 (n0/n3): unchanged. hot_text_products' own count (Zen 1): fast 219,145, release
196,663 -> bounds Zen 1 [223_600, 200_600], Skylake-SP [223_700, 200_800] (Zen 1 + 117, + 2%; not run there).
Skylake-SP counts and Intel PT: NOT run this run (no build on mei; products are in keke
~/work/ld-object-once/head2-rel and base-rel for copying).

## Jobs (keke)
Counts: ld-object-once-base-count (next), ld-object-once-head1-count (view change only), ld-object-once-levels
(on the commit "read the image tables once per object": the products code of the final head; later commits
change tests, bounds, the record and this note only).
Final head 884b3e8e2 (code): clippy (workspace, all targets, all features) and `cargo xtask static` clean;
ld-object-once-gate3 (direct tier), ld-object-once-plants3, ld-object-once-startup (`cargo xtask test`, release
products, release hot_text_products; starts when gate3 ends).
SUPERSEDED: ld-object-once-plants (run 1: 8 CAUGHT, 2 SURVIVED: verdef-revision, image-table-address -> tests
`versym_err` cases and `image_span_kind_finds_each_table` added), ld-object-once-gate and ld-object-once-plants2
(stopped: the gate's clippy step failed on `type_complexity` in elf_tables.rs, fixed by `type Case`).

## Open / next
- Per-object target (<= 25k) not reached: 31,363. Not profiled per function this run (PT needs the
  Skylake-SP host). Candidates from STARTUP-REPROFILE's per-object split: `Object::lookup`/`symbols::lookup`
  walking past the libraries (the symbolic-lookup story), `link`'s own per-object code, RELRO, `check_table`;
  the other `Dynamic::once`/`get` users (reloc: RELACOUNT, TEXTREL/FLAGS, RELA/JMPREL/RELR tags; left alone
  because LD-RELR-LOOP works there) could take a `Tags`-like one-pass table too.
- `check_versions`' own count after the change was not isolated (inlined into `link` in release); NOT VERIFIED
  against LD-VERSIONS-ONCE's <= 5k / <= 10k.
- Do not redo: the hash-first version check must stay "found only"; a wrong `vd_hash` must never turn into
  Missing (test `reloc_version_check_hashes_never_change_the_answer`).
