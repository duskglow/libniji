# RENAME-NIJI-CODE(-2, -3) report: Liella -> libniji (code)

## RENAME-NIJI-CODE-3 (branch `task/rename-niji-code-3`, role keke `~/w/r5`, scripts in keke `~/work/RENAME-NIJI-CODE-3/`)

Inherited head 3abfa7769: clippy + static green (`rename-niji-code-3-lint0`).

### CI's red and its cause
Train 756, native_static: `FAIL file: pwrite was short`. The rename shortened `LIELLA`/`liella` to `NIJI`/`niji` (two
bytes) and the numbers that were the old literals' lengths stayed: `pwrite(b"NIJI", 0)` checked against 6, `pread` of
"native" at offset 7 (after "liella "; it would have failed next), and xtask's expected `ok file: 25 bytes written`
(the data is 23 bytes). Fixed in 953c0081d: the example derives the lengths from its literals (`head.len()`, the
offset after the first space, the read-back built from `data` and `head`); the expected line says 23 (it is exact
stdout, so it stays a literal).

### Length sweep (the same class elsewhere)
Method: `git diff -U14 origin/next...HEAD` (the rename's own diff; every commit on the branch is this story's), outside
handoff/, records/, docs/, plan/, spec/ and *.md. Scripts (scratch, not committed): `sweep.py` lists every removed line
whose literal held the old name with an unchanged number within 14 lines (3104 rows); a row is STRONG when the number
is the old literal's length, length + 1, or the offset just past the old name (279 STRONG rows, all read, including
the 105 that match only 6 or 7, the lengths of `liella`/`liella_`: that is where CI's bug and the codemap one were);
`slices.py` lists unchanged lines within 14 lines that slice, index, offset, seek/pread/pwrite, size an array or emit
`.ascii/.byte/.long/.p2align` (772 lines; the offset/slice subset and every line near a `Liella` literal read);
`offsets.py` lists, in all 1174 touched files, slices/substr/cut/`${x:N}`/`k + N` within 3 lines of the new name
(66 lines, all read; this catches unquoted stems such as shell/awk regexes and `match_indices` arguments).
Also checked: literals with `\0` (string tables, prctl/hostname/xattr buffers, note names), fixed arrays initialised
from renamed strings (`Liella` -> `libniji` grows by one byte), byte counts quoted in expected outputs (native
example, conformance, csys/util, tools/deb, validation/smoke), and the transcripts/corpus/fuzz seeds the rename touched.
Found and fixed:
- crates/native/examples/demo/mod.rs and xtask/src/native_examples.rs (above).
- conformance/c/s22_sync.c: the semaphore name's PID was written at offset 13 (= len "/liella-conf-"); after the
  rename that left a NUL inside the name and dropped the PID silently (the program still passed). The offset now
  comes from the prefix (`SEM_PREFIX`, 953c0081d), and the program fails with status 34 when the name is not as long
  as what it wrote (609ba6252).
- xtask/src/codemap.rs `modules()`: matched `niji_` but skipped 7 bytes (`liella_`), so `niji_core::names` became
  `re::names` in the generated README's per-section module lists. The skip now comes from the stem; new unit test
  `boundary_sources_name_their_modules_by_crate` (5c6e47be0). The train's `codemap --in-place` uses this parser, so
  its README output changes with the fix (the train regenerates it; the branch does not carry it).
Checked and already right (earlier hand commits or self-deriving): appendix_a SECRET `[u8; 8]` `niji!!!!`, prctl name
(16 bytes), s24 hostname/domain slices `[..10]`/`[..15]`, xattr list `user.niji\0`, scanner_races and s01 putenv fixtures,
dylib/nonshared note owners and sizes, NSC magic (`NIJI_NSC`, 8 bytes), rt TAG_PREFIX (prefix-derived), sync.rs's
`/dev/shm/sem.niji-` temp name (40-byte buffer), csys/util version strings (no widths), ldconfig transcripts (cache
written at test time). Everything else in the lists was a coincidental number (errno values, buffer sizes far larger
than the name, line counts).

### Jobs (keke), final code head 5c6e47be0 (synced head 170f53589; later commits change only handoff/)
- `rename-niji-code-3-lint2`: `cargo xtask clippy` + `cargo xtask static`, rc 0.
- `rename-niji-code-3-plants3`: 6 plants (pwrite length, pread offset, read-back, expected count, s22 offset, codemap
  stem skip): 6 CAUGHT, 0 SURVIVED, 0 WARN (rc 0); the file with its results is handoff/RENAME-NIJI-CODE-3-plants.txt.
  The three baselines (native example, conformance s22_sync, the new codemap unit test) passed on the fixed code.
- `rename-niji-code-3-gate2`: the train's generators (`records-gen`, `kernel-index`, `codemap`, all `--in-place`, in the
  role only, ruling #6086), then `cargo xtask test` (all steps, what CI runs), then `NIJI_FULL_PROFILE=1 cargo xtask so`
  and `NIJI_FULL_PROFILE=1 cargo test -p niji --test hot_text_products`: rc 0 in 110 min (13.2 GB written). Every
  `cargo xtask test` step green: products, tests-nextest 6738 run / 6738 passed (786 skipped), including the 10
  affected::direct_tier_ tests that -2's gate failed on the stale view (20 PASS lines: plain and interposed), the
  interposed run, conformance 137 passed /
  0 failed, native-examples, rust-dylib, symgap; hot_text_products 26 passed. The role tree keeps the regenerated
  views until its next sync (never committed).
- Superseded: `lint0` (inherited head, green), `lint1` (609ba6252, green), `plants1` (stopped: two mutants used
  `+ 2`, which the example's clippy step refuses before the program runs, WARN-NO-MSG; restated as the old number `6`),
  `plants2` (stopped for the codemap fix; no verdict yet), `gate` (stopped for the codemap fix, which its
  `codemap --in-place` step uses).

### Refusals and process notes
No refusals. One slip: an empty heredoc (`python3 - <<'X'` with no body, a no-op) was run once while editing the
plants file in scratch, against the no-heredocs rule; the edit itself was done with a `python3 -c` one-liner.

## RENAME-NIJI-CODE-2

Branch `task/rename-niji-code-2`, base `origin/next` e1afc4e47, code head 30766aacb (4d7b91d99 + report commits change only handoff/). Host: mei (role `~/w/s1`);
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
- 30766aacb three tests fixed whose fixtures depended on the old length (s01 sym_putenv offset, scanner_races
  Getenv byte, hottext v0 back-references recomputed from the base strings).
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
Final code head 30766aacb (synced head 4d7b91d99): `rename-niji-code-2-failing` rc 0 (clippy, static, records check,
csys/abi/native/headers/compat gen and stubs unchanged, then the suites the full gate failed: s01_startup_symbols,
scanner_races, hottext::, rust-dylib); `rename-niji-code-2-plants6` (17 plants; plants5 on ee350824a: 17/17 CAUGHT).
Full gate `rename-niji-code-2-gate5` (ee350824a, `cargo xtask test` + hot_text_products): hot_text_products rc 0;
`cargo xtask test` 6737 run, 7 failed: 5 fixed in 30766aacb (rerun green in `failing`), 2 are
`affected::tests::direct_tier_reaches_conformance_programs_through_their_entry_points` and
`direct_tier_follows_cross_crate_users_only_into_matching_sections`, which read docs/codemap/symbols.tsv, a
train-owned view that still lists `liella_*` implementation names (a branch may not edit it).
`rename-niji-code-2-codemap-check` (RAM copy, view regenerated by `cargo xtask codemap` and put in place in the copy
only): all 10 direct_tier_ tests pass; with the tracked view the same 2 fail (in `failing`, not counted).
math-gen/tables/remez `--check` green at ee0a5e923 (`lintgen3`); nothing they read changed after.
Evidence, not on the final head: `proof-before` (rc 1: base swap root), `proof-after3` (03db94b18, rc 1: swap root
and rust-dylib, fixed in ee350824a), `lint7` (ee350824a: dylib built, exports compared).
Superseded: clippy0 (inherited head, red), lintgen1-3, lint4-7 (older heads), gate/gate2/gate3/gate4 and
plants/plants3 (stopped for a newer head), gate5 (red, see above), plants2 (macro-features SURVIVED, fixed in
43846bf53), plants4/plants5 (older heads, all CAUGHT), proof-after, proof-after2 (older heads).

## RULING NEEDED
- The full gate stays red on the 2 affected:: tests until docs/codemap/symbols.tsv is regenerated: recommend the
  landing regenerate the train-owned views (codemap first) before its gate for this branch.

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
