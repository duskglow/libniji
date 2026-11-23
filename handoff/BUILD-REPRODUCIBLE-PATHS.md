# BUILD-REPRODUCIBLE-PATHS: handoff (after leg 2 of run 1; refresh)

Branch `task/build-reproducible-paths`. Commits: f9d03aedc (item 1, path fix), 565671f9e (item 2, count fix), then
this note. Role mei:~/w/s1 (physically `/mnt/zram/claude/w/s1`, `~/w/s1` is the logical spelling through a symlink).
Base origin/next 9d12e7ee3. Wall-time deadline 2026-11-23T04:40Z. Scripts and data: mei and keke
`~/work/build-reproducible-paths/` (repcount.sh, spread.py, build-scan.sh, gdbprobe/run.sh, *.tsv, manifest-*).

## Item 1: path fix VERIFIED on mei (one host)

* f9d03aedc: every spelling of the root (given, canonical, the logical `$PWD` ancestor) is mapped to `/niji` for the
  C compiles (`so::debug_prefix_flags`, `align::root_spellings`); `remap::check_dir` scans every top-level product
  file; `.hot-lists` line 1 relative. Before: 19 utilities/small libraries + `.hot-lists` carried the logical root
  (only `DW_AT_comp_dir`; cause: GCC records `$PWD`).
* Verified (jobs `build-reproducible-paths-lint1` clippy ok, static ok after the handoff home-path fix,
  `-ut1` 11 xtask tests ok incl. `remap::tests::c_debug_info_is_the_same_in_two_tree_paths`, `-so2` product
  build in ~/w/s1: layout `PRODUCT-PATH-REMAP ok: 47 products`, scan of every file under target/niji (bench/
  excluded) for the home path, both root spellings and /mnt/zram: 0 hits; manifest copied to
  ~/work/build-reproducible-paths/manifest-s1, profile `release traced`).
* gdb (job `-gdb3`, s1 products, program linked with the products' loader and LD_LIBRARY_PATH, gdb
  `startup-with-shell off`): `bt` from a qsort comparator shows 15 libniji frames with file:line
  (`boundary/src/posix/s14_search.rs:119`, `core/src/search.rs:355`, ..., `qsort () at ...s14_search.rs:80`), compile
  dir `/niji/crates`; after `set substitute-path /niji <tree>` `list` prints the source. (RUNPATH is not used for
  libc.so.6 by design: differences row, COMPAT-37.)
* Cross host NOT DONE: keke build (job `build-reproducible-paths-kk-so`, tree in keke
  /dev/shm/claude/build-reproducible-paths/kk/tree-c, same .synced head 96f0f1b2a) passed its remap contract with 0
  hits, but it built profile `fast traced` (keke's environment), mei `release traced`: manifests differ in 94 lines
  for that reason only, not comparable. Find what picks the profile (grep `traced\|NIJI_FULL_PROFILE\|Profile::` in
  xtask/src/so.rs, and keke's env) and rebuild keke with the release profile.
* Second path on mei NOT BUILT yet: tree copied to /mnt/zram/claude/build-reproducible-paths/t/second/tree-b (head
  96f0f1b2a .synced; plain path, no symlink).

## Item 2: cause found, fixed, verified on mei

* Lead confirmed: event 4 (user instructions) was multiplexed (values 0.001 to 110 for one row); f9d03aedc pins it.
* Before (old head, 20 runs memref + 8 runs gated hot; files before-*.tsv): event 0 moves in every row; beyond
  max(2,2%) only memset_64k 6.7%, memcpy_64k 5.2% (ungated). After pinning (after-*.tsv, 233 rows incl. opt):
  user column exact except 28 rows; ASLR: strstr_hit_64/strstr_miss_64 (GATED) 71 or 135 user instr/call (90%),
  strcasestr_hit_64 3.7% (GATED); all exact under setarch -R. Kernel share: event 0 moves 0.1-0.3 instr/call in every
  row with ASLR off (interrupts in the task's user+kernel count).
* Fix 565671f9e: cbench_counters.h constructor sets ADDR_NO_RANDOMIZE and re-execs (/proc/self/cmdline, environ)
  when CBENCH_COUNTERS is on; bench.rs `--count-runs N` (default 3) and `check_count_spread` (COUNT-SPREAD failure
  for a limited row moving more than max(2, 2%) over the runs) + unit test
  `count_spread_fails_a_limited_row_that_moves_beyond_its_tolerance`.
* After fix (job `build-reproducible-paths-check2`, plain runs, 20 memref + 6 opt): clippy ok, 11 tests ok; user
  column exact on every memref row (strstr 0); gated rows beyond tolerance: 0. Not deterministic, ungated:
  gettext_language_steady 14.6% user, gettext_language_alternating 4.2% user, ngettext_found 0.5%, re_mt4_* (threads),
  memcpy_64k 7.3% / memset_64k 2.7% (kernel share). Not yet measured after the fix: cbench_hot (gated 32 rows), and the
  other gated benches (locale 8, malloc 7, stdio 4, time 3, patterns 2, unmeasured 3; startup 4 does not include the
  counter header).

## Exact next steps (about 2 h 50 min of wall time were left at 01:50Z)

1. Rebuild on keke with the release profile; diff its `.products-manifest` against manifest-s1 (expect identical).
   Build tree-b on mei (`build-scan.sh <tree-b> treeb`, job), diff manifests, run gdbprobe/run.sh against tree-b.
   Both builds were from head 96f0f1b2a: the s1 manifest is from that head too (rebuild s1 for the final head only if
   a product input changed: 565671f9e changes only bench/ and xtask).
2. The repeat test on the final head: `cargo xtask bench --counters-only --count-runs 10 --only
   cbench_memref,cbench_hot,cbench_opt,cbench_locale,cbench_malloc,cbench_stdio,cbench_time` (job, label
   `build-reproducible-paths-repeat`); it must end with no COUNT-SPREAD. cbench_hot takes ~66 s per run.
3. D-row: reserve with `python3 ~/src/liella-ops/bin/ops reserve --story BUILD-REPRODUCIBLE-PATHS D-X` for the count
   rule (counted runs ASLR off; COUNT-SPREAD), cite the measurements above; docs/benchmarks.md one paragraph.
4. Plants (scratch file, `cargo xtask plants`): drop the logical spelling in `align::root_spellings`
   (msg:c_debug_info_is_the_same), drop the `hi - lo > tol` check (msg:count_spread_fails), all-files scan in
   `remap::check_dir` (msg:check_dir_names). Gate `cargo xtask affected --run --base origin/next`, precheck, report
   json, handback-check.

## Not to redo

Cause analysis of both items; before/after counts; gdb on s1; s1 build and scan; the lint of 565671f9e.
