# LG6-CTYPE-OBJECTS: the seven legacy data objects old binaries link to

State: branch `task/lg6-ctype-objects` (from origin/next), HEAD: see `handoff/reports/LG6-CTYPE-OBJECTS.json`.
Plan row LG-6 (plan/06-phases/18-phase-15-legacy-tier-and.md); audit plan/reviews/CUTOVER-RESIDUE-AUDIT.md.
Decision in force: D-5.13 (`records/decisions/D-5/D-5.13.toml`, id reserved with `ops reserve`).

## What was done

* `xtask/src/compat_objects.rs` generates `__ctype_b`, `__ctype_tolower`, `__ctype_toupper`, `__ctype32_b`,
  `__ctype32_tolower`, `__ctype32_toupper` (8-byte writable pointers to `"C"` tables at character 0) and `_obstack`
  (8 zero bytes), all `@GLIBC_2.2.5` (single `@`), into `csys/src/machine/compat_aliases.S` (regenerated with
  `cargo xtask compat-gen`). `spec/abi/compat-missing.tsv` shrinks by these 7 rows.
* `crates/boundary/src/data.rs`: the unversioned `__ctype_*` and `_obstack` statics are no longer `no_mangle`
  (libc.a offered compat-only names, R15-2); they remain for the in-process tests.
* D-5.13: the tables are rendered by the generator (5376 B rodata, libc.so.6 only) rather than pointing at
  `niji_legacy::ctype_tables` (that crate forbids `unsafe`, so its statics have no linkable name). So the objects
  hold the same contents as `*__ctype_*_loc()` at other addresses.
* Probe record: `spec/behaviour/ctype-objects.md` + `ctype-objects.glibc.txt`. Differences row (brief step 4):
  the objects stay on "C" after `setlocale` (docs/differences-from-glibc.md, D-5.13, no compat option yet).
* Tests: `ctype_tables_equal_the_glibc_probe`, `ctype_objects_point_at_character_zero_at_the_compat_version`
  (xtask); `conformance/c/s05_ctype_legacy_objects.c` (dynamic mode, both libraries) via
  `legacy_ctype_objects_give_an_old_binary_the_c_tables` (crates/boundary/tests/s07_locale_symbols.rs).
* Plants: `handoff/LG6-CTYPE-OBJECTS-plants.txt`.

## Gate history (keke, role n2)

* `lg6-ctype-objects-gate` (head 8fb3d2f3f): RED, one program only: `s05_ctype_legacy_objects` on libniji
  exit 3 (its pointer-identity checks against `*__ctype_*_loc()`, which D-5.13 makes false). Every other step ok
  (peak 12G). Fixed in e3a93acf6: the program compares contents over -128..255.
* `lg6-ctype-objects-plants` / `-plants-2`: superseded (malformed lines; then copy 1 stopped on the red program).
* `lg6-ctype-objects-suite` (code head e3a93acf6: `cargo xtask so`, `affected --only conformance:s05_ctype_legacy_objects`
  1 passed, `s07_locale_symbols` 43 passed). `lg6-ctype-objects-plants-3`: 5 CAUGHT, 0 SURVIVED, no WARN
  (results in handoff/LG6-CTYPE-OBJECTS-plants.txt).

* Final head a7e8d87b3 only renames the placeholder in a generator comment (precheck placeholders red at
  36a355f3e): `lg6-ctype-objects-suite-2` (adds `cargo test -p xtask compat_objects`) and
  `lg6-ctype-objects-plants-4` started on it, PENDING at hand-back.

## Counts (keke, `lg6-ctype-objects-startup-7`)

Empty dynamic program, each libc.so.6 with its own build's ld.so (`ld.so --library-path`), instructions:u median
of 5: base (merge base 279a24ee4) 309279, after 309465 (+186, +0.06%); page faults 63 both. R_X86_64_RELATIVE
2219 -> 2225; .rodata 1183956 -> 1189376, .data 100256 -> 100304 bytes.

## Probe-settled reference behaviour (keke, glibc 2.39)

All seven `OBJECT GLOBAL` size 8, `GLIBC_2.2.5` only (libniji's rows identical). In "C" each pointer equals
`*__ctype_*_loc()`; the 32-bit case pointers equal the 16-bit ones. After `setlocale` the program's copies FOLLOW
the global locale. libniji's stay on "C".

## Open (RULING NEEDED)

* Follow `setlocale` with these objects? Recommend a follow-up story storing the new tables into the six objects in
  a cold setlocale path (removes the differences row). Users: none known (LG-0c sweep: 0 binaries bind compat versions).

## Next step

Dispatcher: verify `lg6-ctype-objects-suite-2` and `lg6-ctype-objects-plants-4` (keke) on a7e8d87b3.
