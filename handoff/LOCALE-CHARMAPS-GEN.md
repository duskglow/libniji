# LOCALE-CHARMAPS-GEN: charmaps for localedef, read off libniji's own codecs

**State.** Branch `task/locale-charmaps-gen` (from origin/next `fb35fac4f`), role mei `~/w/s1`. Head: `git log -1`.
Decision D-31.22 (`records/decisions/D-31/D-31.22.toml`). Leg 1 of the run wrote the code and the probes; leg 2
(after a REFRESH) runs the proof jobs and hands back.

## Done (do not redo)

* New host-tool crate `crates/charmaps` (`niji-charmaps`, workspace member, dev opt-level 2): `charmap(name, codec)`
  probes a built-in codec through `niji::iconv::Conv` (prefix enumeration up to 4 bytes), lists the round-trip set,
  coalesces `<Ua>..<Ub>` ranges, writes a WIDTH section (`width()`: C.UTF-8 width; 2 for multi-byte sequences except
  EUC-JP SS2 kana and GB18030 4-byte). Ten charsets: ANSI_X3.4-1968, BIG5, CP949, EUC-JP, EUC-KR, GB18030, GBK,
  ISO-8859-1, UTF-8, WINDOWS-31J (all builtin names; the reference's `locale -m` names). Bin `niji-charmaps DIR`.
* Test `crates/charmaps/tests/codespace.rs` (11 tests): every listed mapping round-trips through the codec; every code
  point 0..=0x10FFFF: charmap encodes it iff the codec round-trips it, same bytes; mb_cur_max; WIDTH exact. Green
  (release 9 s, job locale-charmaps-gen-t1).
* `cargo xtask so` (`xtask/src/charmaps.rs::write_generated`, called in so.rs after the libc6 charmaps) runs the bin
  into `target/niji/locale-charmaps/` and `gzip -9n`s each. Manifest: 10 rows `system locales
  usr/share/i18n/charmaps/<NAME>.gz` (source `locale-charmaps/<NAME>.gz`); manifest.py lint refuses a generated
  charmap that duplicates a libc6 one, and `check` gunzips and checks `<code_set_name>`. Lint OK locally.
* `crates/locale/tests/reference.rs`: `Generated` Files (charmaps from niji_charmaps), `compare_with(.., ctype_only)`,
  tests `generated_charmaps_compile_like_the_reference` (the #14 set: en_US/de_DE/fr_FR/ja_JP/tr_TR UTF-8 +
  en_US ISO-8859-1; all 6 equal the reference, job t2) and `generated_cjk_charmaps_give_the_reference_ctype`
  (ja_JP EUC-JP/WINDOWS-31J, ko_KR EUC-KR/CP949, LC_CTYPE only).
* `locale -m` (csys/util/locale.c) already lists every file of the directory and reads `.gz` headers; iconv reads
  `.gz` too and finds built-in names first, so the new files change no conversion. Nothing to change there.

## Probe-settled (black box: both localedefs, compared by the reader)

* EUC-JP: with C.UTF-8 widths 497 widths differed (reference 2 for every JIS X 0208/0212 char); with the 2-column
  rule none. BIG5/GBK/EUC-KR/CP949/WINDOWS-31J: no width difference with the rule.
* GB18030 4-byte sequences: the reference gives them their ordinary width (153465 differed with "2"). Remaining
  GB18030 width rows after the fix: see t5 (combining marks ref 1, ~44k unassigned code points absent from the
  reference's charmap).
* GB18030 after the 4-byte fix (job t5): 45470 width rows differ, e.g. U+00A1 ref 2 ours 1 while U+00A2 ref 1:
  the reference gives East Asian *ambiguous* 4-byte characters 2 columns. Needs EAW data: next story
  (INFORMATIONAL). Also one LC_CTYPE transliteration row (U+1079C -> U+1DF04): our GB18030 charmap encodes every
  scalar value, the reference's lacks code points unassigned in its Unicode version.
* Differences that are localedef's, not the charmaps' (job t5: the same with the installed charmaps, libniji's
  localedef): `LC_COLLATE collseq` for byte 255 in every non-UTF-8 multi-byte locale; class `hanzi` empty in
  zh_CN GB18030/GBK and zh_TW BIG5 (20923 code points); ko_KR EUC-KR LC_MONETARY items 1/15/23/37 (the KRW symbol).
  With the installed charmaps libniji's localedef also gets widths wrong (ja_JP.EUC-JP 6944, zh_CN.GBK 870,
  ko_KR.EUC-KR 4073, zh_CN.GB18030 24995); with the generated ones those are 0, 0, 0 and 45470.

## Exact next step (leg 2)

1. (done in leg 1: t5 read, attribution above, temporary test removed.) Jobs so far: t1 (crate tests), t2/t3/t4
   (probes, SUPERSEDED: earlier heads), t5 (tests 11/11 + 2/2 green on the leg-1 head).
2. `cargo xtask clippy`, `cargo xtask static` on the role; `cargo xtask so` once (checks the new so step + gzip),
   then `python3 tools/deb/manifest.py lint --products target/niji`.
3. Gate (direct tier): `cargo xtask affected --run` via jobrun (label locale-charmaps-gen-gate).
4. Plants file `handoff/LOCALE-CHARMAPS-GEN.plants` on crates/charmaps/src/lib.rs (range merge `lb.checked_add(1)`
   -> `add(2)`; width rule narrow EUC-JP SS2; mb_cur_max 6 -> longest; round-trip filter dropped), run
   `cargo xtask plants` (label locale-charmaps-gen-plants, dir /mnt/zram/claude/locale-charmaps-gen/plants-1).
5. Report json, push, precheck, handback-check.
