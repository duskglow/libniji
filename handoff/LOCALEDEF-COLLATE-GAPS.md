# LOCALEDEF-COLLATE-GAPS: compact code point order in LC_COLLATE, refuse an empty LC_COLLATE

**State.** Branch `task/localedef-collate-gaps` (from origin/next 0495c4684), role keke `~/w/r2`. Code, tests,
records and plants committed (two legs). Code head 7eba3dfdb, proven by keke jobs `localedef-collate-gaps-gate2`
(direct tier: 12 suites, 7445 passed, 0 failed; conformance 147/0; peak 11G), `localedef-collate-gaps-plants2` (8 of 8
CAUGHT, results in handoff/LOCALEDEF-COLLATE-GAPS-plants.txt), `localedef-collate-gaps-lint5` (records check, clippy
-D warnings, static: green); `cargo xtask precheck` exit 0. Superseded: `-gate` (killed: code changed), `-plants`
(run-inside-off SURVIVED: the check was redundant and was removed), `-lint`..`-lint4` (fixes / unsynced tree).
Report: handoff/reports/LOCALEDEF-COLLATE-GAPS.json.

**Decisions in force.** D-31.31 (this story: code point order without rules; lazy `..` runs; empty LC_COLLATE
refused), D-31.29 (defaults for undefined categories; the 92-byte LC_COLLATE). Spec clause added: s31_8_f_3.

## What the branch does

* `collate.rs`: a `..` line is one node (`Key::Run`) until a line names a character inside it (order line or
  `reorder-after`: `touch`) or the tables are written; `code_point_order()` sends an order that is every charmap
  character in code point order, each weighted by itself, no element/UNDEFINED/backward, over a charmap whose byte
  order is its code point order, to `no_rules(Some(cs))` (the installed C.UTF-8's items). `levels == 0` from a source:
  `Err(EMPTY)` (`ErrorKind::Refused`); as a default: the 92-byte file.
* `prog.rs`: Refused: `<file>:<END line>: LC_COLLATE: empty category description not allowed`, `[error] too many
  errors; giving up`, status 1, output directory made, no file (probed with and without -c).

## Measured (Zen 1 host)

* C.UTF-8 order (`<U00000000>` `..` `<U0010FFFF>`), products localedef: 1530 bytes, 5 MB peak RSS, 0.25 s, runs
  under `ulimit -v 16384`. Before (eager, from LOCALE-SOURCES-GEN #7792, not re-measured): 31 MB, 1.2 GB.
  Reference localedef on the same source: 7.2 MB, 177 MB. Reference C.UTF-8: 1406 bytes.
* Reference strcoll/wcscoll/strxfrm/wcsxfrm on libniji's file == on its C.UTF-8: 3600 pairs of 60 code points over
  every plane (probe keke `~/work/localedef-collate-gaps/planes.c`, `pl-after.txt` vs `pl-cutf8.txt`); the reference's
  own compile of the source differs on 2850 of them (`pl-refcp8.txt`).

## Open

* RULING NEEDED: the differences row for D-31.31 has no compat option (recommend: none; the result is the installed
  C.UTF-8 collation, and a source wanting tables adds a non-self weight).
* Brief step 2 (generate the C.UTF-8 source with LOCALE-SOURCES-GEN's generator and build it in the locale tests) is
  NOT done: LOCALE-SOURCES-GEN-2 has not landed on next (checked 10:05Z). Next step once it lands: merge origin/next,
  restore `c_source()` in `xtask/src/locales.rs` with `<U00000000>`..`<U0010FFFF>` (equal-width names: the
  reference refuses `<U0000>..<U10FFFF>`), and build it in the locale tests.

## Not to redo

Probes of the reference (empty section, symbols-only, order-only, UNDEFINED-only, undefined category, cp8) are in
keke `~/work/localedef-collate-gaps/`; results above and in D-31.31.
