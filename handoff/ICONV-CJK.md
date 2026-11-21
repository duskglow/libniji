# ICONV-CJK: BIG5, EUC-KR, CP932, CP936, GB18030 built in (GitHub #6), leg 1 handoff

**State.** Branch `task/iconv-cjk` (from origin/next 5e1903cce), role mei `~/w/s2`. Leg 1 wrote the code and
data; NOTHING IS COMPILED OR TESTED YET (no local cargo: build only on mei). Crate name is `niji`
(`-p niji`), not `niji-core`.

## Done (leg 1)

* Probe (mei, host C library Ubuntu GLIBC 2.39-0ubuntu8.9), scratch `~/work/iconv-cjk/` on mei:
  `crates/core/tests/oracle/iconv_codespace.c` dumps `dump_<SET>.txt` (GB18030's is 3.7 GB, 165M lines: every
  four-byte sequence). Names: each candidate name's whole dump hashed (`fp.txt`): same converter as
  * BIG5: BIG5 BIG-5 BIG-FIVE BIGFIVE CN-BIG5 CP950
  * EUC-KR: EUC-KR EUCKR CSEUCKR OSF0004000A
  * GB18030: GB18030 only
  * CP932: CP932 MS932 WINDOWS-31J SJIS-OPEN SJIS-WIN (SJIS/SHIFT_JIS and IBM-943 are other converters)
  * CP936: CP936 GBK MS936 WINDOWS-936 GB13000 (GB2312/EUC-CN another converter)
* Sources chosen by fewest differences from the probe: BIG5 = Microsoft CP950.TXT (0 differing cells, +408
  cells C6A1..C8FE -> U+F6B1.. one run; BIG5.TXT had 267 differing), EUC-KR = KSC5601.TXT's KS X 1001 part
  (+3 cells A2E6..A2E8), CP932 = CP932.TXT (+ user area F040..F9FC -> U+E000.., 382 encode choices for
  duplicated NEC/IBM cells), CP936 = CP936.TXT (exact), GB18030 = WHATWG index-gb18030 (1 cell A3A0 differs)
  + index-gb18030-ranges (19 four-byte pointers differ, incl. 7457). Vendored unchanged under
  `spec/encoding/unicode/...` and `spec/encoding/whatwg/`.
* Variant data `spec/encoding/reference/<SET>.tsv`, written by `tools/probe/iconv_variant.py` (records: leads,
  trails, single, cells runs, four, encode, skip1, ignore-result). Skips settled with a second probe
  (invalid pair + a byte completing a character with the trail byte).
* Generator `xtask/src/encodings_cjk.rs` (part of `cargo xtask encodings-gen`; writes
  `crates/core/src/iconv/cjk_tables.rs`, committed). Codec `crates/core/src/iconv/cjk.rs` (one generic codec;
  GB18030 four-byte code: invalid 4 if third byte is a lead else 3; L+digit+anything is incomplete until 4
  bytes). Wired in `mod.rs`: `Codec::Cjk(Which)`, tags 16..=20, 21 names in NAMES (BY_LEN widened to u128),
  CP932 `//IGNORE` decode reports success (probed: r=0), the others EILSEQ.
* Table sizes (generator output): BIG5 82,961 B, EUC-KR 89,101, CP932 72,982, CP936 95,030, GB18030 107,594.

## Exact next steps

1. On mei: `cargo test -p niji --lib iconv` (unit tests in cjk.rs: counts 13912/8259/9667/21792/23940 are
   my arithmetic from the probe, check), `cargo xtask clippy`, `cargo xtask static`. Fix compile errors
   (other exhaustive `match Codec` sites outside mod.rs may exist).
2. Conformance: add the five to `CHARSETS` in `crates/core/tests/iconv_codespace_products.rs`. GB18030's dump
   is 3.7 GB: give the oracle a compact mode for sequences of 3+ bytes (e.g. one hash line per two-byte prefix,
   like BT/BI) so both sides stay small; then run it (needs `cargo xtask so`). TRANSLIT block lists per
   set will be needed (D-6.9), as for EUC-JP: take them from the first run's diff, BT lines only.
   Also add the five names to whatever `iconv -l`/names tests exist (`crates/core/tests/iconv_cmd_names.rs`).
3. Plants (`docs/plants.md`): one on a table entry (e.g. a `cells` line in spec/encoding/reference/BIG5.tsv or
   a value in cjk.rs decode), one on the GB18030 four-byte range (e.g. `189_000` in `four_cp`).
4. Decision record (pending D-NEW amending D-6.8, cite sources + versions: Unicode CP950/CP932/CP936 table
   version 2.01, KSC5601.TXT table 1.1 2011-10-14, WHATWG index-gb18030 / ranges 2024-09-18 with their
   identifiers), READMEs (`spec/encoding/unicode/README.md`, `spec/encoding/whatwg/README.md`, new
   `spec/encoding/reference/README.md` = record format), xtask/main.rs help line, affected.rs map for
   `cjk_tables.rs` -> encodings-gen-check. Packaging: built into libc (like EUC-JP), check the manifest row.
5. Gate (direct tier) + plants + precheck; report json `handoff/reports/ICONV-CJK.json`.

## Possible size work (not done)

CP936 and GB18030 two-byte cells are nearly identical (could share one table); EUC-KR's encoder pages are
sparse (Hangul): 32-code-point pages or sorted pairs would shrink it. Measure before choosing.
