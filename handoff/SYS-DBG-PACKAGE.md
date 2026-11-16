# SYS-DBG-PACKAGE: libc6-dbg from the system build (INCOMPLETE)

Branch `task/sys-dbg-package` (from origin/next 24dc5587a). Brief: `~/state/briefs/SYS-DBG-PACKAGE.md`
(gap 12 of plan/package-parity-audit.md). Build role mei:~/w/s1.

## State

* `tools/deb/system.sh`: a seventh package, `libc6-dbg` (Section debug, Priority optional, Multi-Arch: same,
  Depends: libc6 (= version), Provides: libc-dbg, libc6-dbgsym). Before the strip, every shared object of
  libc6 and libc-bin's four programs is split with `objcopy --only-keep-debug --compress-debug-sections=zlib`
  into `/usr/lib/debug/.build-id/xx/yyyy.debug` (xx/yyyy its GNU build-id; the build stops if an object has
  no build-id or two share one). The packaged objects are stripped exactly as before (no debuglink).
* `tools/deb/check-system.sh`: checks the fields; each stripped object keeps its build-id and has exactly
  one debug file with the same id and a symbol table; no orphan debug files; libc.so.6's debug file has
  `.debug_line`; host gdb with only the debug directory shows a source line for `DBG_FUNC` (default getenv).
* `tools/deb/root-system.sh`: seven packages installed and purged.
* `tools/deb/dbg-system.sh` + `dbg-cmds.sh` (new, NOT RUN yet): trixie root with gdb, libniji's libc6 and
  libc-bin swapped in, gdb `info line` before/after installing libc6-dbg, and a stop in a running /usr/bin/true.

## Measured (mei, job sys-dbg-package-pack3, products of sys-dbg-package-build1)

* Packaging the same products twice: identical .debs (all seven).
* The six existing packages are byte-identical to the ones the pre-story system.sh makes from the same
  products (libc6 included, so the layout contracts are untouched).
* libc6-dbg: one debug file per object (9 objects; build-ids present on every product: gcc passes --build-id).
* check-system.sh (before the fix in the last commit): 2 FAIL. (1) its own object list picked up libc6-dev's
  `libc.so` script; fixed (`*.so.*`); rerun on the final head (job sys-dbg-package-pack4): only (2) fails. (2) **the real gap: gdb shows no source line for any libc
  function.** The release libc.so.6's DWARF holds compile units only for core, alloc and compiler-builtins
  (the precompiled standard library); libniji's own crates contribute none. The Rust archive member
  (target/product/libc-so-input.a, the fat-LTO cgu) does carry `.debug_line`, the profile is `release`
  (`debug = "line-tables-only"`), the product rustflags are `-Cllvm-args=-align-all-functions=6` and two
  `--remap-path-prefix`; nothing in xtask strips debug from libc. Cause NOT found.

## Next step

1. Find why the niji crates emit no DWARF in the release product (start: does `cargo rustc --profile release
   --crate-type staticlib` of crates/core apply `debug` to the workspace crate; compare `readelf -wi` CU names
   of the archive member with the linked libc.so.6; check the hot-text grouping relink). Fix in xtask/so.rs or
   Cargo.toml; the stripped libc6 must stay byte-identical (strip removes the DWARF; check with the pre-story
   system.sh as pack.sh did).
2. Rerun: build, `check-system.sh` (expect PASS), `root-system.sh`, `dbg-system.sh <debs> trixie` with
   `KEYRING=~/keyrings/debian-archive-keyring.gpg` (mei has no Debian keyring in /usr/share/keyrings), plants
   (e.g. drop `--only-keep-debug`, drop the Provides line, skip one object), the coexist build, and
   `tools/deb/parity.sh` (gap 12, `gaps.tsv:5`, should go).
3. docs/install-packages.md and docs/RELEASE-NOTES.md: add libc6-dbg to the system-package table.

Job scripts: `~/work/SYS-DBG-PACKAGE/pack.sh` on mei (old system.sh in `old-system.sh` there).
