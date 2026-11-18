# ICONV-NAME-MATCH (issue #6: `iso88591`): iconv_open name rule — run 3 (ICONV-NAME-MATCH-3)

**State (run 3, leg 2, finished).** Branch `task/iconv-name-match-3`, code head `876a0e018`; role mei:~/w/s3.
Every measured `iconv_open` case is now below base (table in "Run 3" below); gate, start-up checks and 20 plants
green. Run 2: branch `task/iconv-name-match-2`, head `b79bfe76a`. Decision: `records/pending/ICONV-NAME-MATCH-1.toml`
(D-NEW, series D-6, amends D-6.8; CI assigns the id). Rule, probe table and alias provenance:
`spec/encoding/iconv-names.md`. Plants: `handoff/ICONV-NAME-MATCH-plants.txt` (20).

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

## Run 2 results (head b79bfe76a; its cost is fixed by run 3)

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

## Run 3 (ICONV-NAME-MATCH-3, role mei:~/w/s3): name matching at or below base

Ruling #6385: no landing at +28..69%; remove the 128-byte copy and reach base for every measured case.

* Profile (leg 1; perf record instructions:u, `-n 500000`, per symbol; `~/work/ICONV-NAME-MATCH-3/prof.sh` on mei
  builds base `06c0b561e` once into `/mnt/zram/claude/ICONV-NAME-MATCH-3/base-06c0b561e`, then the role head):
  run 2's cost was `parse_name` (37%: zero-fill, bounds-checked push per byte), `memmove` (7%, the 150-byte `Spec`
  moved by value) and `bcmp`. Leg 1 (`dc120f974`) compared names in place and was still +15..22%.
* Leg 2 (`876a0e018`): `iconv::Name` is 4 words: one slice of the caller's text (first part, then `/` and the cut
  second part), the index of that `/`, and the matching-form length. When every byte counts (`text.len() == len`)
  the form is the text uppercased and table rows compare in place (`folds_to`). One const 256-byte table `FOLD` gives
  each byte's matching-form byte (0 = dropped) and a plain flag (significant, not `,` or `/`). A name made only of
  plain bytes is read in one pass with no bookkeeping. The full rule (`parse_full`, `#[inline(never)]`) is used only
  for suffixes, a second part or dropped bytes. `names_charset` uses the same table for its case-only shortcut.
  `builtin` walks only the rows of the name's length (`BY_LEN`). The rule, the alias table and the `iconv -l` header
  are unchanged. New tests: a two-part name with a dropped byte (the slow path's `/`) and a same-length non-match
  for `names_charset`'s shortcut.

  Job `iconv-name-match-3-counts` (mei, head `876a0e018`; iconv_open both ways + close, x1000, instructions:u,
  generated charmaps mounted, same host and probe for both):

  | name | base 06c0b561e | run 2 b79bfe76a | leg 1 dc120f974 | final 876a0e018 | final vs base |
  | --- | ---: | ---: | ---: | ---: | ---: |
  | UTF-8 / utf-8 | 1,697,672 | 2,635,671 | 2,029,663 | 1,533,671 | −9.7% |
  | CP1252 | 3,208,966 | 4,691,334 | 3,701,032 | 2,931,122 | −8.7% |
  | ISO-8859-15 | 3,013,762 | 5,079,913 | 3,585,701 | 2,855,829 | −5.2% |
  | KOI8R | 4,051,088 | 5,203,462 | 4,111,191 | 3,211,339 | −20.7% |
  | latin1 | 2,378,672 | 3,426,671 | 2,380,663 | 1,756,671 | −26.1% |
  | EUC-JP | 1,656,673 | 2,604,671 | 2,018,664 | 1,514,671 | −8.6% |
  | ISO88591 | 788,151 (EINVAL) | | 2,668,664 | 1,906,670 | not comparable |

  ISO88591: base fails both opens with EINVAL (errno 22, print mode in the same job), which is issue #6, so its
  count is the cost of failing. The final head opens it as Latin-1 with the same fingerprint as `latin1`.
  Start-up alone (`-n 0`): 3,191 base, 3,190 final.
* Gate `iconv-name-match-3-gate` (direct tier, head `876a0e018`): rc 0, 32 suites, 12,104 passed, 0 failed, 1,397
  ignored; conformance 138/0; peak 16G. Plants `iconv-name-match-3-plants`: 20 CAUGHT, 0 SURVIVED, 0 WARN.
  Start-up checks `iconv-name-match-3-startup`: rc 0: `cargo xtask test` 6,784 + 5,296 passed; `NIJI_FULL_PROFILE=1` `xtask so` and `hot_text_products` 26 passed; peak 11G.

## Exact next step

None for this story: it is ready to land. Follow-ups are under "Open items".

## Open items

* 493 listed names have no letters-and-digits partner among libniji's names (`BIG-FIVE`, `CP1140`, `8859_2`, ...).
  next did not open them either. A follow-up could map them by the same fingerprint check against every charmap
  (some charsets have no charmap at all). Scratch on mei: `~/work/ICONV-NAME-MATCH-2/{gap.py,pairs.py,fp.c}`.
* `iconv -l` (D-31.16) lists built-ins and charmap stems, not aliases; the reference also lists its aliases.
* Suffix spelling (run 1): the reference accepts `//translit ` with a trailing space; libniji does not.

## Do not redo

The 32,915-spelling probe (run 1, keke `~/work/ICONV-NAME-MATCH`) and the 108-pair fingerprint check (mei).
