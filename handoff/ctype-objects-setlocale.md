# CTYPE-OBJECTS-SETLOCALE: the six ctype objects follow the global locale

State: branch `task/ctype-objects-setlocale` (from origin/next 0495c4684), HEAD: the last commit on the branch
(leg 1 ended at the "WIP: record the setlocale probe..." commit). Role keke:~/w/r1. Decision in force: D-5.13
(amended in `records/decisions/D-5/D-5.13.toml`).

## Done (leg 1)

* Reference probe (black box, keke, glibc 2.39): table in `spec/behaviour/ctype-objects.md` ("setlocale probe").
  The six objects follow the global LC_CTYPE only (LC_ALL, LC_CTYPE, ""), not LC_NUMERIC and not uselocale; 16-bit =
  `*__ctype_*_loc()`, 32-bit = `nl_langinfo` items 5/16/15 (CLASS32/TOLOWER32/TOUPPER32, by code point), "C" = the
  start pointers. Probe sources: scratch `~/work/ctype-objects-setlocale/probe.{c,sh}` on keke.
* Code: `xtask/src/compat_objects.rs` emits hidden `niji_compat_ctype_store(words[6])` (stores through the GOT of
  `niji_compat_<name>_store_ref` bound `.symver` to `<name>@GLIBC_2.2.5`; order `CTYPE_STORE_ORDER`);
  `csys/src/adapter/runtime.c` section 4 + `niji_csys.h`: `niji_csys_compat_ctype_store`, weak hook (0 in libc.a);
  `crates/boundary/src/ffi.rs` declares it as `compat_ctype_store`; `crates/boundary/src/locale.rs` `publish_global`
  (cold, only on a global change) calls it with `compat_ctype_words`; `crates/core/src/locale/ctype.rs` gains
  `toupper32_table`/`tolower32_table`.
* Test: `conformance/c/s05_ctype_legacy_objects.c` `setlocale_rows()` (de ISO-8859-1 via host localedef + LOCPATH).
* Docs: differences row D-5.13 shrunk to two residuals (pristine-start addresses; built-in C.UTF-8 32-bit tables).

## NOT yet done / exact next steps

1. `csys/src/machine/compat_aliases.S` is @generated and NOT yet regenerated in the worktree (no local cargo): on keke
   the suite runs `cargo xtask compat-gen` first; copy the regenerated file back
   (`ssh keke cat ~/w/r1/csys/src/machine/compat_aliases.S > <worktree>/csys/src/machine/compat_aliases.S`), commit it.
   Without it `compat-gen --check` (static) is red.
2. Re-sync, rerun `sh ~/work/ctype-objects-setlocale/suite.sh` as job `ctype-objects-setlocale-suite-3` (kind test).
   suite-2 result: reference passed; libniji failed only at "compiling and loading de_DE.ISO-8859-1" (missing
   `de_DE.iso88591` symlink, fixed in the last commit). If a `follows` row fails on libniji, check whether libniji's
   `nl_langinfo(5/15/16)` returns the item pointer.
3. Plants: add to `docs/plants.md` (a) drop the `crate::ffi::compat_ctype_store(...)` call in `publish_global`;
   (b) `compat_ctype_words` returning `lower` for upper32. cmd: `cargo xtask affected --only
   conformance:s05_ctype_legacy_objects --with-exports --run`, msg `s05_ctype_legacy_objects`. Results to
   `handoff/CTYPE-OBJECTS-SETLOCALE-plants.txt`.
4. Counts: empty main start-up (instructions:u) and isalpha/toupper per call, base vs head (brief step 2); the change
   touches no start-up or ctype hot code (publish_global is cold), expect unchanged.
5. Direct gate (`cargo xtask affected --run` under `~/bin/gate 16G`), clippy (foreground), `cargo xtask static`,
   start-up checks (`cargo xtask test`; `NIJI_FULL_PROFILE=1 cargo xtask so && NIJI_FULL_PROFILE=1 cargo test -p niji
   --test hot_text_products`), precheck, report json `handoff/reports/CTYPE-OBJECTS-SETLOCALE.json`, handback-check.

## Jobs so far (keke)

* `ctype-objects-setlocale-suite`: SUPERSEDED (xtask borrow error, fixed).
* `ctype-objects-setlocale-suite-2`: SUPERSEDED (test locale symlink missing, fixed).

## Open

* RULING NEEDED candidate: give the built-in "C.UTF-8" its own 32-bit tables (3 KiB rodata) to remove the last part
  of D-5.13; recommend a follow-up story.
