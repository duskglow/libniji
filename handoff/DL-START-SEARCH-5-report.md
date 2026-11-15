# DL-START-SEARCH-5 report

Branch `task/dl-start-search-5` (on -4, head fe720ac08 inherited). Role keke:~/w/n1. Brief ~/state/briefs/DL-START-SEARCH-5.md.
The -4 run ended in a CI CONFLICT (crates/boundary/src/ld/dlopen.rs, merge with origin/next). Ruling #5909: keep
D-27.16; this story changes sonames_products to run its dynamic programs with LD_LIBRARY_PATH.

## Step 0 (inherited head fe720ac08)
* dl-start-search-5-lint0: `cargo xtask clippy && cargo xtask static`, rc 0.

## Commits
* 20ed5211d Merge origin/next. One conflict, the `use super::{...}` list of crates/boundary/src/ld/dlopen.rs: this
  branch replaced `open_whole` with the shared `read_cache`; next added `report_nonshared` (D-27.29). Resolution keeps
  both (`read_cache`, `report_nonshared`; `open_whole` is no longer used in dlopen.rs but stays in ld.rs, where
  `read_cache` and the program open use it). ld.rs merged without conflict: next's nonshared-report loop runs over
  `objs[..n]`, which the start-time search fills.
* 2cfd3024c sonames_products: dynamic programs run with `LD_LIBRARY_PATH=<target/liella>`; the `-Wl,-rpath` and the
  `NO_SEARCH` skip are gone (`build_run_with` returns `String`, so a dynamic run that cannot start fails the test).
  Why RUNPATH was wrong: `libc.so.6` is pinned (R14-3 (a), D-27.16): only `<prefix>/<libdir>` (an unprefixed build:
  `/lib`), after LD_LIBRARY_PATH outside secure mode; never RPATH/RUNPATH/cache.
* ba2ef4bfa handoff/DL-START-SEARCH-5-plants.txt: the -4 seven plus `pinned-ignores-library-path` (crates/ld/src/search.rs:
  the pinned branch skips LD_LIBRARY_PATH), caught by sonames_products.

## Jobs (keke)
* dl-start-search-5-gate (head ba2ef4bfa): clippy all targets/all features, static, `affected --base origin/next --run`:
  rc 0; 5 suites ok, 139 tests, 0 failed, SKIPPED lines 0; sonames_products and ldso_search selected; peak 1.6G.
* dl-start-search-5-plants (`cargo xtask plants`, RAM copy): rc 0, 8 plants: 8 CAUGHT, 0 SURVIVED, 0 WARN-RC-ONLY,
  0 WARN-NO-MSG; `pinned-ignores-library-path` caught by sonames_brokenlocale_mb_cur_max (the dynamic runs now
  execute). One WARN-DEV-SHM line (copy under /dev/shm; no plant here mounts its own /dev).
* dl-start-search-5-startup (`cargo xtask test`, then `LIELLA_FULL_PROFILE=1 cargo xtask so` and
  `LIELLA_FULL_PROFILE=1 cargo test -p liella --test hot_text_products`): rc 0. xtask test 6621/6621 and 5229/5229
  passed; full-profile hot_text_products 26 passed, including both localtime_r page-fault bound tests (41 > 40 from
  the -3 CI run did not reproduce on keke, Zen 1, for this merged head). Peak 12G.
* Job head: ba2ef4bfa (code); later commits on the branch touch handoff/ only.

## Left as is
* tools/deb/inside.sh and root-system.sh still have WAIT branches for "the library search is not available in this
  build"; they turn into PASS by themselves (SL-FLAVOUR's next step, see handoff/DL-START-SEARCH.md).
* The p_path surplus (+540 instructions over base, -4 report "The cold move") still needs a layout-aware design for
  ld.so text; not attempted here.
