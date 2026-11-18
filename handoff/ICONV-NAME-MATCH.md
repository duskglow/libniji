# ICONV-NAME-MATCH (issue #6: `iso88591`): iconv_open name rule — run 2 (ICONV-NAME-MATCH-2)

**State (run 3, leg 1).** Branch `task/iconv-name-match-3`, code head `dc120f974` (WIP, see "Run 3" below; still
above base, must not land). Run 2: branch `task/iconv-name-match-2`, code head `b79bfe76a`, role mei:~/w/s2.
Decision: `records/pending/ICONV-NAME-MATCH-1.toml` (D-NEW, series D-6, amends D-6.8; CI assigns the id). Rule,
probe table and alias provenance: `spec/encoding/iconv-names.md`. Plants: `handoff/ICONV-NAME-MATCH-plants.txt` (14).

## The rule (run 1, unchanged)

Only letters, digits and `, - . / : _` count; other bytes drop; case is ignored; the name is `PART[/PART2]`; the
result must equal a known name exactly (`cp-1252`, `-utf8`, `utf8.` name nothing). See the spec file.

## Run 2: why run 1's gate was red, and the fix

* `iconv_cmd_names`: the built-ins gained `ISO88591`, and the header `csys/util/iconv-names.h` did not have it.
  The reference's `iconv -l` lists `ISO88591//` (probed on mei), so the header now carries it.
* `iconv_products` (2 lines): `KOI8R` and `ISO885915` failed with `EINVAL` against the host's distro charmaps. The
  exact rule knows a charmap's own names only. libniji's generated charmaps carry these aliases, but the distro
  files do not; the earlier loose match (letters and digits) had hidden that. The gap was measured on mei
  (scratch `gap.py`): the reference lists 1,180 names, and libniji knows 651 under the exact rule. Of the 601 listed
  names libniji does not know, 108 have the same letters and digits as a known name. Those are the names next
  opened and run 1 lost.
* Fix: `crates/core/src/iconv/aliases.rs`, a sorted table of 106 such names → charmap file stem (`ISO88591` is a
  built-in; `NF_Z_62-010_(1973)` already reduces to its stem). Every pair passed a black-box fingerprint check on
  the reference (scratch `fp.c`: all 1- and 2-byte decodes, all BMP encodes and a stride of the rest): 108 of 108 the
  same, 0 different. The probe separates BIG5 from BIG5-HKSCS and EUC-KR from CP949. `HPTHAI8` points to
  `ISO-8859-11`, as `spec/encoding/charmaps.tsv` does: the reference converts ISO-8859-11, HP-THAI8 and HPTHAI8
  alike. `load_charmap` consults the table only after the loaded charmaps miss, so reopening a loaded charmap
  costs nothing extra. Locales pass `no_alias` and are unchanged.
* Test: `s06_multibyte_clauses::s6_4_c_1_charmap_aliases` checks that the table is sorted, every row is in
  matching form, no row is a built-in, and the generated charmaps never give a row's name to another charmap
  (that check found the HPTHAI8 conflict). `iconv_products` proves the lookup end to end (KOI8R, ISO885915).

## Results (final head)

* Gate `iconv-name-match-2-gate` (mei, head `b79bfe76a`), rc 0. Direct tier: 32 suites, 12,104 passed, 0 failed,
  1,397 ignored; conformance 138 passed. Start-up checks: `cargo xtask test` (6,784 + 5,296 passed) and
  `NIJI_FULL_PROFILE=1` `hot_text_products`, green. Peak 17G.
* Plants `iconv-name-match-2-plants`: 14 CAUGHT, 0 SURVIVED, 0 WARN (`handoff/ICONV-NAME-MATCH-2-plants.txt`).
* Alias evidence (`iconv-name-match-2-counts`, host's distro charmaps): libniji opens all 106 rows; 94 convert
  exactly as the reference does. The other 12 (EUCJP-MS EUCKR EUCTW IBM-1132/1133/1160/1161/1162 MACCYRILLIC
  SHIFT-JIS SHIFTJISX0213 T.618BIT) differ from the reference, but just as the charmap's own name does in libniji
  (same libniji fingerprint): these are charmap conversion differences that predate this story, not name errors.
* **Instructions: REGRESSION, measured on the same host** (`iconv-name-match-2-before` built base `06c0b561e`;
  probe `spell.c -n 1000`, open both ways and close, generated charmaps mounted; instructions:u per 1000 iterations):

  | name | before (base) | after (b79bfe76a) | change |
  | --- | ---: | ---: | ---: |
  | UTF-8 | 1,697,700 | 2,635,672 | +55% |
  | utf-8 | 1,697,700 | 2,635,671 | +55% |
  | CP1252 | 3,208,988 | 4,691,329 | +46% |
  | ISO-8859-15 | 3,013,810 | 5,079,917 | +69% |
  | KOI8R | 4,051,123 | 5,203,470 | +28% |

  Start-up alone (`-n 0`) is unchanged (3,218 / 3,190). UTF-8 is a built-in and never reaches run 2's alias step, so
  the cost is in run 1's name handling. Lead, not yet profiled: `parse_name` now builds a 128-byte matching form per
  name (zero-filled, a bounds-checked `push` per byte, `significant` + uppercase). Before, it returned a borrowed
  slice. `builtin` then scans the table with exact compares. All of this happens twice per `iconv_open`.

## Run 3 (ICONV-NAME-MATCH-3, branch `task/iconv-name-match-3`, role mei:~/w/s3): in-place matching, leg 1

Ruling #6385: no landing at +28..69%; remove the 128-byte copy, reach base for every measured case.

* Profile (perf record instructions:u, `-n 500000`, per symbol; `~/work/ICONV-NAME-MATCH-3/prof.sh` on mei builds
  base `06c0b561e` once into `/mnt/zram/claude/ICONV-NAME-MATCH-3/base-06c0b561e` and the role head, prints the
  count table and per-symbol profiles; run it under `jobrun`): run 2's cost was `parse_name` (37%: zero-fill,
  bounds-checked push per byte), `memmove` (7%, the 150-byte `Spec` moved by value) and `bcmp`.
* Commit `dc120f974` (WIP): `iconv::Name` keeps the caller's text (first/second part slices, matching-form
  length, `plain` flag); one pass with a plain-byte shortcut (`part`); `Name::is` compares a row in place;
  `builtin` walks only the rows of the name's length (`BY_LEN`, const bit table, `#[allow]` as in
  time/format.rs); only the cold `gl::charmap_codec` writes the form out (`Name::form`, into a stack buffer);
  `names_charset` takes a case-only match first. Rule, alias table, `iconv -l` header unchanged. Tests adapted;
  new: every byte 0..=255 between two letters (both paths), `Name::is` table. clippy clean; s06 + iconv_cmd_names pass.

  | name (x1000, instructions:u) | base 06c0b561e | run 2 b79bfe76a | dc120f974 |
  | --- | ---: | ---: | ---: |
  | UTF-8 / utf-8 | 1,697,671 | 2,635,671 | 2,029,663 (+19.6%) |
  | CP1252 | 3,208,960 | 4,691,334 | 3,701,032 (+15.3%) |
  | ISO-8859-15 | 3,013,762 | 5,079,913 | 3,585,701 (+19.0%) |
  | KOI8R | 4,051,082 | 5,203,462 | 4,111,191 (+1.5%) |
  | latin1 | 2,378,671 | 3,426,671 | 2,380,663 (+0.1%) |
  | EUC-JP | 1,656,672 | 2,604,671 | 2,018,664 (+21.9%) |

  Remaining UTF-8 gap (+332/iteration = 4 names), per symbol vs base: `parse_name` +165 (78 vs 37 per name),
  `boundary::iconv::open` (+ inlined `codec`) +100 (the 56-byte `Spec` x2), `builtin` +58.

## Exact next step (leg 2)

1. Shrink `Name` to 4 words: `text` (first part, then `/` and the cut second part, contiguous in the caller's
   string), `split` (index of that `/`; `text.len()` when none), `len`. Drop `plain`: the shortcut condition is
   `text.len() == len` (every byte significant, so the form is `text` uppercased, `/` included).
2. Classify bytes with one `static CLASS: [u8; 256]` (dropped / plain / comma / slash, built by a const fn,
   `#[allow(clippy::indexing_slicing, clippy::as_conversions)]`) instead of the `SIG_LO`/`SIG_HI` masks.
3. Re-measure (`prof.sh`); if still above base, look at `open` (avoid moving `Spec`; parse `to` after `from`
   fails?) and the `builtin` row compare. Below base everywhere: update `handoff/ICONV-NAME-MATCH-plants.txt`
   (plants `name-case-kept`, `name-empty-second-part-kept`, `name-overflow-truncates` target removed code; add
   plants for `BY_LEN`, the plain shortcut, `cut` of commas, `names_charset` shortcut), then gate, plants, start-up
   checks, counts. Parity impossible: RULING NEEDED with the table above and the profile.

## Open items

* 493 listed names have no letters-and-digits partner among libniji's names (`BIG-FIVE`, `CP1140`, `8859_2`, ...).
  next did not open them either. A follow-up could map them by the same fingerprint check against every charmap
  (some charsets have no charmap at all). Scratch on mei: `~/work/ICONV-NAME-MATCH-2/{gap.py,pairs.py,fp.c}`.
* `iconv -l` (D-31.16) lists built-ins and charmap stems, not aliases; the reference also lists its aliases.
* Suffix spelling (run 1): the reference accepts `//translit ` with a trailing space; libniji does not.

## Do not redo

The 32,915-spelling probe (run 1, keke `~/work/ICONV-NAME-MATCH`) and the 108-pair fingerprint check (mei).
