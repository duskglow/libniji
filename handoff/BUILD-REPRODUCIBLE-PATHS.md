# BUILD-REPRODUCIBLE-PATHS: handoff (leg 1 of the run; refresh)

Branch `task/build-reproducible-paths`, HEAD f9d03aedc (code commit, NOT YET COMPILED OR TESTED). Role mei:~/w/s1
(the role is physically `/mnt/zram/claude/w/s1`; `~/w/s1` is the logical spelling through a symlink).
Base origin/next 9d12e7ee3. Wall-time deadline 2026-11-23T04:40Z.

## Item 1: cause found and fixed (code committed, unverified)

* Before (role build `build-reproducible-paths-so1`, head 9d12e7ee3, release traced): a scan of all 1313 files of
  target/niji (bench/ excluded) for the home and zram paths found exactly the 19 files of CI-PRODUCTS-ONCE plus
  `.hot-lists`. In each ELF the ONLY leak is `DW_AT_comp_dir` / line-table directory 0 = the logical root
  (`~/w/s1`); source names were already `/niji/...`.
* Cause (logging CC wrapper, job `build-reproducible-paths-ccdiag`, ~/work/build-reproducible-paths/cc.log on mei):
  xtask's root = getcwd = canonical `/mnt/zram/claude/w/s1`; the C compiles run with `PWD=~/w/s1`; GCC takes
  the compile directory from `$PWD` when it names the cwd; the map covered only the canonical root. By hand:
  `-fdebug-prefix-map=<logical>=/niji` maps comp_dir; a trailing-slash map does not.
* Fix (f9d03aedc): `align::root_spellings(root, pwd)` (given, canonical, the `$PWD` ancestor that is the root);
  `so::release_debug_cflags` -> `so::debug_prefix_flags(root, pwd)` maps each; `remap::needles` uses the same
  spellings and `remap::check_dir` now scans every top-level file of the products (before: only libc/libm/crt +
  placeholder lists, so the utilities/libmvec/BrokenLocale leak was never seen). `.hot-lists` line 1 is now
  relative to the products (`../product/release`, `hotlists::relative_dir`); `marked_dir` joins a relative line
  to the products dir, absolute (old) markers still work. Rust flags unchanged (rustc uses getcwd).
* Test: `remap::tests::c_debug_info_is_the_same_in_two_tree_paths` (two trees, symlinked, top and subdir PWD).
  Test changed: `hotlists::tests::marker_names_the_lists_and_hashes_them` (relative line, old absolute form).

## Item 2: partial

* `bench/c/cbench_counters.h`: user-mode instructions (event 4) joined the pinned group (leader event 0). NOT YET
  CHECKED that the 4-event group schedules on both hosts (if not, every column reads -1: check a ctr line).
* The gated column is event 0 (`instructions`, user+kernel; `bench.rs` `check_instr_limits` reads `v[0]`, median
  of 3 runs, limits `bench/instr_limits.tsv`, tolerance max(2 instr, 2%)). Hypotheses for its spread, unmeasured:
  kernel instructions (interrupts/preemption on a loaded host count in a per-task counter with exclude_kernel=0),
  and stack ASLR moving stack-buffer alignment (user instructions of string rows). The rows use a fixed op count
  (7*iters), BENCH_ENV is fixed (PATH, HOME), so ops/env are not the cause.
* Baseline job `build-reproducible-paths-cnt-before1` (`cargo xtask bench --counters-only --only
  cbench_memref,cbench_hot,cbench_startup` on the old head) was running at refresh; its table is in its log.

## Exact next steps

1. Sync (`sync_role.sh --story BUILD-REPRODUCIBLE-PATHS --base origin/next <wt> mei s1`), then on s1 one job:
   `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo xtask static`, `cargo test -p xtask
   -- remap:: hotlists::`. Fix anything.
2. `NIJI_FULL_PROFILE=1 cargo xtask so` on s1 (job); it must pass its layout contract (now scanning all files with
   the logical root as a needle). Rescan target/niji for `/home/claude|/mnt/zram` (expect none).
3. Second path on mei: copy the tree (no target/, no .git) to e.g. ~/work/build-reproducible-paths/t/second/tree-b,
   build there (`cd` into it; job), `diff` the two `target/niji/.products-manifest` (expect identical except
   nothing: head/tag equal). Cross host: same on keke (Zen 1) in ~/work/build-reproducible-paths/, diff manifests.
4. gdb backtrace through libc with the products (debug info usable: `set substitute-path /niji <tree>`).
5. Item 2: a repeat-count script in ~/work/build-reproducible-paths/ (5+ runs of the cbench binaries with
   CBENCH_COUNTERS=1, env PATH/HOME only), columns 0 and 4 per row, with and without `setarch -R`; decide; make the
   CI check skip rows whose measured spread exceeds tolerance (or count user-only) - likely RULING NEEDED if the
   gated column changes.
6. Plants in docs/plants.md format (scratch file), gate (`cargo xtask affected --run`), precheck, report json.
