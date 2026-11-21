# ICONV-REST: CP949/UHC, WINDOWS-125x spellings, iconv -l (issue #6, the rest)

**State (leg 1 end, WIP).** Branch `task/iconv-rest` from origin/next `4f32aa946`, role mei `~/w/s3`. All code
written, NOT yet compiled or tested: job `iconv-rest-check1` (mei) was started on the leg-1 head
(encodings-gen --check, xtask clippy, niji lib iconv tests, s06 `s6_4_c`, `iconv_cmd_names`): read
`~/logs/iconv-rest-check1.log` first. ICONV-CJK-STARTUP (relocation-free descriptors) was still running at
admission, not landed: CP949 uses the current `Set` layout (8 more RELATIVE relocations until that story's
generic conversion covers the sixth row; measure and report as `cost`, fix-forward ICONV-CJK-STARTUP).

## Done (leg 1)

* Probes (mei, host Ubuntu GLIBC 2.39-0ubuntu8.9; scratch `~/work/ICONV-REST/`): reference `iconv -l` = 1,180
  names; CP949 whole code space (`dump-CP949.txt`, `ambig-CP949.txt`). CP949 = UHC (same fingerprint); names that
  open: `CP949 UHC MSCP949 OSF100203B5` (any case); `CP-949 MS949 WINDOWS-949 KS_C_5601-1987 KSC5601` do not.
  `WINDOWS-1252`, `windows-1251`, `MS-ANSI`, `KOI8R`, `ISO885915` open on the reference; `WINDOWS1252` does not.
* CP949: vendored `spec/encoding/unicode/VENDORS/MICSFT/WINDOWS/CP949.TXT` (Unicode, 2.01, 1/7/2000, sha256
  f57e6fef...); variant `spec/encoding/reference/CP949.tsv` (no cell/encode differences from CP949.TXT: only
  leads 81-C8,CA-FD, trails, skip1 rows, ignore-result error); generator row (`xtask/src/encodings_cjk.rs`),
  `cjk_tables.rs` regenerated on the role (CP949 108,801 B); `Which::Cp949` appended last (tag 21; untag
  16..=21); NAMES +4 CP949 names.
* Why WINDOWS-1252 failed: the name is an alias only in libniji's generated charmaps (charmaps.tsv), not in the
  distro's CP1252 header; the alias table (106 rows, same-letters-and-digits rule) did not have it.
* New reproducible tools: `tools/probe/iconv_fingerprint.c` (from ICONV-NAME-MATCH-2's fp.c) and
  `tools/probe/iconv_aliases.py` (run on mei: `python3 -I tools/probe/iconv_aliases.py <fp bin>
  /usr/share/i18n/charmaps <outdir>`, jobs iconv-rest-aliases3). Output: `spec/encoding/iconv-aliases.tsv`
  (658 rows; all 104 old rows reproduced; no conflict with generated charmaps) and `csys/util/iconv-aliases.h`.
  216 listed names have no libniji converter (`unsupported.txt` in scratch: IBM EBCDIC/DBCS sets with no installed
  charmap, ISO-2022-*, UTF-7, ...); `ISO/TR_11548-1` left out (a `/` in the name).
* 19 listed names convert as a built-in without being one: added to NAMES and `csys/util/iconv-names.h`
  (ANSI_X3.4, IBM891/903 + CP/CSIBM, OSF0001002x.., CSWINDOWS31J, OSF05010001, UNICODE, OSF0001010x). NAMES 101
  rows + 7 LATIN1_MORE = 108 bits of BY_LEN's 128.
* `aliases.rs` now reads the TSV at compile time (include_bytes, const-fn row offsets; no pointers).
* `iconv -l` (`csys/util/iconv.c`): also lists alias rows whose charmap file opened in the dir scan.
* Tests: s6_4_c_1_charmap_aliases (WINDOWS-1252/1251, MS-ANSI, >= 658 rows), new
  `iconv_listing_aliases_match_the_library`, cjk_round_trips CP949 17,048, codespace test + CP949 (TRANSLIT_BLOCKS
  for CP949 copied from EUC-KR as a first guess: fix from the test's output).

## Next steps (leg 2)

1. Read check1's log; fix compile/test failures. 2. `cargo xtask so`; run `iconv_codespace_products` (CP949
   block list) and `iconv_products`; `iconv -l` diff before/after vs reference on mei (expect ~1,180 - 216 + extra
   stems); fingerprint every listed name through libniji (iconv_fingerprint.c linked static against
   target/niji/libc.a like the oracle) vs the reference. 3. Docs: `spec/encoding/iconv-names.md` rule 5 (fingerprint
   rule, TSV) + new rule for built-in aliases; `spec/encoding/unicode/README.md` CP949 row; reference README list;
   pending decision record (`ops reserve --story ICONV-REST D-6` or D-NEW) amending D-6.18 and D-6.17.
4. Plants (`docs/plants.md`), direct gate (`cargo xtask affected` may say FULL: then precheck + crate tests),
   start-up counts + relocation count vs next, clippy/static, precheck, report json, handback-check.
