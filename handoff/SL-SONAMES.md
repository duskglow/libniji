# SL-SONAMES: handoff (plan/system-libc.md §6 item 5)

**State.** Branch `task/sl-sonames-2` (continues `task/sl-sonames`, from `origin/next` 2d5625a3e); HEAD in
handoff/reports/SL-SONAMES-2.json. Role mei:~/w/s3. Work complete; see the verdicts below.
Decision: `records/pending/SL-SONAMES-1.toml` (D-NEW, series D-27, amends D-27.9; CI assigns the id).

## What was done (SL-SONAMES)
* `xtask/src/placeholders.rs`: `LIBRARIES` + `libresolv.so.2`, `libBrokenLocale.so.1`; `ARCHIVES` + `libresolv.a`
  (moved from so.rs step 6), `libg.a`, `libmcheck.a`, `libpthread_nonshared.a`; `OVERRIDES` (libBrokenLocale's
  `__ctype_get_mb_cur_max`, the one name defined over libc.so.6's); `FORWARDED` (libresolv): generated bodies =
  `jmp __liella_fwd_<name>@PLT`; the 3 data objects are own objects of versions.tsv size, filled by a constructor from
  libc.a's object (size via `nm -S`); `libBrokenLocale.a` from the same body file; `DEV_LINKS` libresolv.so,
  libBrokenLocale.so.
* `xtask/src/so.rs`: libc.so.6 exports `__liella_fwd_<name>@LIELLA_PRIVATE` (67) as `--defsym` aliases.
* `crates/libc-so/placeholders/libBrokenLocale.c`: MB_CUR_MAX width n -> `n >= 6 ? 4 : (n + 1) / 2`.
* `tools/deb/build.sh`: installs the two libraries, the new archives and dev links; libm.so stays a link (no libmvec,
  so no linker script needed).

## What SL-SONAMES-2 did
* Cause of the red `sl-sonames-plants` (3 SURVIVED): (1) `resolv-dev-link-missing` and `empty-archive-missing`
  survived because `cargo xtask so` left the previous build's `libresolv.so` / `libresolv.a` in target/liella, so
  `-lresolv` still found them (a real defect: a product dropped from the tables lingered in any tree built before).
  Fix: `placeholders::build` keeps a manifest `target/liella/.placeholders` (`MANIFEST`, `product_names()`,
  `remove_stale()`) and removes, at the start of each build, names the last manifest lists and the tables no longer
  do; unit test `a_product_dropped_from_the_tables_is_removed`. The deb script installs named files only, so the
  manifest is not shipped. (2) `brokenlocale-halving` survived because only widths 1 and 6 were tested: the test now
  also runs ko_KR.EUC-KR, ja_JP.EUC-JP, zh_CN.GB18030 (widths 2, 3, 4; host `localedef` into target/test-locales,
  skip printed without it) statically: plain `1 2 3 4`, `-lBrokenLocale` `1 1 2 2` (probe-settled values).
* The red `sl-sonames-gate` was clippy `unwrap_used` in the new test, already fixed by SL-SONAMES's f7de9c193.
* Plants (handoff/SL-SONAMES-plants.txt, 7): the 4 earlier ones plus `brokenlocale-round-up`,
  `brokenlocale-cap-from-4`, `stale-product-kept`.

## Verdicts (mei)
* `sl-sonames-2-check`: clippy, static, `cargo test -p xtask placeholders` (9), `cargo xtask so`, sonames_products
  (4/4, the wide-locale branch ran) all rc 0.
* `sl-sonames-2-plants` (`cargo xtask plants`, 7 plants): 7 CAUGHT, 0 SURVIVED, 0 WARN (each by its named test;
  results in handoff/SL-SONAMES-plants.txt).
* `sl-sonames-2-gate` (`cargo xtask affected --run --base origin/next`): the tool escalates an xtask-source change
  to FULL itself (default tier is direct); left running at hand-back for the dispatcher (clippy-workspace ok).
  Re-attach: `~/bin/jobwait sl-sonames-2-gate` on mei; log ~/logs/sl-sonames-2-gate.log.

## NOT VERIFIED / open
* **No dynamic Liella program can run yet**: the start-path loader refuses any DT_NEEDED soname
  (`crates/boundary/src/ld.rs`; DL-8, plan item 1). The dynamic tests link and check NEEDED and versioned bindings,
  and run automatically once DL-8 lands (skip printed). libresolv's forwarders and data-copy constructor are not
  run-verified.
* INFORMATIONAL: libc.a's `__p_type_syms` is 1080 bytes (45 entries); versions.tsv gives 1104 (46). libresolv.so.2's
  copy is 1104 bytes, the last 24 zero.
* A tree built before the manifest existed keeps any already-stale file until it is cleaned once (only names a
  manifest lists are removed, never guessed).

## Next step
If `sl-sonames-2-gate` is red: run only the failing tests (direct), fix, hand back. Otherwise none: it lands. Follow-up when DL-8 lands: the dynamic runs in sonames_products start running;
check they pass. Read: D-27.9, D-16.6, records/pending/SL-SONAMES-1.toml.
