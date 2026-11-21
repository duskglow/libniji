# SL-SYMBOLS: libc6's dpkg symbols and shlibs (plan/system-libc.md §3.2, §6 item 3; ruling R2)

**State:** branch `task/sl-symbols` (from origin/next `8261e29b2`); head in handoff/reports/SL-SYMBOLS.json.
Role mei `~/w/s4`. Pending record `records/pending/SL-SYMBOLS-1.toml` (D-NEW, series D-X, amends D-X.106; CI
assigns the id). Plants `handoff/SL-SYMBOLS-plants.txt`.

## What it does

* `tools/deb/symbols.py files` (run by `tools/deb/system.sh` after stripping, before libc6's control) reads every
  shared library in the staged libc6 (`readelf -VW`, `--dyn-syms`) and writes `DEBIAN/symbols` + `DEBIAN/shlibs`:
  * one `<soname> libc6 #MINVER#` block per library (10 today; libresolv/libBrokenLocale join automatically when
    SL-SONAMES lands: the generator takes whatever libc6 ships), `* Build-Depends-Package: libc6-dev`;
  * `name@GLIBC_2.x` -> `2.x`; a node above `--abi` (GLIBC_2.41 sched_getattr/sched_setattr) -> the build's
    upstream version (RULING NEEDED below); version-node symbols (`GLIBC_2.3@GLIBC_2.3`) included;
  * `LIELLA_NATIVE_<n>` -> template `| liella-native-abi (>= <n>)`; libc6 now `Provides: liella-native-abi (= <n>)`
    (`symbols.py native-level`);
  * LIELLA_PRIVATE -> one `(symver)LIELLA_PRIVATE <upstream>` pattern; no GLIBC_PRIVATE (exporting it is an error);
  * shlibs `<name> <n> libc6 (>= 2.39)` per library.
  * Cross-check: exports must be versions.tsv rows (default required, compat allowed) + native.tsv functions; a
    missing default row is allowed only if docs/spec-symbol-gaps.tsv lists it missing in the .so.
* `tools/deb/check-symbols.sh DIR`: line rules (awk), Provides, shlibs, `dpkg-gensymbols -c4`, `dpkg-shlibdeps` on
  smoke/sym-base.c (own `_start`), smoke/hello.c (Scrt1), smoke/sym-native.c, `dpkg --compare-versions`.
  check-system.sh's libc6 control-member list gains `./shlibs ./symbols`. build.sh passes `spec` (the built tree's).

## Results (mei)

* `sl-symbols-build-2` rc 0: system build of 8261e29b2 (products reused by every later job via `--products`).
* `sl-symbols-deb-7` rc 0 (code head 4bad63324; later commits touch only the record text and plant results):
  `check: PASS`; `check-symbols: PASS (4131 symbol lines, 10 libraries, version 2.39+liella20261115.1-1)`;
  dpkg-shlibdeps: sym-base `libc6 (>= 2.2.5)`, hello `libc6 (>= 2.34)`, sym-native
  `libc6 (>= 2.34), liella-native-abi (>= 1)`; root-system `pass=8 fail=0 wait=1` (the DL-8 WAIT, unchanged).
* `sl-symbols-plants` rc 0: 5 CAUGHT (drop a symbol, min version = abi, `<=` -> `<` boundary, a GLIBC_PRIVATE line,
  native template lost), 0 WARN.
* `sl-symbols-gate-2`: clippy, static, direct tier (see report json). Superseded: `sl-symbols-build` (rc 2: role
  is not a git tree, needs LIELLA_DEB_COMMIT/SOURCE_DATE_EPOCH), `sl-symbols-deb-1..6` (development iterations),
  `sl-symbols-gate` (static: bare `|` in the record body; fixed).

## RULING NEEDED

* GLIBC nodes above the packages' ABI level (GLIBC_2.41 sched_[gs]etattr at abi 2.39): the brief's rule `2.x` would
  give `libc6 (>= 2.41)`, which Liella's own `2.39+liella...` never meets (rebuilt util-linux, Essential, would not
  install). Implemented: the build's upstream version (`2.39+liella20261115.1`). Recommend keeping it; when R3 moves
  `<abi>` to 2.41 the rule gives `2.41` again with no change.

## INFORMATIONAL

* 7 default-binding spec exports are not built and so are not in the file (all in docs/spec-symbol-gaps.tsv): ld.so
  `_dl_mcount`, `_r_debug`; libc `__libc_sa_len`, `_dl_mcount_wrapper`, `_dl_mcount_wrapper_check`, `bindresvport`,
  `dlmopen@GLIBC_2.34`. They appear automatically once exported.
* No lintian here (mei/keke lack it); SL-FLAVOUR's 10 `no-shlibs` errors should clear: NOT VERIFIED.
* Rule slips (mine): one empty Bash heredoc (no effect); first commit's message says "Zen 1 build host" for mei,
  whose role lives under /mnt/zram (a Skylake-SP host by the brief's rule): the class there is wrong.

## Next step

None for this story. SL-SWAP-SMOKE / Aqours stage 1 consume the file; rerun `tools/deb/check-symbols.sh` on each
system build (it is not wired into a cargo test).
