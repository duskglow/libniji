# RENAME-NIJI-CODE(-2) report: Liella -> libniji (code)

Branch `task/rename-niji-code-2`, base `origin/next` e1afc4e47, code head ee350824a. Host: mei (role `~/w/s1`);
all before/after records on mei, `~/work/RENAME-NIJI-CODE-2/{before,after}` (before = a RAM copy of the primed base
tree, stem liella; after = a RAM copy of the role, stem niji; same proof script `proof3.sh`, `compare.sh`).

## Commits of RENAME-NIJI-CODE-2 (on top of RENAME-NIJI-CODE's, see handoff/rename-niji-code.md)
- 31d7587f1 RUST_IDENT rule: a bare `Liella` as a Rust type or variant becomes `Niji` (clippy was red on
  `heap::libniji`, `Build::libniji`).
- 6db2e18bf PROTECT the compat records' key list (reader xtask records schema + tools/records_import.py keep
  `liella`, records/ is history) and the two cache_write.rs test names D-27.43 cites; RUST_IDENT covers
  `starts_with("Liella")` (native-gen: `NijiTimespec has no C mapping`).
- 82d68a3a3 + d9ccada4e stub files `cargo xtask stubs` still owns are left to it (s31 quotes spec prose).
- ee0a5e923 include/niji/native.h as native-gen writes it (comment rewrap only).
- f9f9eddf5 source files with a NUL byte are text (crates/core/src/time/zone.rs kept two prose mentions).
- 43846bf53 niji_macro_always_defined checks features.h's own fallback (plant macro-features survived before).
- 03db94b18 lr1's hash of the renamed fields (b4a17b0ef811ee2c), recorded in place; pending record updated.
- ee350824a rust-dylib takes rustc's `libniji_native.so` and links it as `libniji.so` (Q2); before, the rename
  step failed ("No such file").

Reproduction: the head's script on e1afc4e47 equals the branch except the hand commits listed in the handoff
(checked three times on mei, last at d9ccada4e: only appendix_a_primitives.rs, s24_system_symbols.rs, dylib.rs,
notice.h, headers.rs differ, all from 8edbe7557/36faffcf0/8e5551fea). `--self-test` PASS; `--check` 0 changes.

## Proof (mei, before = e1afc4e47, after = 03db94b18 for products and packages, ee350824a for the dylib)
- Steps, both sides: coexist so/build/check/lintian rc 0; system so/build/check-system/check-symbols/lintian
  rc 0; empty root rc 0; **swapped trixie root rc 1 on both, `same=54 differ=7`, the differ lines identical
  after renaming** (the ldd-1..7 cases DL-TRACE-INTERP fixes; not masked, not fixed here).
- check-coexist, check-system, check-symbols, root-system, deb lists: identical after renaming. Lintian tags:
  identical sets (coexist 102 lines, system 142).
- Exported dynamic symbols of the 14 shared products: **identical sets after renaming (4278 rows)**; the only
  textual change is the names `LIELLA_*`/`liella_*` -> `NIJI_*`/`niji_*` (version nodes NIJI_NATIVE_1 45,
  NIJI_PRIVATE 71, NIJI_OWNER_1 3). Rust dylib `libniji_rust.so.lr1`: 2164 exports both sides (2086 FUNC,
  78 OBJECT), the same after removing crate names and Rust hashes, except the renamed `NIJI_RUST_ABI` and the
  crate metadata symbol.
- Layout: all 12 contracts hold on both sides (STARTUP-DATA-WINDOWS 1 ro window, 7 rw pages, 99 of 99 start-up
  objects); the layout-check output is identical apart from paths and the verdict-cache key.
- Start-up, neutral static program (5 runs): before 6709-6710 instructions:u, 17-18 page faults; after
  6683-6684 instructions:u, 17-18 page faults. Static binary 160 bytes smaller.
- Sizes: mostly name lengths (.dynstr libc -238, libresolv -136). Code: ld.so .text -128, libc.so.6 -192,
  libm.so.6 -4864 (libm.a -4809, so not link padding), libc.a -167; libm .rodata +192. **Cause not traced**
  (hypothesis, not verified: rustc's codegen-unit partitioning and symbol hashes depend on the crate name, so
  inlining and ordering move a little). Every number is smaller or equal except .rodata +8/+192.
- Dynamic start-up: NOT MEASURED on either side. These default products' loader finds libc.so.6 neither
  through DT_RUNPATH nor DT_RPATH to a real directory, and takes no `--library-path`; both sides fail the same
  way (the failing load counts 85095 vs 85086 instructions). Measuring it needs installed products (the
  packages' paths) or LD_LIBRARY_PATH inside the program's own environment.

## Jobs (mei)
Final head ee350824a: `rename-niji-code-2-lint7` (clippy, static, records check, csys/abi/native/headers/compat
gen and stubs unchanged, macro test, rust-dylib + export comparison) rc 0; `rename-niji-code-2-gate5`
(`cargo xtask test` full tier + `NIJI_FULL_PROFILE=1` so + hot_text_products); `rename-niji-code-2-plants5`
(17 plants). math-gen/tables/remez `--check` green at ee0a5e923 (`lint3`); nothing they read changed after.
Evidence, not on the final head: `proof-before` (rc 1: base swap root), `proof-after3` (03db94b18, rc 1: swap root
and rust-dylib, fixed in ee350824a), plants4 at 03db94b18: 16/16 CAUGHT.
Superseded: clippy0 (inherited head, red), lintgen1-3, lint4-6 (older heads), gate/gate2/gate3/gate4 and
plants/plants3 (stopped for a newer head), plants2 (macro-features SURVIVED, fixed in 43846bf53), proof-after,
proof-after2 (older heads).

## INFORMATIONAL
- Codegen moved slightly with the crate names (libm .text -4.8 KB, static start-up -26 instructions): the
  landing's benchmark/workload count should confirm no row got slower (not measured per row here).
- Records keep `liella` (compat key, `liella.` tunable text, D-27.43 test names); renaming them is one change of
  reader + records + test names (records correction or RENAME-NIJI-DOCS).
- docs/toolchain.md still shows lr1's old hash and `LIELLA_RUST_ABI*` (RENAME-NIJI-DOCS).
- `git grep -i liella` at the head: 2012 files; outside handoff/, records/, docs/, plan/, spec/behaviour,
  validation/results/, results/ and *.md: tools/rename_niji.py (its table), bench/perf-ledger.tsv and
  bench/cycle_cpus.tsv and conformance/c/s04 (`liella-historic` references), spec/clauses.tsv, spec/schedule.tsv
  and the s31 stub file (spec prose quotes), crates/ld/tests/cache_write.rs (cited test names),
  xtask/src/records/schema.rs and tools/records_import.py (records key), tools/deb/build.sh and
  check-system.sh (author identity), validation/lab_host.sh and wl_prime_base.sh (`liella-ops`), tools/probe
  (LIELLA-UTILITIES-DESIGN, liella-historic), fuzz/seeds/ld_cache (binary seeds).

## Refusals
None.
