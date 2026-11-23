# LD-SYMBOL-VALIDATE-ONCE: the loader's symbol lookup over tables validated once at map time

**State (leg 1, 2026-11-23 ~18:20Z):** branch `task/ld-symbol-validate-once` from origin/next 3bf4b7342; code commit
d60bce3ea (pushed; NOT YET COMPILED: no local cargo, and the role was busy with the base build). Role keke `~/w/r5`
(Zen 1, 60 vCPU). Brief: ~/state/briefs/LD-SYMBOL-VALIDATE-ONCE.md (RETHINK, Effort max). Decision id reserved:
**D-27.96** (record not written yet: records/decisions/D-27/, §10). Wall-time deadline 21:53Z.

## Premise and what was tried (start here, not from the code)
Audit (results/perf-gap-structure/README.md §3.1): 1,510 user instructions per imported symbol, reference 643 eager.
Tried before: LD-LOOKUP-COST (D-27.77, hashed version memo: 1,947 -> 1,507), LD-LOOKUP-STRUCTURE (version table,
+34, abandoned), LD-LOOKUP-RETHINK D1b (walk-level Bloom, -50/lookup; held on `origin/task/ld-lookup-rethink-2`,
NOT on next). Its phase split (per lookup, libc names, 1,449): decode+dispatch 191, check+store 64, reference
symbol/name/request 205, hash 82, versions 271, scope walk 151, per-object filter 221, candidate check 255.
**Overlap:** this story subsumes the rethink's briefs D2 (LD-VERSION-INDEX: versions by index) and D3
(LD-RELOC-FLAT: flat path over a view validated at load); it conflicts textually with D1b only if D1b changes
`Object::lookup`/`for_each_candidate` (the exact path): the quick path does its own Bloom test per object, so
D1b's gain is already in it for the relocations; D1b still helps dlsym and the exact path.

## Designs considered (§11)
A (adopted): validate once + quick walk + version by index (below). B: per-process global name index (hash every
export once): rejected, ~30 x exports (libc ~3k) > the whole gap (also rejected in LD-LOOKUP-COST). C: persistent
resolution cache keyed by build ids (prelink-like): bound ~0/symbol but a new on-disk trust boundary: needs a
ruling, not this story. D: relocation-major batching (resolve each symbol index once): few duplicates in practice;
not done. Estimated bound for A: ~150-250 per lookup (hash ~4/byte, Bloom ~12/object, chain+name ~30, version ~3).

## The change (d60bce3ea; crates/ld only)
* `symbols.rs` `Object::build` -> `validate` (cold): LE; GNU table; Bloom count a power of two; symbol count known
  and count x 24 bytes mapped; versym count x 2 mapped; strtab ends in NUL; GNU chains end below a count <= the
  symbol count (`GnuHash::symbol_count` again only when a SysV table set the count). Result in `GnuHash::quick`
  (u8 in tail padding: `Object` stays 0x128; check the size assert compiles).
* `Object::quick_symbol/quick_version/quick_lookup/quick_candidate` (+ cold `quick_version_named`): in-place LE reads;
  validated reads "trust" (failed bounds check = no candidate); name offset and version index checked per entry
  (-> `Probe::Slow`, exact path). `same_bytes` inline name compare. `quick_id(obj, index) = (obj+1)<<16|index`.
* `UniqueRegistry::scope_ordered()` (default false; `ScopeOrder` true): unique hits stay quick only at the start.
* `reloc.rs`: `Wants` (64 x (id, name place) = 512 B per pass), `fill_wants` (cold; one walk of verneed then
  verdef; first entry per index decides; any table error = all unknown), `quick_target` (None = exact `one`),
  `quick_kind`; used in `pass1` and `relocate_deferred` for GLOB_DAT/JUMP_SLOT/64. A requirement's id is learned
  from the first definition whose version name matched (cold compare), then integer compares.
* Tests (crates/ld/tests/reloc.rs): `reloc_quick_versions_bind_by_index_each_to_its_own`,
  `reloc_quick_unterminated_string_table_is_the_exact_walks_error`, helper `model_of`.
* Plants: handoff/LD-SYMBOL-VALIDATE-ONCE-plants.txt (version-index-skip, validation-skip, quick-bloom).

## Measurement kit (keke `~/work/ld-symbol-validate-once/`, copies in the local scratch dir)
`prod.sh <tag>`: `NIJI_FULL_PROFILE=1 cargo xtask so` in ~/w/r5, copy target/niji to `prod-<tag>`, print IFUNC /
IRELATIVE / symbolic reloc counts of libc.so.6. `meas.py <products> ~/w/r5 <outdir> [imports|all]`: the audit's probe
5 (imports 0/50/200 lazy and -z now, per-import user instructions; the nine binaries, both loaders, LANG=C.UTF-8,
median of 7). Job **ld-symbol-validate-once-before** (kind probe) = prod.sh before + meas.py on prod-before:
running at 18:16Z (base build). Its log: ~/logs/ld-symbol-validate-once-before.log.

## Exact next steps
1. `~/bin/jobwait ld-symbol-validate-once-before` (do NOT sync before it ends: it builds in ~/w/r5). Record the
   before numbers (per import niji/ref now; nine-binary table) in the report.
2. Sync (sync_role.sh ... keke r5), then one job: `cargo clippy -p niji-ld --all-targets -- -D warnings` and
   `cargo test -p niji-ld --test reloc --test symbols --test elf_tables`; fix compile/lint errors.
3. Job `prod.sh after` + `meas.py prod-after ... all`: per-import and nine-binary after. Target <= 300/import.
   If above, profile (validation/wl_trace.sh or the audit's trace_start.py) and say what bounds it.
4. Plants: `cargo xtask plants handoff/LD-SYMBOL-VALIDATE-ONCE-plants.txt --dir /dev/shm/claude/ld-symbol-validate-once/plants-1 --timeout 600`.
5. Gate (direct tier, `cargo xtask affected --run` under ~/bin/gate), plus §9 start-up checks (`cargo xtask test`;
   `NIJI_FULL_PROFILE=1 cargo xtask so && NIJI_FULL_PROFILE=1 cargo test -p niji --test hot_text_products`): the
   loader's `DYN_START_INSTR` bounds will go red DOWNWARD (more than 10% under): tighten them as LD-LOOKUP-COST did
   (ALWAYS FORWARD). Diff target/product/fast/layout-bins.tsv vs base (loader growth is not chased, §9).
6. D-27.96 record, report json, handback-check, precheck. Then hand back.
