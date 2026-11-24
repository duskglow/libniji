# LOCALE-CHARMAPS-GEN: charmaps for localedef, read off libniji's own codecs

**State.** Branch `task/locale-charmaps-gen` (from origin/next `fb35fac4f`), role mei `~/w/s1`. Head: `git log -1`.
Decision D-31.22 (`records/decisions/D-31/D-31.22.toml`). COMPLETE: leg 1 wrote the code and probes, leg 2 the proof.

## What it does

* Host-tool crate `crates/charmaps` (`niji-charmaps`): `charmap(name, codec)` probes a built-in codec through
  `niji::iconv::Conv` (prefix enumeration up to 4 bytes), lists the round-trip set, coalesces `<Ua>..<Ub>` ranges,
  writes WIDTH (`width()`: C.UTF-8 width; 2 for multi-byte CJK sequences except EUC-JP SS2 kana and GB18030
  4-byte). Ten charsets: ANSI_X3.4-1968, BIG5, CP949, EUC-JP, EUC-KR, GB18030, GBK, ISO-8859-1, UTF-8, WINDOWS-31J.
  The other built-in codecs (UTF-16/32, UCS-2/4) are not ASCII-compatible: no charmap. Single-byte charsets already
  have libc6's charmaps (D-6.16); manifest lint refuses a duplicate.
* `cargo xtask so` (`xtask/src/charmaps.rs`) writes `target/niji/locale-charmaps/<NAME>.gz` (`gzip -9n`, 1 366 420
  bytes in all, GB18030 851 720); `locales` ships them at `/usr/share/i18n/charmaps/`. `locale -m` and iconv
  already read `.gz`; built-in names are found first, so no conversion changes.
* Tests: `crates/charmaps/tests/codespace.rs` (11: each charmap equals its codec's round-trip set over 0..=0x10FFFF,
  header, WIDTH); `crates/locale/tests/reference.rs` `generated_charmaps_compile_like_the_reference` (#14 set, all
  categories equal the reference) and `generated_cjk_charmaps_give_the_reference_ctype` (ja_JP EUC-JP/WINDOWS-31J,
  ko_KR EUC-KR/CP949, LC_CTYPE).

## Results (leg 2, mei s1)

* clippy (xtask and `--all-features -D warnings`), static: clean (fixed: `#![forbid(unsafe_code)]`, trace comment
  on the macro-generated tests).
* `locale-charmaps-gen-so`: `cargo xtask so` + `manifest.py lint --products`: OK (139 rows), 10 `.gz` files. Job rc
  141 is SIGPIPE from the `zcat | head` I appended to the job, after the build and lint passed.
* Leg 1 left Cargo.lock without the niji-charmaps entry (cargo rewrote it at each build); committed in c05336e81.
  Gate `locale-charmaps-gen-gate` and plants `locale-charmaps-gen-plants` (head 4dc49bb2f: 882 passed, 146/146
  conformance; 7/7 caught) are SUPERSEDED by the reruns on c05336e81 for that reason.
* Gate `locale-charmaps-gen-gate2` (direct tier, head c05336e81): 10 suites ok, 882 passed / 0 failed / 1 ignored,
  conformance 146/146, SKIPPED 0, peak 2.2G.
* Plants `locale-charmaps-gen-plants2` (`handoff/LOCALE-CHARMAPS-GEN.plants`): 7 CAUGHT, 0 survived, 0 WARN.
  GB18030's 4-byte width exception has no plant: no test can check it while the EAW gap below remains.

## Probe-settled (black box: both localedefs, compared by the reader)

* EUC-JP/BIG5/GBK/EUC-KR/CP949/WINDOWS-31J: 0 width differences with the 2-column rule (497 for EUC-JP without it).
* GB18030: 4-byte sequences keep their ordinary width (153465 differed with "2"); 45470 rows still differ: the
  reference gives East Asian *ambiguous* 4-byte characters 2 columns (needs EAW data). One LC_CTYPE translit row
  (U+1079C) because our GB18030 charmap encodes every scalar value.
* localedef's own differences (same with the installed charmaps): `LC_COLLATE collseq` of byte 255 in non-UTF-8
  multi-byte locales; class `hanzi` empty in zh_CN GB18030/GBK and zh_TW BIG5; ko_KR EUC-KR LC_MONETARY items
  1/15/23/37 (the KRW symbol).

## Next step

Land. Follow-ups (INFORMATIONAL in the report): GB18030 ambiguous widths (EAW data); the three localedef
differences above; LOCALE-SOURCES-GEN (held) is unblocked for its charmaps.
