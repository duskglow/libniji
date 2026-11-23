# LOCALEDEF-INPUTS: POSIX symbolic names and the I18NPATH / working-directory search (handoff, leg 2)

**State.** Branch `task/localedef-inputs` (from origin/next 3db4c4009), role shiki `~/w/sh1`. L2 and L3 are done,
compiled, tested and planted. Code head c54e9deb (+ a docs/record commit for D-31.26 on top). Probe scripts and
outputs: shiki `~/work/localedef-inputs/` (`probe.sh`/`probe2.sh <localedef>`; `stock*.txt`, `niji*.txt`,
`table.txt`).

## Done (do not redo)

* L2 (s31_8_i_5): charmap lines named without `<Uxxxx>` take the `<Uxxxx>` after the bytes or the portable-set code
  point; source names resolved in statements and strings; undefined names skipped in LC_CTYPE lists, ranges, map
  pairs and (leg 2) LC_COLLATE order lines (the probe showed stock status 0 where libniji gave "an unknown collating
  symbol", status 1; fixed in `collate.rs`).
* L3 (s31_8_i_4): the search (cwd, I18NPATH `<e>/locales|charmaps/<n>`, `<e>/<n>`, `.gz` for charmaps outside cwd,
  default dir only without `/`, `-f` with `/` a path). Boundary reads I18NPATH from envp.
* Tests: `crates/locale/tests/prog.rs` (`source_search_order`, `charmap_search_order`, `posix_symbolic_names`,
  `undefined_names_are_skipped`), `crates/locale/tests/reference.rs`
  `products_localedef_searches_i18npath_and_the_working_directory` (real binary, env I18NPATH + cwd).
* Plants: `handoff/LOCALEDEF-INPUTS.plants`, results `handoff/LOCALEDEF-INPUTS-plants.txt`: 13/13 CAUGHT, no WARN
  (shiki job `localedef-inputs-plants2`, head c54e9deb).
* Direct gate: shiki `localedef-inputs-gate2` (head c54e9deb), STILL RUNNING at the leg-2 refresh (started 04:16Z):
  collect it first with `~/bin/jobwait localedef-inputs-gate2` and `~/bin/test-summary ~/logs/localedef-inputs-gate2.log`
  (fix any red in the direct tests; a later code change needs a new gate). Superseded: gate1/plants1 (killed, older
  head), t1..t5 (development runs). `cargo xtask affected`
  says `FULL: the train's (spec/clauses.tsv: generated table or its input)`; the direct tier ran.
* Clippy (workspace, all targets) clean on c54e9deb; `cargo xtask static` ran clean on the leg-1 head (re-run on final).
* `--big-endian` assessed: NOT small (about 30 little-endian write sites across cats/build/ctype/collate/archive in
  `crates/locale/src` plus core's table builders; no reader to test against). Refusal kept; D-31.26 record and its
  differences row reworded to "libniji currently supports only little-endian x86-64 ... on the roadmap".
  Roadmap: choose byte order at output in one writer helper, route every `to_le_bytes` there, test by a byte-swapping
  reader in the test, then implement `--big-endian`/`--little-endian` as probed.

## Probe table (stock 2.39 vs libniji, shiki; 52 cases)

All I18NPATH/cwd cases (i1..i12, f1..f13, fi) match stock in status and the file chosen. Name cases match except
where stock emits its own extra diagnostics for POSIX-name charmaps (`[warning] character map ... not ASCII
compatible`, `[error] <SP> character not in class space`): stock status 1 vs libniji 0 (probe2 s-posix-posix,
s-ucs-posix, s-posix-unames), stock 4 vs libniji 1 without -c (s-posix-posix-noc, s-ucs-posix-noc), and in those
error states stock's toupper(a) stays `a` where libniji maps it to `A`. libniji has no such checks (pre-existing).
INFORMATIONAL / possible RULING: recommend keep libniji's (no false error), record as a differences row in the
next leg if the coordinator agrees.

## Next step (exact)

1. D-31.24 (undefined-category defaults, brief's second coordinator note): three-way rule. POSIX XBD 7.3 / localedef
   (cite the clause on categories absent from the source), man pages, black-box probe (compile a source that omits
   each category with stock and libniji and read back through the locale functions). Then keep + sharpen the row,
   or match the reference's values and remove the row; tests per category; plants.
2. Re-run `cargo xtask static`, direct gate and plants for any new code; `cargo xtask precheck --jobs ... --head`;
   report json; `ops handback-check`.

Plan rows: D-31.24, D-31.26 in `plan/07-decisions/s31-utilities.md`.
