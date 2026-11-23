# LOCALEDEF-INPUTS: POSIX symbolic names and the I18NPATH / working-directory search (handoff, leg 1)

**State.** Branch `task/localedef-inputs` (from origin/next 3db4c4009), role shiki `~/w/sh1`. One WIP commit:
code + tests + spec clauses written; NOT YET COMPILED GREEN (last compile error fixed, not re-run). No records, no
differences rows changed yet. Probe scripts and outputs: shiki `~/work/localedef-inputs/` (`probe.sh` -> `stock.txt`,
`probe2.sh` -> `stock2.txt`; `sh probe.sh <localedef>` runs the same cases on any localedef). Scratch copies:
`.scratch/LOCALEDEF-INPUTS/` (local).

## Probe-settled (stock localedef 2.39, shiki)

* Search, `-i`: cwd as given, then per I18NPATH entry (in order, empty entries skipped, relative entries from cwd)
  `<e>/locales/<n>` then `<e>/<n>`, then default dir. Directories are skipped. A relative `-i` name WITH `/`
  is also searched in I18NPATH (i8). `copy` names search the same way (i7, i7b). Missing: rc 4, `[error] cannot
  open locale definition file `X': No such file or directory`.
* Search, `-f`: cwd (plain only, `.gz` NOT tried in cwd: f8), then `<e>/charmaps/<n>`, `<e>/<n>` with `.gz`
  (f9, f10: plain before gz), then default. A `-f` name with `/` is a path only (f11/f12). Missing with -c: rc 1
  (falls back to the default charmap); without -c rc 4.
* Names: a charmap of POSIX names only (`<a> /x61`) is accepted; `<a>` in a source resolves (alpha set). The
  `<Uxxxx>` after the bytes behaves the same. Names the charmap lacks in class lists / map pairs: skipped silently,
  rc unchanged (s-unknown rc 0 even without -c). In strings: `symbol `x' not in charmap` + rc 1 under -c.
  Stock also prints `[warning] character map `UTF-8' is not ASCII compatible` and a false `[error] <SP> character
  not in class `space'` for POSIX-name charmaps (rc 1 with -c, rc 4 without); libniji has no such checks at all
  (pre-existing), so libniji gives 0 there: INFORMATIONAL / possible RULING (not a row yet).

## What the code does (uncommitted-to-green)

* `crates/core/src/charmap/mod.rs`: `<name> bytes` lines without `<Uxxxx>` take the portable-set code point
  (XBD 6.1 table `PORTABLE`, `portable()`); named mappings recorded, `Charmap::named(name)`.
* `crates/locale/src/lib.rs` `resolve_names`: after parse, every `Sym`/string name the charmap defines becomes
  `<Uxxxxxxxx>` (`lex::resolve_string`, `lex::ucs_text`). `ctype.rs`: unknown names in lists/ranges/pairs skipped.
* `prog.rs` `Inputs::read`: the search above; `Host::i18npath()` (default None); boundary `Disk` reads I18NPATH from
  envp after argv.
* Spec: two new Inputs bullets in §31.8 (should become `s31_8_i_4` search, `s31_8_i_5` names after `cargo xtask
  stubs`; check the ids in spec/clauses.tsv and fix the test doc comments if they differ).
* Tests: `crates/locale/tests/prog.rs` `source_search_order`, `charmap_search_order`, `posix_symbolic_names`,
  `undefined_names_are_skipped`.

## Next step (exact)

1. Sync; on shiki: `cargo xtask stubs`, `cargo test -p niji-locale --test prog`, core charmap tests; fix.
2. `cargo xtask clippy`, `cargo xtask static`; plants file `handoff/LOCALEDEF-INPUTS.plants` (one per rule:
   portable fallback, resolve_names, skip-unknown, cwd first, I18NPATH order, charmaps/.gz, slash rule) via
   `cargo xtask plants`; direct gate; start-up checks (§9: boundary entry touched).
3. Run `probe.sh`/`probe2.sh` with libniji's localedef (`~/w/sh1/target/niji/...`) for the probe table.
4. Not started: `--big-endian` (assess writer: how many `to_le_bytes` sites; if not small, reword D-31.26's reason to
   "libniji currently supports only little-endian x86-64") and D-31.24 (undefined-category defaults three-way
   rule). Likely hand back with these as next steps if budget is short.
