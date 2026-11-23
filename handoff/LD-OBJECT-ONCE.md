# LD-OBJECT-ONCE: one validating pass per loaded object (LD-VERSIONS-ONCE merged in)

**State:** done; should land (run 3: merged next, CI red not reproduced). Branch `task/ld-object-once-3` (= `task/ld-object-once-2`
+ a merge of origin/next 8e1837914 + this note), first cut from origin/next fd973fc44; code head c6f3f05a5 (later commits: plants line, notes, reports only). Run 1: keke `~/w/n3`
(Zen 1 host). Run 2 (LD-OBJECT-ONCE-2): mei `~/w/s4` (Skylake-SP host). Briefs: ~/state/briefs/LD-OBJECT-ONCE.md,
~/state/briefs/LD-VERSIONS-ONCE.md, ~/state/briefs/LD-OBJECT-ONCE-2.md. Decision: D-27.85
(records/decisions/D-27/D-27.85.toml). Plants: handoff/LD-OBJECT-ONCE-plants.txt (10 plants).

## Run 3 (LD-OBJECT-ONCE-3, branch `task/ld-object-once-3`, mei `~/w/s1`, Skylake-SP host)
- CI red on a9be076d8 (CI merge 4d6305816, 20:28Z): eleven address-lookup tests (dladdr/_dl_find_object one object
  short, atexit_owner main handlers unattributed, s27_dladdr status 134, ...). LD-RELR-LOOP-2 (same base fd973fc44)
  failed the same eleven at 22:15Z; trains on next alone at 21:05Z/21:55Z and 22:51Z (8e1837914) are green.
- Merged origin/next 8e1837914 (clean, no conflicts; no code change in this run). On that head, NOT reproduced:
  direct-tier gate 13 suites / 1,048 passed (atexit_owner, dl_find_object_index, dlclose, dlopen_concurrent,
  dlopen_process_objects, runpath_search included), and `cargo xtask test` (what CI runs, s27_dynlink_clauses incl.)
  green; plants 10/10 caught. Between 08af73077 (green train) and 8e1837914 only LDSO-PROGRAM-OPTIONS-REST-2 and
  THREAD-EXIT-NO-UNWINDER landed (they touch `ld/entry.rs`, `ld/image.rs`, `ld/state.rs`, `ld/dlopen.rs`).
- Cause NOT FOUND: the failing path (index.rs `find` / readers.rs) reads only program headers, nothing this branch
  changed (elf.rs tags/hashes/versions, symbols.rs, reloc.rs, image.rs `image_tables`). The red needed a
  next of 20:28-22:15Z plus a fd973fc44-based branch; reproducing it needs a build of an old next, which the
  clean-room git rules forbid (no old checkouts of others' work). If CI goes red the same way again, keep
  ci-z's products (`target/niji`) with the scratch so the failing ld.so can be examined.
- Start-up kit counts were not re-measured (no code change); hot_text_products bounds pass on the merged head.
- Jobs (mei, merged head): ld-object-once-3-lint (clippy, static, so), ld-object-once-3-gate, ld-object-once-3-plants
  (10 CAUGHT, no WARN), ld-object-once-3-startup (`cargo xtask test` 7,152 passed + full-profile hot_text_products 33).

## What changed (no check dropped; every error the same, in the same order)
- `crates/ld/src/elf.rs`: `Tags` (the nine tags a lookup view reads, one pass over the entries, `once`'s answers incl.
  `DuplicateDynamic`); `Hashes` (`sysv_hash`/`gnu_hash` read once, results incl. errors kept); `GnuHash::symbol_count`
  scans the bucket words as one slice (a bucket below `symoffset` falls back to the old checked walk);
  `check_verdefs`/`check_verneeds` check from fixed record offsets and yield the `Versions` index; `Verdefs::has_hashed`
  (hash-first, answers only "found"); `image_tables` (16 slots, fallback walk past that).
- `crates/ld/src/symbols.rs`: `Object::new`/`with_style` -> `build` with one `Tags` and one `Hashes`.
  `crates/ld/src/reloc.rs`: `check_versions_live` tries `has_hashed` first. `crates/boundary/src/ld/image.rs`:
  `image_view` reads `image_tables` once. `Tags::scan`, `Dynamic::tags`, `Dynamic::hashes`, `Object::build` are `pub`
  (ThinLTO `.llvm.<hash>` names otherwise differ between the products and coexist-prefix builds; see run 1 notes in
  git history of this file).
- Tests: elf_tables `relocation_tags_checked` (+2), `verdef_malformed`/`verneed_malformed` assert `versym()` alone,
  `image_span_kind_finds_each_table`; reloc `reloc_version_check_hashes_never_change_the_answer`;
  hot_text_products `DYN_START_INSTR` tightened.

## Numbers (release products, STARTUP-REPROFILE kit meas.sh, median of 101, ASLR off; user instructions)
| row | next fd973fc44 Zen 1 / Skylake-SP | this branch Zen 1 / Skylake-SP | glibc 2.39 Zen 1 / Skylake-SP |
|---|---|---|---|
| n0 empty main | 268,797 / 268,959 | 196,486 / **196,630** (-72,329, -26.9%) | 96,040 / 101,855 |
| n3 three one-function libraries | 378,047 / 378,209 | 290,575 / **290,698** (-87,511) | 132,752 / 138,761 |
| per extra object (n3-n0)/3 | 36,417 / 36,417 | 31,363 / **31,356** (target 25,000: NOT reached) | 12,237 / 12,302 |
| t /bin/true | 329,173 / 329,335 | 254,366 / 254,510 | 122,349 / 128,175 |
| n200 200 GLOB_DAT imports | 554,706 / 554,868 | 486,317 / 486,461 | 230,916 / 236,773 |
Faults 58/70 and system calls 29/59 (n0/n3, Skylake-SP) unchanged. Skylake-SP rows are the final code head's products
(run 2 built them in `~/w/s4`, release); the Zen 1 head rows were counted before the `pub` commits (run 1). The two
classes differ by 144-162 instructions, as STARTUP-REPROFILE found. Kernel instructions (Skylake-SP, not attributed,
they vary between runs): n0 758,779 -> 742,356, n3 1,099,068 -> 1,112,660.
hot_text_products' own count (final head, Skylake-SP): fast 219,314, release 196,807, inside the bounds run 1 set from
Zen 1 + 117 + 2% ([223_700, 200_800] = level + 2.0% each); Zen 1 (run 1) fast 219,145, release 196,663. No bound change.
Layout (§9): `target/product/fast/layout-bins.tsv` of the final head against the merge base's (built in run 2 from
fd973fc44): every bin's bytes identical; 5 sections differ only in their `.llvm.<hash>` suffix (same bin, same size).

## The remaining 31,356 per object (Skylake-SP, Intel PT, release, inclusive, (n3-n0)/3; base in brackets)
- `link` 31,233 [36,293]. Of it, `relocate_first` 17,278 [17,279]: `pass1` 16,567 -> `reloc::one` 12,271 ->
  `reloc::target` 11,401 (own 1,692) -> `symbols::resolve` 9,653 -> `symbols::lookup` 9,501 (own 2,928;
  `Object::lookup` 6,646, the hash-table walks past other objects). `publish_dynamic` 919, `textrel` 529,
  `protect_relro` 314. -> LD-LOOKUP-STRUCTURE / LD-RELR-LOOP are judged against these ~17k.
- `Dynamic::once` 3,330 [6,740]: the users left are reloc's tag reads (RELACOUNT, TEXTREL/FLAGS, RELA/JMPREL/RELR),
  untouched because LD-RELR-LOOP works there; a `Tags`-like table for them is the next ~3k.
- `link` own code (check_versions inlined) 3,327 [3,338]; `Elf::parse` 2,823 (`check_table` 2,079, `read_header`
  413); `Object::new` 1,397 [5,634]; `Elf::dynamic` 1,318; `map_planned` 1,265; memset/memmove 1,065; init hooks 616.
  Gone: `image_span_kind` 948 per object; `bytes_at` 887 -> 425, `bytes_from` 630 -> 210.

## Jobs
Run 2 (mei, final code head; tree synced from df3ea5f37, the code of c6f3f05a5): ld-object-once-2-gate (direct tier,
13 suites, 1,017 passed / 0 failed, peak 1.8G), ld-object-once-2-plants (10 CAUGHT, 0 SURVIVED, no WARN),
ld-object-once-2-startup (`cargo xtask test` + release products + release hot_text_products, rc 0),
ld-object-once-2-fastbound (fast products + hot_text_products, 33 passed), ld-object-once-2-base-pt and
ld-object-once-2-head-pt (kit counts + PT), ld-object-once-2-basefast (merge base's fast layout-bins).
Run 1 (keke): see the run-1 report handoff/reports/LD-OBJECT-ONCE.json (gate5, plants5+6: all 10 caught; startup3 was
cut off by run 1's end, superseded by run 2's startup job).

## Open / next
- Per-object target (<= 25k) not reached: 31,356. Next by size: symbol lookup (~17k, LD-LOOKUP-STRUCTURE /
  LD-RELR-LOOP), reloc's `once` tag reads (~3.3k), `check_table` (~2k).
- `check_versions`' own count is not isolated (inlined into `link` in release); NOT VERIFIED against
  LD-VERSIONS-ONCE's <= 5k / <= 10k (link's own code per object is 3.3k in total, so at most that per object).
- Do not redo: the hash-first version check must stay "found only"; a wrong `vd_hash` must never turn into Missing
  (test `reloc_version_check_hashes_never_change_the_answer`).
- The comment above `DYN_START_INSTR` still says the Skylake-SP bounds were not run in that test; they now are
  (numbers above); update the comment with the next change to that file.
