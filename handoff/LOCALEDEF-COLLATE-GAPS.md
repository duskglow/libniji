# LOCALEDEF-COLLATE-GAPS: compact code point order in LC_COLLATE, refuse an empty LC_COLLATE

**State.** Branch `task/localedef-collate-gaps-2` (LOCALEDEF-COLLATE-GAPS-2 continues `task/localedef-collate-gaps`;
origin/next 997a3a8c3 merged in at 2434ce53b), role shiki `~/w/sh1`. The code is unchanged since 7eba3dfdb. Leg 3
(LOCALEDEF-COLLATE-GAPS-2) fixed the CI merge conflict (next added transliteration tests at the end of
`crates/locale/tests/prog.rs` and `reference.rs`; both sides kept) and cited ruling #7942 in the D-31.31 row and
record, and committed the stub header `cargo xtask stubs` writes after the merge (both sides added a §31 clause: 115).
Proven on head 25ed17cb5 by shiki jobs `localedef-collate-gaps-2-gate2` (direct tier: 12 suites, 7454 passed, 0 failed; conformance 148/0; peak 11G),
`localedef-collate-gaps-2-plants2` (8 of 8 CAUGHT, results in handoff/LOCALEDEF-COLLATE-GAPS-plants.txt),
`localedef-collate-gaps-2-lint2` (clippy --all-features -D warnings, xtask clippy, static: green). Superseded:
`-gate` (7454/0, conformance 148/0), `-plants` (8/8 CAUGHT), `-lint` (green): all on the merge head before the stub
header commit (precheck: other content than the head).
Earlier legs' jobs (keke `localedef-collate-gaps-gate2`, `-plants2`, `-lint5`) proved 7eba3dfdb before the merge.
Report: handoff/reports/LOCALEDEF-COLLATE-GAPS-2.json (leg 3), handoff/reports/LOCALEDEF-COLLATE-GAPS.json (legs 1-2).

**Decisions in force.** D-31.31 (this story: code point order without rules; lazy `..` runs; empty LC_COLLATE
refused; no compat option, ruling #7942), D-31.29 (defaults for undefined categories; the 92-byte LC_COLLATE).
Spec clause added: s31_8_f_3.

**Start-up checks.** Not run: the change is in the `localedef` compiler (`niji-locale`, reached only from
`crates/boundary/src/localedef.rs`), no code that runs at process start-up. The direct gate's start-up layout tests
(`hot_text_products`, `cold_bss_products`, `heap_meta_products`) passed.

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
  every plane (probe keke `~/work/localedef-collate-gaps/planes.c`); the reference's own compile of the source differs
  on 2850 of them.

## Open

* Brief step 2 (generate the C.UTF-8 source with LOCALE-SOURCES-GEN's generator and build it in the locale tests) is
  NOT done: LOCALE-SOURCES-GEN-2 has not landed on next (checked 2026-11-23 22:13Z: `xtask/src/locales.rs` absent on
  next; that story is queued for CI). **Exact next step** once it lands: merge origin/next, restore `c_source()` in
  `xtask/src/locales.rs` with `<U00000000>`..`<U0010FFFF>` (equal-width names: the reference refuses
  `<U0000>..<U10FFFF>`), and build it in the locale tests. This could also be a follow-up story on
  LOCALE-SOURCES-GEN's side, so this branch can land now.

## Not to redo

Probes of the reference (empty section, symbols-only, order-only, UNDEFINED-only, undefined category, cp8) are in
keke `~/work/localedef-collate-gaps/`; results above and in D-31.31. The merge conflict is resolved (keep both sides).
