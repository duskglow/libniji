# PRODUCTS-HARNESS-SHARED (audit item M5): one link line and one filter for the products tests

State: branch `task/products-harness-shared`, cut from origin/next 0495c4684; head: see the report json.
Build role: shiki `~/w/sh2` (Haswell, 56 threads).

## What changed

`crates/core/tests/support/oracle.rs` now holds the pieces the products tests each carried a copy of:

* `root()`: the repository root (57 local copies removed; `use oracle::root;`).
* `build_pair(src)` -> `Pair`: one C source built against the host C library (`build(out, false)`) or
  statically against the products (`build(out, true)`), with per-test options: `flags` (both sides),
  `host_flags`, `host_libs`, `niji_flags`, `archives` (before `libc.a`), `niji_libs` (after it, `-L
  target/niji`), `crt`, `libm()` (the math tests' set), `quiet()`. `Pair::command` gives the argv;
  each migrated test's argv is the one it built before, argument for argument, so the oracle stamps
  (keyed on the command line) stay valid and nothing rebuilds.
* `with_headers(text, markers)` and `without(lines, list, who)`: the header-keyed known-differences
  filter (5 copies: aio, catalogs, mntent with `== `/`-- ` headers; posixipc, syslog with `== `).
  Index-free (the file is compiled into targets that deny `indexing_slicing`); same matching and messages.

55 of 58 `build` definitions moved (each is now a one-line wrapper, so call sites are unchanged).
No assertion, no recorded-difference list and no test was changed or removed.

Left as they were, with the reason:
* `build`: forced_unwind, hot_text (dynamic links through the products' loader), ns_parse (three
  link modes incl. dynamic), heap_meta (asserting direct link without `oracle::build`; moving it
  would add `--gc-sections` to its program, a behaviour change).
* filters whose matching differs (moving them would change what they accept): fstree/process/terminals
  (keyed on the next line, first match only), netdb (line only), mallocdbg (ordered), dns (sectioned).
* the 32 KNOWN diff loops: six or more matching rules (locale-keyed, header-keyed, line-only,
  labelled); one shared loop would change assertions.
* `root` in the six products files that do not include `oracle.rs` (including it only for `root` costs
  more compile than it saves).

New test target `crates/core/tests/oracle_pair.rs` (6 tests, no compiler or products needed except
the quiet test, which uses `cc` if present): exact argv of every pair form, quiet builds, and the
filter's take-once, case-keyed and must-be-printed rules.

## Numbers

* Lines: products files 14,515 -> 13,740 (-775; 56 files, +186/-961); `oracle.rs` +135 (net harness
  -640); `oracle_pair.rs` +132 (tests). Audit estimate was ~-1,000; the rest sits in the filters and
  loops listed above as different by design.
* Time: no change expected (same programs, same stamps). Before, by the audit's T1 method (nextest
  PASS lines of CI gates on the CI host, the 56 migrated targets): full gates at 0495c4684 and
  aacf0bce8: 187 test runs, 2,506 and 2,467 test-s. After: to be read from the next CI gates once landed.

## Proof

See the report json and hand-back for the job labels (direct-tier gate, the math targets in both
passes, plants). Plants: `handoff/PRODUCTS-HARNESS-SHARED-plants.txt` (10: each pair option dropped
or ignored, the filter ignoring its case / taking a line twice / forgetting a stale entry, a dropped
`libm.a` in a real math products link).

## Next step

Land. Then M7 (PRODUCTS-ONE-BINARY) can merge targets mechanically. A follow-up could fold the
next-line filter (3 copies) into the shared one by keying entries on (line, next line); it changes
the first-match rule, so it needs its own plants.
