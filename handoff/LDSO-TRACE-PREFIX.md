# LDSO-TRACE-PREFIX: the interpreter trace of a prefixed (coexist) build

State: branch `task/ldso-trace-prefix`, base origin/next 852c1b495. Role mei:~/w/s2. Brief:
~/state/briefs/LDSO-TRACE-PREFIX.md. Decisions in force: D-X.123 (LAYOUT-LDSO-ORDER, the interpreter's traced
layout); no new decision record (tooling fix; the behaviour of the products does not change).

## Cause
`tools/hottext/ldso_trace.sh` pointed only `/etc/ld.so.preload` at an absent file. An interpreter built with
`NIJI_PREFIX=/usr/lib/niji` (the coexist flavour, tools/deb/build.sh) opens `<prefix>/etc/ld.so.preload`, a host
path trace_gdb.py refuses: RELEASE-SNAPSHOT-1118B's debs job, mei `~/logs/release-snapshot-1118b-debs-1.log`
line 260 (`trace_gdb: the program read host files the tree does not supply (canon_host.sh):
/usr/lib/niji/etc/ld.so.preload`). That log is the reproduction; the plant `ltp-script-etc-only` (the old
`/etc`-only redirect) reproduces it again on this head.

## Fix
* `crates/ld/src/search.rs`: `PRELOAD_FILE`, `CACHE_FILE`, `CONFIG_FILES` (every configuration file the
  interpreter opens below its prefix; a grep of the loader finds no other) and `DEFAULT_PREFIX`. The two open
  sites in `crates/boundary/src/ld.rs` use the constants (the same bytes; no code change).
* `xtask/src/so.rs`: `ldso_config_paths(prefix)` joins the prefix with each of `CONFIG_FILES` the way
  `search::prefixed` does; the build's prefix is `NIJI_PREFIX` of the environment that the product build
  hands to cargo (the setting `option_env!` reads), else `DEFAULT_PREFIX`. `ldso_trace` passes each path as
  `--config`; the script redirects each to an absent `@run/loader-config.<k>` and requires at least one.
  trace_gdb.py and canon_host.sh are unchanged: a read of an unredirected host path is still an error.
* Refactor so the build and the check share code: `LdLink` (the interpreter's link inputs and link) and
  `ldso_trace` (the trace run), split out of `run_ld` / `ldso_layout`.
* `ld.so.conf` is read by ldconfig only, not by the interpreter; hwcaps subdirectories and `<prefix>/lib` are
  library directories, reached in the trace by neither program (libc by path, the three libraries by RUNPATH).

## Check (step 4)
`cargo xtask ldso-trace --prefix <dir>` compiles the interpreter with that prefix (its own cargo target directory,
`target/product/ld-prefix`), links it ungrouped, traces it as the build does, and fails on a host read or when its
names differ from the products' own trace. The products test
`init_prefixed_interpreter_trace_reads_no_host_file_and_matches_the_products_trace` (crates/core/tests/
hot_text_products.rs) runs it with the coexist prefix; it names `tools/hottext/ldso_trace.sh`, so a change there
selects it, and xtask/src/so.rs and the loader crates escalate to FULL, which runs it. 61 s warm on the Skylake-SP
host (first run includes the interpreter crates' compile). Unit test `ldso_trace_redirects_the_paths_the_loader_opens`
pins the join against `search::prefixed`.

## Results (code head 750abb9a6; all jobs on mei, Skylake-SP)
* Same names in both flavours (step 3): the prefixed trace gives the system trace's lists exactly (fast: true
  98 names, libs3 107, ldso-trace-prefix-check2 on df4f6e940, whose code equals the final head's apart from
  an affected.rs test). Release, in the debs: both flavours 92 / 99, and `ldso.true.names`, `ldso.libs3.names`,
  `ldso-text.list` byte-identical between the system and coexist builds. The prefix changes only the bytes of
  the path the loader builds, not which functions run.
* `ldso-trace-prefix-debs2` (750abb9a6, NIJI_DEB_COMMIT and SOURCE_DATE_EPOCH of that commit): rc 0. System
  flavour: libc6, libc6-dbg, libc6-dev, libc-bin, libc-dev-bin, libc-l10n, locales .debs. Coexist flavour
  (trace with prefix /usr/lib/niji): niji6, niji6-dev .debs. Output `~/work/ldso-trace-prefix/debs-2/`.
  (`debs1` failed before building: build.sh needs those two variables outside a git tree.)
* `ldso-trace-prefix-plants1` (handoff/LDSO-TRACE-PREFIX-plants.txt, /mnt/zram/claude/ldso-trace-prefix/plants-1):
  3 CAUGHT, 0 SURVIVED, 0 WARN (the /etc-only script, the default prefix passed to the script, a doubled `/`).
* `ldso-trace-prefix-gate1` (48d51ae9c; affected escalated to FULL, so.rs changed): 6826 passed, 1 failed:
  `affected::tests::direct_tier_hottext_scripts_reached_by_listing_select_the_xtask_tests`. It expected no
  test target for a tools/hottext script; hot_text_products now runs ldso_trace.sh (which sources runner.sh and
  runs canon_host.sh), so selecting it is correct and is the tier the brief asks for. Expectation updated
  (750abb9a6). Its chained release steps did not run.
* `ldso-trace-prefix-rerun1` (750abb9a6): clippy -D warnings, static, `affected::` unit tests 96 passed;
  `NIJI_FULL_PROFILE=1` so + whole hot_text_products 28 passed (the new test included, release). rc 0, peak 3.0G.
* Start-up code: crates/boundary/src/ld.rs changes only literals to constants with the same bytes.

## Next step
Land (train). Snapshot 18.2 can rebuild the coexist debs after this lands.
