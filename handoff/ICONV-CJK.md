# ICONV-CJK: BIG5, EUC-KR, CP932, CP936, GB18030 built in (GitHub #6)

**State.** Branch `task/iconv-cjk` (from origin/next 5e1903cce), role mei `~/w/s2`. Leg 2 built, fixed and
compared the codec. Code head 781e1a64a: clippy + static clean, plants 5/5 CAUGHT (mei:iconv-cjk-plants4,
`handoff/ICONV-CJK-plants.txt`), affected escalates to FULL (the train's), changed crates' tests green after
two stale-expectation fixes (mei:iconv-cjk-crates then mei:iconv-cjk-fix). Report: `handoff/reports/ICONV-CJK.json`.
Crate name is `niji` (`-p niji`).

## What the branch does

* One generic codec `crates/core/src/iconv/cjk.rs` over generated tables `cjk_tables.rs` (`cargo xtask
  encodings-gen`, generator `xtask/src/encodings_cjk.rs`) from vendored public tables (Unicode CP950/CP932/CP936
  2.01, KSC5601.TXT 1.1; WHATWG index-gb18030 and -ranges 2024-09-18) plus the reference's variant as data,
  `spec/encoding/reference/<SET>.tsv` (format: that directory's README.md; written by
  `tools/probe/iconv_variant.py` from the code-space probe). 21 names (`mod.rs` NAMES, `csys/util/iconv-names.h`).
* Table sizes: BIG5 82,961 B, EUC-KR 89,101, CP932 72,982, CP936 95,030, GB18030 107,594 (447,668 total,
  read-only data in libc.so.6/libc.a; no new package files, the libc6 row already ships libc.so.6).
* Decision: pending `records/pending/ICONV-CJK-1.toml` (D-NEW, series D-6, amends D-6.15).

## Probe-settled (mei, host C library Ubuntu GLIBC 2.39-0ubuntu8.9)

* Names (same converter by whole-dump hash): BIG5 BIG-5 BIG-FIVE BIGFIVE CN-BIG5 CP950 / EUC-KR EUCKR CSEUCKR
  OSF0004000A / GB18030 / CP932 MS932 WINDOWS-31J SJIS-OPEN SJIS-WIN / CP936 GBK MS936 WINDOWS-936 GB13000.
* Leg 2 findings: the reference writes each of GB18030's 19 four-byte departure code points as its departure
  pointer (generator now derives these from the `four` records); GB18030 writes U+E0000..U+E007F as four-byte
  sequences (the tag characters are dropped only when a set cannot write them).
* Code-space comparison (`iconv_codespace_products.rs`, 6 sets, GB18030 with the oracle's new `-c`: sequences of
  3+ bytes hashed per two-byte prefix, 1,160,213 lines): identical except the `//TRANSLIT` block hashes of D-6.9,
  listed per set in the test. GB18030 takes ~75-80 s per side; the test ~160 s.

## Not done / possible follow-ups

* Size: CP936 and GB18030 two-byte cells are nearly identical (could share one table); EUC-KR's encoder pages are
  sparse (Hangul). Measure before choosing.
* The test's GB18030 pass dominates its time (two 75 s walks of 82M four-byte sequences); a cheaper oracle (e.g.
  only the fourth byte's 0x30..0x39 plus one representative of each other class) would need a ruling on coverage.

## Next step

If the report says COMPLETE: nothing; the dispatcher verifies the named jobs. Otherwise follow its `next_step`.
