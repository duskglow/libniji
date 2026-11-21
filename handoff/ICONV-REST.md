# ICONV-REST: CP949/UHC, WINDOWS-125x spellings, iconv -l (issue #6, the rest)

**State (leg 2 end).** Branch `task/iconv-rest` from origin/next `4f32aa946`, role mei `~/w/s3`. Code, tests, docs
and record done; gate `iconv-rest-gate1` (direct tier, 39 of 310 targets) and plants `iconv-rest-plants1`
(`handoff/ICONV-REST-plants.txt` when they end) run on the final code head. Decision: **D-6.20** (amends D-6.17,
D-6.18). Report: `handoff/reports/ICONV-REST.json`.

## What landed on the branch

* **CP949 / UHC** built in (sixth set of the generic CJK codec): `CP949.TXT` 2.01 vendored, variant
  `spec/encoding/reference/CP949.tsv` (no cell departures), 108,801 B of read-only tables; names CP949, UHC,
  MSCP949, OSF100203B5 (probed; CP-949, MS949, WINDOWS-949, KS_C_5601-1987, KSC5601 do not open on the reference).
  Whole code space equals the reference (`iconv_codespace_products`, CP949 added), except the pair A2E8, which
  CP949.TXT does not assign: libniji reports it invalid with nothing consumed (`cp949_cells_and_unassigned_pair`,
  `OWN_TEST` in the code-space test). No //TRANSLIT block note for CP949 (the list copied from EUC-KR holds).
* **Alias table** `spec/encoding/iconv-aliases.tsv` (654 rows; the 104 earlier rows unchanged) + `csys/util/iconv-aliases.h`,
  written by `tools/probe/iconv_aliases.py` (fingerprints: `tools/probe/iconv_fingerprint.c`). Leg 2 fixed the
  tool: a charmap's conversion now comes from its file name and its installed file's header names only (it also
  took the aliases libniji's generated charmaps add, which made CP9448/IBM-9448/IBM9448/CSIBM9448 rows for CP1256
  that convert otherwise on the reference). `aliases.rs` reads the TSV at compile time (offsets, no relocations).
  WINDOWS-1252, WINDOWS-1251, MS-ANSI, KOI8R, ISO885915 open.
* **iconv -l**: lists alias rows whose charmap is an installed file or opens under another file's name (CP874 via
  IBM874's header; tried once per charmap), and that charmap name. New product test `iconv_listing_products`.
  On mei's Ubuntu 24.04 charmaps: 982 of the reference's 1,180 names (was 294 per the brief), + 26 charmap file
  names the reference does not list; 222 listed names libniji cannot open (no converter: EBCDIC/DBCS IBM sets
  without charmap, ISO-2022-*, UTF-7, ...), and `ISO/TR_11548-1`, `NF_Z_62-010_(1973)` open but are not listed.
* Fingerprints of every reference-listed name through libniji (static, `~/work/ICONV-REST/list.sh`): 851 same,
  107 different = 68 alias rows exactly as their charmap differs (pre-existing charmap-vs-module differences) + 39
  pre-existing (UTF-8/UTF-16/UNICODE built-ins, CP1255, CP1258, BIG5-HKSCS, EUC-TW, GB2312, SHIFT_JIS, IBM113x...)
  + the CP949 family (A2E8 only).
* Cost: CP949's descriptor adds 9 RELATIVE relocations in libc.so.6 (60 for all six sets, in 3 .data.rel.ro pages
  shared by the sets); layout contracts hold. Fix-forward: ICONV-CJK-STARTUP (offset descriptors).

## Not verified

* Start-up counts against next (no base products built on the role); vim's cp949 test in a trixie root (the
  whole-code-space comparison and the CP949 round trip stand in for it).

## Next step

Done: gate iconv-rest-gate1 green (16 suites, 3393 passed, 0 failed, peak 5.1G), plants iconv-rest-plants1 7 of 7
CAUGHT (`handoff/ICONV-REST-plants.txt`). Next: land; ICONV-CJK-STARTUP takes the descriptors to offsets.
