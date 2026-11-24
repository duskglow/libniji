# C-LOCALE-IDENTIFICATION (GitHub issue #11): the C locale's LC_IDENTIFICATION

State: branch `task/c-locale-identification`, cut from origin/next 0495c4684. Code commit dce808f7f; the
report commit sits on top of it (see `git log`). Role keke:~/w/n4 (Zen 1 host).

## What changed (D-7.22, amends D-7.10 and D-7.16)

- The built-in "C" (and "POSIX") now report the reference's LC_IDENTIFICATION, the ISO/IEC 14652 i18n FDCC-set
  identification: title, source, territory `ISO`, revision `1.0`, date `1997-12-20`, 12 category strings
  `i18n:1999` (the 7th, the LC_ALL slot, is empty in the reference and here), codeset. The issue's item
  `_NL_IDENTIFICATION_TERRITORY` is `"ISO"`.
- The address, contact, e-mail, telephone and fax items hold an individual's contact data and stay EMPTY (the
  owner accepted this in D-7.10, 2026-10-01; coordinator #7965 confirmed it for this story: no ruling needed).
  Public text names them only as "contact fields of an individual", never their values.
- C.UTF-8 already matched the reference on all 16 items (probe); unchanged.
- Provenance: `spec/tools/langinfo_probe.py` records the new values; its PROBE program was missing from the tree
  and is now `spec/tools/probe_langinfo.c` (written from the interface the .py documents). A full re-record on
  the reference system (glibc 2.39, Ubuntu build) reproduced every other row of `spec/abi/langinfo.tsv` byte for
  byte; only the C column of category 12 changed. `builtin_items.rs` regenerated (`cargo xtask locale-gen`).
- docs/differences-from-glibc.md: the C-locale LC_IDENTIFICATION row narrowed to the five contact fields; no
  compat option (it would mean shipping the personal data), reason in the row.

## Probe-settled behaviour (reference, black-box, all 17 indices 0..16 of category 12)

C and POSIX identical. C.UTF-8: title "C locale", e-mail a mailing list, revision 2.1, date 2022-01-30,
category i18n:2012 x12, codeset UTF-8, the rest empty. Index 16 (past the end) is "" in all three.

## Tests

- `s7_4_c_2` (crates/core/tests/s07_locale_clauses.rs): TERRITORY is "ISO", items 2..6 are "".
- `locale_products::locale_behaviour_equals_the_reference`: `locale_diff.c` gains `identification()`, every
  item of C, POSIX, C.UTF-8 diffed against the host library (all 12 category strings; items 2..6 of C/POSIX
  printed as a fixed marker).
- Plants: handoff/C-LOCALE-IDENTIFICATION-plants.txt (4; mutate the TSV, regenerate in the command).

## Results, open questions, next step

See handoff/C-LOCALE-IDENTIFICATION-report.md (jobs, verdicts) and handoff/reports/C-LOCALE-IDENTIFICATION.json.
Not to redo: the probe and the TSV re-record. Plan rows: D-7.10, D-7.16, D-7.22.
