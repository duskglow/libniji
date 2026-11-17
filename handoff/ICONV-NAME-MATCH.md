# ICONV-NAME-MATCH (issue #6: `iso88591`): iconv_open name rule — run 2 (ICONV-NAME-MATCH-2)

**State.** Branch `task/iconv-name-match-2` (on `task/iconv-name-match`), code head `b79bfe76a`. Role mei:~/w/s2.
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

## Exact next step

Profile one `iconv_open("UTF-8","UTF-8")` per phase (validation/wl_trace.sh on base `06c0b561e` and this head), then
make name parsing cost no more than before. For example: match in place without the 128-byte copy (compare the
significant bytes of the argument against each candidate as `names_charset` does), or index the built-ins by
length or first byte. Re-measure with `~/work/ICONV-NAME-MATCH-2/{counts.sh,before.sh}` (mei) and land only at
or below the before row. Do not change the rule or the alias table; both are verified.

## Open items

* 493 listed names have no letters-and-digits partner among libniji's names (`BIG-FIVE`, `CP1140`, `8859_2`, ...).
  next did not open them either. A follow-up could map them by the same fingerprint check against every charmap
  (some charsets have no charmap at all). Scratch on mei: `~/work/ICONV-NAME-MATCH-2/{gap.py,pairs.py,fp.c}`.
* `iconv -l` (D-31.16) lists built-ins and charmap stems, not aliases; the reference also lists its aliases.
* Suffix spelling (run 1): the reference accepts `//translit ` with a trailing space; libniji does not.

## Do not redo

The 32,915-spelling probe (run 1, keke `~/work/ICONV-NAME-MATCH`) and the 108-pair fingerprint check (mei).
