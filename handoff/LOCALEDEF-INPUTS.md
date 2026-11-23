# LOCALEDEF-INPUTS: POSIX symbolic names, the I18NPATH / working-directory search, undefined-category defaults

**State.** Branch `task/localedef-inputs` (from origin/next 3db4c4009), role shiki `~/w/sh1`. Code head e87311c6
(the hand-back commit with this note and the report sits on top). All three parts done, tested and planted.
Probe scripts and outputs (L2/L3): shiki `~/work/localedef-inputs/` (`probe.sh`/`probe2.sh <localedef>`,
`stock*.txt`, `niji*.txt`, `table.txt`).

## Done (do not redo)

* L2 (s31_8_i_5): charmap lines named without `<Uxxxx>` take the `<Uxxxx>` after the bytes or the portable-set code
  point; source names resolved in statements and strings; undefined names skipped in LC_CTYPE lists, ranges, map
  pairs and LC_COLLATE order lines (stock status 0 there).
* L3 (s31_8_i_4): the search (cwd, I18NPATH `<e>/locales|charmaps/<n>`, `<e>/<n>`, `.gz` for charmaps outside cwd,
  default dir only without `/`, `-f` with `/` a path). Boundary reads I18NPATH from envp.
* D-31.24 (leg 3, ruling #7754 by the three-way rule): POSIX.1-2017 XBD 7.3 ("In the event that some of the
  information for a locale category ... is missing from the locale source definition, the behavior of that category,
  if it is referenced, is unspecified"; a file holds "one or more" category definitions); the XCU localedef page,
  localedef(1) and locale(5) say nothing more. So the reference's values: `defaults.rs` rewritten (ASCII classes
  without punct/cntrl, print/graph = alnum (+space), monetary 255s, empty alt months, country codes of spaces), and
  a `defaults` flag in `cats::Body` / `ctype::write` for what a source cannot state (empty wide LC_TIME strings up to
  `t_fmt`, no wide input digits, one-byte `mon_grouping` 0x7f, CRNCYSTR `-`). D-31.24 rewritten; its differences row
  removed; spec clause s31_8_g_3 reworded; stubs regenerated.
* Tests: `crates/locale/tests/prog.rs` (`source_search_order`, `charmap_search_order`, `posix_symbolic_names`,
  `undefined_names_are_skipped`); `crates/locale/tests/reference.rs`
  `products_localedef_searches_i18npath_and_the_working_directory`, `undefined_categories_get_the_references_defaults`
  (now strict: every item of every file vs the reference, UTF-8 and ISO-8859-1, no exceptions) and five per-category
  tests without the reference (`undefined_{ctype,time,monetary,address,other_categories}...`).
* Plants: `handoff/LOCALEDEF-INPUTS.plants` (13, L2/L3) and `handoff/LOCALEDEF-INPUTS-defaults.plants` (9, D-31.24),
  results `handoff/LOCALEDEF-INPUTS-plants.txt` (13/13 CAUGHT, shiki `localedef-inputs-plants4`, e87311c6) and `handoff/LOCALEDEF-INPUTS-defaults-plants.txt` (9/9 CAUGHT, `localedef-inputs-plants5`, f9e8bcfb; plants3 refused a two-site pattern, superseded). No WARN.
* Gate: shiki `localedef-inputs-gate3` (direct tier, head e87311c6): 9555 passed, 0 failed, 147 conformance. Superseded: gate2 (c54e9deb, green: 9550 passed,
  0 failed), plants2 (c54e9deb, 13/13), gate1/plants1, t1..t5, d1..d3 (development runs), lint3 (stubs + clippy +
  static on the pre-commit tree, all clean). `cargo xtask affected` says FULL (spec/clauses.tsv is a generated
  table's input): FULL is the train's.
* `--big-endian`: NOT small (about 30 little-endian write sites across cats/build/ctype/collate/archive plus core's
  table builders). Refusal kept; D-31.26 reason "libniji currently supports only little-endian x86-64 ... on the
  roadmap". Roadmap: one writer helper choosing byte order at output, every `to_le_bytes` routed there, a
  byte-swapping reader in the test, then `--big-endian`/`--little-endian` as probed.

## Open (INFORMATIONAL)

For POSIX-name charmaps the reference adds its own diagnostics (`not ASCII compatible`, `<SP> character not in class
space`): status 1 vs libniji 0, 4 vs 1 without -c, and toupper(a) stays `a` there. libniji has no such checks
(pre-existing); recommend keeping libniji's (no false error) and a differences row if the coordinator agrees.

## Next step

Collect gate3/plants3/plants4 if the hand-back said PENDING JOBS; otherwise none (story complete).
Plan rows: D-31.24, D-31.26 in `plan/07-decisions/s31-utilities.md`.
