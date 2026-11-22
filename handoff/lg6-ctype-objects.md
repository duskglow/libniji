# LG6-CTYPE-OBJECTS: the seven legacy data objects old binaries link to

State: branch `task/lg6-ctype-objects` (from origin/next), HEAD: see the report json. Plan row LG-6
(plan/06-phases/18-phase-15-legacy-tier-and.md); audit plan/reviews/CUTOVER-RESIDUE-AUDIT.md.

## What was done

* `xtask/src/compat_objects.rs` generates `__ctype_b`, `__ctype_tolower`, `__ctype_toupper`, `__ctype32_b`,
  `__ctype32_tolower`, `__ctype32_toupper` (8-byte writable pointers to `"C"` tables at character 0) and `_obstack`
  (8 zero bytes), all `@GLIBC_2.2.5` (single `@`), into `csys/src/machine/compat_aliases.S` (regenerated with
  `cargo xtask compat-gen`). `spec/abi/compat-missing.tsv` shrinks by these 7 rows.
* `crates/boundary/src/data.rs`: the unversioned `__ctype_*` and `_obstack` statics are no longer `no_mangle`
  (libc.a offered compat-only names, R15-2); they remain for the in-process tests.
* Decision: `records/pending/LG6-CTYPE-OBJECTS-1.toml` (D-NEW, series D-5): why the tables are rendered by the
  generator (5376 B rodata in libc.so.6 only) rather than pointing at `niji_legacy::ctype_tables` (that crate
  forbids `unsafe`, so its statics have no linkable name).
* Probe record: `spec/behaviour/ctype-objects.md` + `ctype-objects.glibc.txt`.
* Tests: `ctype_tables_equal_the_glibc_probe`, `ctype_objects_point_at_character_zero_at_the_compat_version`
  (xtask); `conformance/c/s05_ctype_legacy_objects.c` (dynamic mode, both libraries) via
  `legacy_ctype_objects_give_an_old_binary_the_c_tables` (crates/boundary/tests/s07_locale_symbols.rs).
* Plants: `handoff/LG6-CTYPE-OBJECTS-plants.txt` (offset, default version, class bit, negative case entries, and
  the generated object end to end).

## Probe-settled reference behaviour (keke, glibc 2.39)

All seven `OBJECT GLOBAL` size 8, `GLIBC_2.2.5` only. In "C" each pointer equals `*__ctype_*_loc()`; the 32-bit
case pointers equal the 16-bit ones. After `setlocale` the program's copies FOLLOW the global locale (C.UTF-8 and
en_US.UTF-8: 16-bit tables unchanged in content; `__ctype32_*` differ at 128-255). libniji's stay on "C".

## Open (RULING NEEDED)

* Follow `setlocale` with these objects (the reference does; libniji keeps "C")? Recommend: a follow-up story that
  stores the new tables into the six objects (through the GOT, so a copy-relocated program sees them) in a cold
  setlocale path, plus a differences row/compat option only if the owner prefers to keep "C". Users: none known
  (LG-0c sweep: 0 binaries bind compat versions).

## Jobs (keke, role n2)

* `lg6-ctype-objects-base-so` (origin/next baseline build): libc.so.6 has 2219 R_X86_64_RELATIVE; .rodata 1183956,
  .data 100256 bytes. Copies in `~/work/lg6-ctype-objects/base/`.
* `lg6-ctype-objects-gen`: compat-gen + `cargo test -p xtask compat_objects`: 6 passed (head 8fb3d2f3f's code).
* `lg6-ctype-objects-gate` (head 8fb3d2f3f: clippy, static, `affected --base origin/next --run`): RUNNING at refresh.
* `lg6-ctype-objects-plants`: SUPERSEDED (refused: two malformed plant lines, fixed in the plants file).
* `lg6-ctype-objects-plants-2` (`~/work/lg6-ctype-objects/plants.txt` = the committed plants file): RUNNING.

## Next step (refreshed session)

1. `~/bin/jobwait lg6-ctype-objects-gate lg6-ctype-objects-plants-2` on keke; read `~/logs/<label>.log` (grep
   CAUGHT/SURVIVED/WARN; affected summary). Fix any red (only failing suite again).
2. After count: `readelf -r -W ~/w/n2/target/niji/libc.so.6 | grep -c R_X86_64_RELATIVE` (expect 2219 + 6 minus the
   six dropped `__ctype_*` AtomicPtr statics if they were kept before) and `readelf --dyn-syms -W` grep of the seven
   names (expect `OBJECT GLOBAL 8 name@GLIBC_2.2.5`).
3. Sync the final head (this commit only adds handoff text and the plants file: no code change since the gate's
   head), run `cargo xtask precheck --jobs lg6-ctype-objects-gate,lg6-ctype-objects-plants-2 --head <sha>` on the
   role, write `handoff/reports/LG6-CTYPE-OBJECTS.json`, push, `ops handback-check`, hand back.
