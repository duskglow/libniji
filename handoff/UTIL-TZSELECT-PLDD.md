# UTIL-TZSELECT-PLDD: tzselect, pldd and the /usr/bin/ld.so link (GitHub #14, the rest)

**State (leg 3, hand-back).** Branch `task/util-tzselect-pldd` from origin/next `88c0518f6`, role keke `~/w/n2`.
Head: see the report json (`handoff/reports/UTIL-TZSELECT-PLDD.json`). Records: **D-31.34** (tzselect: -c order, -n,
missing tables), **D-31.35** (pldd mechanism). **COMPAT-43** reserved and UNUSED (the three differences rows give
no-compat reasons).

## What the branch does
* `csys/util/tzselect.c` (D-31.34): libm-free C front end beside zic/zdump; reads `$TZDIR` (default
  `/usr/share/zoneinfo`) `iso3166.tab` and `zone1970.tab`; `--help`, `--version`, `-c COORD` (ISO 6709), `-n LIMIT`,
  the interactive menus, clock lines, the confirmation; end of input at every prompt exits 1.
* `csys/util/pldd.c` (D-31.35): PTRACE_SEIZE + INTERRUPT of every task, auxv AT_PHDR/AT_PHNUM, PT_DYNAMIC ->
  DT_DEBUG -> r_debug -> link_map, reads through `/proc/PID/mem`, bounded (65536 entries, PATH_MAX names).
  ELFCLASS32 layout by the executable's class: NOT VERIFIED (no 32-bit target on the host).
* Packaging: manifest rows libc-bin `usr/bin/tzselect`, `usr/bin/pldd`, `usr/bin/ld.so` -> `../lib64/ld-linux-x86-64.so.2`;
  coexist niji6 `usr/lib/niji/usr/bin/ld.so`. docs/install-packages.md, release-notes fragment, three differences rows.
* Tests: `xtask/tests/tzselect_pldd.rs` (8 tests: sessions vs the reference, help/version, missing tables, -c order vs an
  in-test haversine, end of input at every prompt, pldd vs the reference incl. dlopen'd objects and a second thread,
  pldd options, manifest link in both flavours). `tools/deb/root-system.sh`: `/usr/bin/ld.so --version` equals the
  loader path's, `/usr/bin/ld.so --library-path` runs hello, `tzselect -c +4852+00220` picks Europe/Paris, pldd lists a
  waiting tzselect's libc.so.6 (PID namespace with its own /proc).

## Results (keke)
* `util-tzselect-pldd-t2`: tzselect_pldd 8 passed / 0 failed (head 1fd1901ea; later commits change only root-system.sh,
  the plants file and two clippy lints in the test).
* `util-tzselect-pldd-rootsys1`: build.sh --flavour system + check-system.sh + root-system.sh on 33097e147:
  `root: RESULT pass=25 fail=0`, the four new checks PASS.
* `util-tzselect-pldd-plants` (handoff/UTIL-TZSELECT-PLDD-plants.txt): 3 plants, 3 CAUGHT, 0 WARN
  (c-order-reversed, pldd-no-dlopened, manifest-no-ld-so).
* `util-tzselect-pldd-gate` (direct tier, bfe269a9a): every step ok (products, xtask tests on the products, niji,
  niji-boundary, niji-math, niji-rt, layout-check, static; peak 5.1G) except one test in "xtask tests that run no
  products", which runs beside the products build: `so::tests::a_product_build_leaves_no_earlier_product_for_the_manifest_lint`
  reads the real target/niji and saw locale-sources being rewritten (SUPPORTED written 14:48:16, inside the test's
  window). `util-tzselect-pldd-xtbin` (`cargo test -p xtask --bin xtask`, alone, same tree): 657 passed / 0 failed.
  INFORMATIONAL: that test should be classed as using the products.

## Probe-settled behaviour (leg 1 and 2 replays; reference = the installed tzselect/pldd)
Prompts on stderr, the TZ value on stdout; "\n" on stdout at end of input at a menu; no try line after
`extra: unknown argument`; byte-order collation folds ASCII case for the country list; no clock lines when the TZ's
offset has seconds; `pldd -5` is `invalid option -- '5'`, status 64; pldd messages prefixed with argv[0]; a static
target gives status 1 and no output.

## Open
* RULING NEEDED: `tzselect -c` orders by great-circle distance (no documented rule to reproduce); recommend keep.
* Challenge to the brief: tzselect is a separate C front end like zic and zdump, not an argv[0] command of `niji-tz`.

## Next step
None on this branch beyond landing.
