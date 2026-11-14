# SL-SONAMES: handoff (plan/system-libc.md §6 item 5)

**State.** Branch `task/sl-sonames` (from `origin/next` 2d5625a3e), HEAD in handoff/reports/SL-SONAMES.json. Role mei:~/w/s4.
Stopped at the context checkpoint (~155k of 180k): code complete and pushed; verification jobs queued on the head, not yet seen.
Decision: `records/pending/SL-SONAMES-1.toml` (D-NEW, series D-27, amends D-27.9; CI assigns the id).

## What was done
* `xtask/src/placeholders.rs`: `LIBRARIES` + `libresolv.so.2`, `libBrokenLocale.so.1`; `ARCHIVES` + `libresolv.a`
  (moved from so.rs step 6), `libg.a`, `libmcheck.a`, `libpthread_nonshared.a`; `OVERRIDES` (libBrokenLocale's
  `__ctype_get_mb_cur_max`, the one name defined over libc.so.6's; plan() and product_errors() accept it);
  `FORWARDED` (libresolv): generated bodies = `jmp __liella_fwd_<name>@PLT` per function; the 3 data objects are
  own objects of versions.tsv size, filled by a constructor from libc.a's object (size via `nm -S`); `libBrokenLocale.a`
  built from the same body file; `DEV_LINKS` libresolv.so, libBrokenLocale.so in target/liella.
* `xtask/src/so.rs`: libc.so.6 exports `__liella_fwd_<name>@LIELLA_PRIVATE` (67) as `--defsym` aliases of libc.a's
  definitions (compat-only names: `liella_compat_<name>`); the plain names stay local. Probe (mei): `--defsym` aliases
  survive `--gc-sections`, but carry size 0 (hence `nm -S` on libc.a for the object sizes).
* `crates/libc-so/placeholders/libBrokenLocale.c`: MB_CUR_MAX width n -> `n >= 6 ? 4 : (n + 1) / 2`.
* `tools/deb/build.sh`: installs the two libraries, the new archives, libresolv.so/libBrokenLocale.so links; libm.so
  stays a link (no libmvec, so no linker script needed).
* Test `crates/core/tests/sonames_products.rs` (4 tests); plants `handoff/SL-SONAMES-plants.txt` (4).

## Probe-settled (reference, black-box, mei)
* `-lBrokenLocale` (dynamic and static): MB_CUR_MAX 1,2,3,4,6 -> 1,1,2,2,4 over 15 charsets (localedef into a private
  LOCPATH). The reference's libBrokenLocale.so.1 exports `__ctype_get_mb_cur_max@@GLIBC_2.2.5`; libBrokenLocale.a
  defines it. libg.a = one dummy member; libpthread_nonshared.a empty; libmcheck.a is an ELF object setting
  `__malloc_initialize_hook` (Liella ships an empty archive; its libc_malloc_debug has no hooks, D-27.9).

## NOT VERIFIED / open
* **No dynamic Liella program can run yet**: the start-path loader refuses any DT_NEEDED soname
  (`crates/boundary/src/ld.rs`, "library search is not available"; DL-8, plan item 1). The tests link dynamically and
  check NEEDED + versioned bindings, run only the static programs, and run the dynamic ones automatically once DL-8
  lands (skip printed). So libresolv's forwarders and the data-copy constructor are not run-verified.
* INFORMATIONAL: libc.a's `__p_type_syms` is 1080 bytes (45 entries); versions.tsv gives 1104 (46). libresolv.so.2's
  copy is 1104 bytes, the last 24 zero.
* Process lapse: one edit of tools/deb/build.sh used a python heredoc (the brief forbids heredocs).

## Next step
Read the jobs `sl-sonames-gate` (so + sonames_products + `cargo test -p xtask placeholders` + clippy + static) and
`sl-sonames-plants` on mei (`~/bin/jobwait sl-sonames-gate sl-sonames-plants`; logs ~/logs/<label>.log). Fix what they
find, then run the direct tier (`cargo xtask affected --run --base origin/next`), drop "WIP" in a final commit, hand back.
Read: D-27.9, D-16.6, records/pending/SL-SONAMES-1.toml.
