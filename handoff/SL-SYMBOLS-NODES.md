# SL-SYMBOLS-NODES: a dependency rule for every Liella version node; the symbols step in the gate

**State:** branch `task/sl-symbols-nodes` from origin/next `99f649ec6`; head in
handoff/reports/SL-SYMBOLS-NODES.json. Role keke `~/w/n1`. Pending record
`records/pending/SL-SYMBOLS-NODES-1.toml` (D-NEW, series D-X, amends D-X.109; CI assigns the id).
Plants `handoff/SL-SYMBOLS-NODES-plants.txt`.

## The fault (reproduced)

`sl-symbols-nodes-build-1` (rc 1): `tools/deb/build.sh --flavour system` of origin/next `99f649ec6`
stops at `symbols.py: no minimum version rule for node LIELLA_OWNER_1`. Same fault as UTIL-LDCONFIG-6 (#5940).

## What changed

* `tools/deb/symbols.py`: one rule for every Liella-only node `LIELLA_<KIND>_<n>` plus the table
  `KINDS` (NATIVE -> `liella-native-abi`, spec native.tsv; OWNER -> `liella-owner-abi`, spec
  "nondefault-libc": the product build checks the names (`take_owner_node`, D-27.29), here each
  export must be a non-default version in libc.so.6). Each symbol at such a node names the template
  `| <package> (>= <n>)`. `check_node` stops on any other node before anything else, with a message
  naming the node and where its rule goes. `--provides OUT` writes libc6's Provides (each package at
  its newest level); the `native-level` subcommand is gone (system.sh was its only caller).
* Why one rule plus a table, not OWNER beside NATIVE: both kinds mean "runs only on Liella" to dpkg,
  so the rule is the same; a new kind is one row; a separate virtual package per kind keeps the levels
  independent (LIELLA_OWNER_2 never reads as LIELLA_NATIVE_2).
* `tools/deb/system.sh`: Provides from `--provides`; `tools/deb/build.sh --symbols-only` (system
  flavour, `--products`) stops after libc6's symbols, shlibs and control.
* `tools/deb/check-symbols.sh`: general template rule `LIELLA_<KIND>_<n>` -> `liella-<kind>-abi`,
  Provides = every template package at its top level (and must name native and owner), a fourth
  dpkg-shlibdeps sample `smoke/sym-owner.c` (binds `atexit@LIELLA_OWNER_1` with `.symver`; the
  packages do not ship the opt-in's link-stub directory).
* The gap: `crates/core/tests/deb_symbols_products.rs` runs `build.sh --symbols-only` on the gate's
  `target/liella` and checks every Liella-node line's template and the Provides. `cargo xtask
  affected` selects it for any change to build.sh, system.sh, symbols.py (named as paths in code),
  and every FULL escalation (xtask/src/so.rs's version script, spec/abi, csys). Cost: the step is
  923 ms on keke plus the test's compile; it needs the products, which a gate selecting it builds.
* plan/system-libc.md §3.2 names the general rule and liella-owner-abi.

## Results (keke)

* `sl-symbols-nodes-deb-1` rc 0 (code head 4ff418430; later code commits touch only the test and
  the record): origin/next's products repackaged with this tools/deb: `Provides: liella-native-abi
  (= 1), liella-owner-abi (= 1)`; `check-symbols: PASS (4135 symbol lines, 10 libraries)`, which
  includes `dpkg-gensymbols -c4` clean; dpkg-shlibdeps sym-owner `libc6 (>= 2.34), liella-owner-abi
  (>= 1)` (the other three unchanged); `check: PASS (6 packages)`; symbols-only step 923 ms.
* `sl-symbols-nodes-plants` rc 0: 3 CAUGHT (drop the OWNER rule, OWNER on the wrong package,
  Provides without liella-owner-abi), 0 WARN.
* `sl-symbols-nodes-gate`: clippy, static, direct tier (see the report json).
* Superseded: `sl-symbols-nodes-build-1` (rc 1: the reproduction, by design), `sl-symbols-nodes-lint`
  (rc 1: static found a bare `|` in the record and the test's clause note; fixed), `sl-symbols-nodes-lint-2`
  (rc 0, older head).

## INFORMATIONAL

* The system libc6-dev does not ship the opt-in's link-stub directory (`liella-owner/`), so on a
  system install a program can opt in only with `.symver` as the sample does. Whether libc6-dev should
  ship it is a packaging decision for the SL stories (not in this brief).
* Rule slips (mine): two empty Bash heredocs (no effect, nothing written).

## Next step

None for this story. The snapshot's reference .debs and Aqours stage 1 can build from next once this lands.
