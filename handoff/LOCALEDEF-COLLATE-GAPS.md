# LOCALEDEF-COLLATE-GAPS: compact code point order in LC_COLLATE, refuse an empty LC_COLLATE

**State.** Branch `task/localedef-collate-gaps` (from origin/next 0495c4684), role keke `~/w/r2`. Leg 1 ended at
a refresh: code committed as WIP (type-checks on keke: `cargo check -p niji-locale --all-targets` clean), no tests
yet, no jobs run. Probe files: keke `~/work/localedef-collate-gaps/` (`probe.sh`, `probe2.sh <localedef> <tag>
<src>...`, `loadcoll.c` -> `./loadcoll <name>` with LOCPATH, sources `empty symonly ordonly undefonly cp cp8 cpbmp
meas`.src, outputs `ref*.txt`, `lc-*.txt`).

## Probe-settled (reference, black box, keke)

* Installed C.UTF-8 LC_COLLATE: 1406 bytes, 19 items: NRULES 0; items 1-12 empty; item 13 a 4-byte word 0;
  14-15 empty; 16 COLLSEQMB = bytes 0..255 identity; 17 COLLSEQWC a table giving cp for cp 0..255; 18 "UTF-8\0".
* Undefined LC_COLLATE (source with only LC_MEASUREMENT): 92 bytes = the same without 16-18 (all empty). The
  reference's newlocale loads it (LOCPATH=dir, name `loc`; a path name with `/` gives ENOENT, use LOCPATH).
  libniji's old 88-byte file lacked the item-13 word.
* Empty LC_COLLATE (no order_start; also a section of only `collating-symbol`): `<file>:<END line>: LC_COLLATE:
  empty category description not allowed`, status 1 with and without -c, NO files written (the output directory is
  made). `order_start`/`order_end` with nothing between, and a section with only UNDEFINED: accepted (1446 bytes).
* The reference's localedef on `<U00000000>\n..\n<U0010FFFF>` (cp8.src): 7.2 MB LC_COLLATE, NRULES 1, 177 MB
  RSS; `<U0000>..<U10FFFF>` (different widths) is an error there ("not valid names for symbolic range") -> the C
  source must use equal-width names (`<U00000000>`..`<U0010FFFF>`). Its strcoll on that compile differs from its
  C.UTF-8 for `\xff` and U+10FFFF (lc-cp8.txt vs lc-cutf8.txt); libniji's compact form gives C.UTF-8's order.

## Done (leg 1, commit "WIP: localedef writes a plain code point order ...")

* `collate.rs`: `Key::Run(id)`; `Collation.runs` / `open`; `expand`, `run_of`, `touch`: a `..` line is one node
  until a line names a character inside it (touch before reorder-after and order lines) or the tables are written
  (all open runs expanded at the start of the table path); overlapping runs expanded first.
  `code_point_order()` (checks runs as ranges; coverage = every `cs.map().mappings()` cp in the covered ranges) and
  `no_rules(Option<&Charset>)`; `write(stmts, cs, defaults)`: levels 0 -> `Err(EMPTY)` (kind `Refused`) for a
  source, the 92-byte file for defaults.
* `lib.rs`: `ErrorKind::Refused`; compile_text returns it with the category's END line (`Source.ends`, source.rs).
* `prog.rs`: Refused arm: `<shown>:<line>: LC_COLLATE: empty category description not allowed`, then `[error] no
  output file produced because errors were issued`, status 1.

## Next steps (in order)

1. Tests (name the spec clause; s31_8_* for localedef, Appendix D.1.4 for the file):
   * prog.rs: `empty_collate_is_refused` (status 1 with and without -c, no category file written, message with
     END line; also a symbols-only section; `order_start`+`order_end` alone still compiles).
   * prog.rs or a lib test: code point order compiles to <= 2048 bytes and NRULES 0 (the size bound the
     "compact disabled" plant must trip); not compact when a character is out of order, when the range stops short
     (coverage), with a contraction, with UNDEFINED, with backward; a `..` run then a line moving a character inside
     it gives the same file as the same order written line by line (general path unchanged by laziness).
   * reference.rs: `code_point_order_compiles_like_the_reference_c_utf8`: libniji compile (Disk charmap UTF-8) vs
     `/usr/lib/locale/C.utf8/LC_COLLATE` (skip if absent): items 0, 13, 16, 18 byte-equal, item 17 values equal over
     every code point (Table3), both `Collation::CodePoint`; the products localedef on that source under `sh -c
     'ulimit -v 262144; exec ...'` succeeds (RSS bound: eager expansion needs over 1 GB), and the reference `locale`
     with LOCPATH=<out parent> LC_COLLATE=<name> prints nothing on stderr (it loads the file).
   * reference.rs line ~451 (`undefined_categories_get_the_references_defaults`): add `diff_items` for LC_COLLATE
     so the 92-byte default is compared item for item.
2. Step 2 of the brief (C.UTF-8 source from LOCALE-SOURCES-GEN's generator): LOCALE-SOURCES-GEN-2 was queued_ci,
   not on next at 09:45Z. If it has landed, merge origin/next and restore `c_source()` in `xtask/src/locales.rs`
   (see D-31.27) with `<U00000000>`..`<U0010FFFF>`; else report it as the remaining step (do not merge an unlanded
   branch without the coordinator).
3. Records: reserve `python3 ~/src/liella-ops/bin/ops reserve --story LOCALEDEF-COLLATE-GAPS D-31` for the
   compact form (the comment in collate.rs says D-31.NEW: replace). RULING NEEDED: a source whose order is all of
   the charmap in code point order compiles without rules, so a byte that is no character compares by byte value
   (the reference's compile of the same source puts it with the undefined characters) and strxfrm returns the
   string; recommend a differences row (neutral wording) and no compat option (reason: matches the reference's own
   C.UTF-8; a user wanting the tables can add any non-self weight).
4. Plants file `handoff/LOCALEDEF-COLLATE-GAPS.plants`: compact disabled (`if code_point_order(&c, cs)` -> `if
   false && ...`, msg: the size-bound test), empty check removed (`if !defaults {` -> `if false {`), item-13 word
   dropped, coverage check made always true, laziness disabled (expand right after placing a run; msg: the ulimit
   products test).
5. clippy + static on keke, gate (`cargo xtask affected --run`, kind gate), plants, measure C.UTF-8 size and peak
   RSS (products localedef, /usr/bin/time) before (origin/next: 31 MB / 1.2 GB per the premise) and after; loadcoll
   probe of libniji's output with the reference (LOCPATH) vs `C.UTF-8`; precheck; report json; handback-check.
