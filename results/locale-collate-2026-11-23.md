# Reference collation order by black-box probe (LOCALE-COLLATE-TAILORINGS, 2026-11-23)

Step 1 of LOCALE-COLLATE-TAILORINGS (coordinator's amended scope, #7950): settle #7795 and derive the reference's
collation order per locale, enough to reconstruct every weight level it distinguishes. Probes and how to run them:
`tools/probe/collate/README.md`. Data: `results/locale-collate-2026-11-23/`. Reference: glibc 2.39 (Ubuntu
2.39-0ubuntu8.9), x86-64. Locales were compiled with the reference's installed `localedef` from its installed data,
then called only through `setlocale`/`strcoll`/`strxfrm`/`wcscoll`. No reference code was read, and no reference
locale source file was viewed (the #7791 allowance was not used).

## 1. #7795: "sv_SE/es_ES/tr_TR strcoll look like byte order": settled, not the reference's order

Three of the four build hosts have only `C`, `C.utf8`, `en_US.utf8` and `POSIX` compiled. There `setlocale(LC_ALL,
"sv_SE.UTF-8")` returns NULL, the program stays in the C locale and `strcoll` is byte order. When the same locales
are compiled (privately with `LOCPATH`, or installed system-wide as on the CI host), the reference applies its
tailorings. On the same reference build, the two setups give byte-identical results: `collprobe` output, and
`collorder`'s full sv_SE table, have the same md5. Comparison logs must check `setlocale`'s return; `collprobe`
prints it.

## 2. The seven locales (word lists in `collprobe.c`; output `collprobe-seven-locales.txt`)

| locale | setlocale (no locale / compiled) | reference order on the word list | rules vs root | case first | reference `localedef` | `LC_COLLATE` | libniji on next |
|---|---|---|---|---|---|---|---|
| sv_SE | NULL / sv_SE.UTF-8 | TAILORED: az zz åa äa öa (å Å, then ä Ä æ Æ ę, then ö Ö ø ő œ ô, all after z; ü sorts with y) | 33 | lower | 8.1 s | 2,586,726 B | no sv_SE source (setlocale fails) |
| es_ES | NULL / es_ES.UTF-8 | TAILORED: nz ña oa (ñ Ñ a letter after n) | 5 | lower | 8.1 s | 2,586,930 B | no source |
| tr_TR | NULL / tr_TR.UTF-8 | TAILORED, not CLDR's: Ia ıa İa ia iz ja (I and ı one letter, I first; then İ and i) | 2,349 vs en_US; 16 vs the upper-first base | upper | 8.7 s | 2,586,730 B | no source |
| pl_PL | NULL / pl_PL.UTF-8 | TAILORED: ą ć ę ł ń ó ś ź ż each a letter after its base (az < ąa) | 21 | lower | 8.0 s | 2,586,758 B | no source |
| zh_CN | NULL / zh_CN.UTF-8 | TAILORED: 阿 八 你 我 中 (Han in pinyin order) | own order: 25,950 Han moved | lower | 11.2 s | 2,801,698 B | no source |
| ja_JP | NULL / ja_JP.UTF-8 | own model: あ か ア カ 亜 日. Hiragana before katakana at level 1 (not CLDR's tertiary pairing), Han after kana; one weight level; code points outside its set all compare equal | own order: 13,136 level-1 elements | n/a | 4.4 s | 439,686 B | no source |
| ko_KR | NULL / ko_KR.UTF-8 | own model: 가 나 伽 羅. Hanja after all Hangul (not next to its reading); one level, as ja_JP | own order: 17,143 level-1 elements | n/a | 5.1 s | 584,930 B | no source |

"Rules vs root" counts `colldiff.py` rules (moved elements plus level changes) against en_US's order. libniji "after"
is not measured: steps 2-4 were not started, on the coordinator's instruction (the generator they need,
LOCALE-SOURCES-GEN-2, is not on next). `localedef` times are wall time on a 60-vCPU Zen 1 host running 12 at once
beside other builds. They are reported, not compared.

## 3. Survey: every UTF-8 locale the reference lists as supported (327)

`collorder` over U+0020-U+D7FF, U+F900-U+FFFD and U+10000-U+1FFFF (122,590 code points) per locale, diffed against en_US
(`diffs.tsv`, `rules/<locale>.rules`).

* **72 distinct orders.** 206 locales are identical to en_US, including de_DE, fr_FR, ru_RU, zh_TW and zh_HK
  (zh_TW and zh_HK apply no Han tailoring). The next groups: es_ES's order is shared by 30 locales (Spanish variants,
  ca, gl, ast, an, oc); bs/hr/sr 5 (19 rules); da/fo/kl 3; bo/dz 3; cs/sk 2. 54 orders belong to one locale each.
* **Rules against en_US:** 0 for 206 locales; 1-10 for 56; 11-100 for 34; 101-1,000 for 7; 1,001-10,000 for 17;
  over 10,000 for 7. Of the 17, 14 sort upper case first (`case-first=upper` in `summaries.txt`). Against en_CA
  (en_US's order with upper case first; `rules/*.vs-en_CA.rules`), they need: tr_TR and tr_CY 16, et_EE 67, iu_CA
  0, shs_CA 3, da/fo/kl 749, nb/nn 754, mt_MT 741, uk_UA 1,115. fr_CA needs 563 level changes and no moves (it
  compares secondaries backward). cs_CZ, sk_SK and se_NO sort lower case first and need about 1,000-1,200 rules each.
* **Own orders (over 10,000 rules):** zh_CN (25,958), cmn_TW (27,526), ja_JP, ko_KR, lo_LA and km_KH (44k-61k), and
  C.UTF-8 (code point order). These are their own tables, not tailorings.
* **Total:** the 320 other locales need 16,151 rules in all, taking the smaller of the en_US and en_CA bases per
  locale.

### Weight levels the reference distinguishes

* **Four levels.** Over the probed range, en_US has 36,140 level-1, 962 level-2, 2,889 level-3 and 9,787 level-4
  steps. 72,787 code points compare equal to a neighbour. `collorder`'s fillers check per locale that the level test
  holds. `fillers-ok=0` in 14 locales. In 5 of them the test does not apply: C.UTF-8 and the one-level models
  ja_JP, ko_KR, km_KH and lo_LA. In the other 9 (da_DK, fo_FO, is_IS, kl_GL, lt_LT, lv_LV, nb_NO, nn_NO, se_NO), a
  filler letter (a, á or A) is itself tailored, so **their level labels are not reliable** until the probe is rerun
  with fillers chosen per locale. Their order is unaffected: it comes from the `strxfrm` sort. Their large
  level-change counts against the root may partly be this artifact.
* **"Level 5" is an artifact.** The 25 en_US steps labelled beyond level 4 are of two kinds. Most are expansions
  (one code point collating as several elements, such as ⑽ and ㌃), where the trailing-filler test meets the
  expansion's second element. The others are the boundaries between undefined classes. The levels of expansions need
  their own probe (section 5).
* **Undefined code points.** Code points the reference's table does not define (unassigned ones, CJK Extension A,
  and the 3,450 code points DUCET 15.1 defines but the reference does not) all sort **first**, compare equal under
  `strcoll` within their class, and form three classes, by UTF-8 length (2, 3 and 4 bytes), split beyond level 4.
  Between those classes `wcscoll` returns 0 where `strcoll` returns -1 (the only `strcoll`/`wcscoll` disagreements
  found: 2 pairs in 324 locales, 3 in 2, none in C.UTF-8). `strcoll` and the `strxfrm` key order agree on every adjacent pair in every locale.
* **No fully ignorable code point in U+0020 and above** (`strcoll(c, "") != 0` for all). C0/C1 controls and format
  characters differ from each other only at level 4.
* **Secondary direction:** forward everywhere except fr_CA (backward). km_KH and ko_KR also report backward, but the
  test does not apply to their one-level models. **Case first:** lower, except the 14 locales above (and C.UTF-8,
  ja_JP, ko_KR and lo_LA, where "A" before "a" follows from code point or own-model order).

### Contractions (`contractions.tsv`)

36 locales contract two characters within the candidate set (Latin letters U+0041-U+017F, plus each locale's moved
code points when at most 300). Examples: cs_CZ and sk_SK ch; cy_GB ch dd ff ll ng ph rh th; hu_HU cs dz gy ly ny sz
ty zs; sq_AL dh gj ll nj rr sh th xh zh; bs/hr/sr lj nj and a d-pair (reported as dþ and dŉ: the out-of-order set can
name a contraction's neighbours instead of the contraction itself, here most likely dž, which needs a direct check);
da/fo/kl/nb/nn aa; br ch; fil and tl ng; mt għ; bo/dz 40 Tibetan pairs; ps_AF 45; ig_NG 36; om 24; th_TH 2. Each is
reported in every case form (ch Ch cH CH). Three-character
contractions (hu_HU dzs and others) and contractions outside the candidate set are not probed. Three hu_HU entries are
pairs of combining marks and need a direct check before use.

## 4. The root order against DUCET (`root-vs-ducet-15.1.rules`)

The reference's defined set in the probed range (49,800 code points) is a strict subset of what DUCET 15.1
(`spec/unicode/15.1.0/allkeys.txt`) defines: every defined code point is in DUCET or is Han. On that set, en_US's order
is DUCET's (shifted variable weighting) plus:
* 9,566 pairs that tie on DUCET's three levels and that the reference splits at level 4;
* 1,146 moved code points. About 950 are Han variants: CJK compatibility ideographs (F9xx, FAxx), Kangxi and CJK
  radicals (2Fxx, 2Exx), and enclosed ideographs (32xx, 1F2xx). The rest are math symbols (22xx, 38), punctuation
  (20xx, 30), the order among C0/C1 controls at level 4, and small groups.

So the shared root can be generated from the spec's DUCET, the reference's defined set (as ranges), a level-4
tie-break, and about 1,150 placement rules, instead of committing a 120k-line probed table. Whether the level-4
tie-break is code point order everywhere needs one more probe (it is wherever no move was recorded).

## 5. Not settled by these probes (for the steps 2-4 story)

* Expansions: which code points expand, and to what (for example æ, ß, œ, ⑽). Method: compare each code point
  with the string of its compatibility decomposition (UnicodeData.txt) and with common expansion candidates at each
  level.
* Three-character contractions, and contractions outside the candidate set (other scripts).
* Planes 2 and above (CJK Extension B onward), and U+0000-U+001F.
* Whether a locale's own-order table (ja_JP, ko_KR, zh_CN, cmn_TW, lo_LA, km_KH) is better stored as a full list or
  as rules.

## 6. Size estimate for steps 2-4

* **Data:** one root (DUCET-derived plus about 1,150 rules plus defined ranges), an upper-first switch or base, and
  16,151 tailoring rules over 320 locales. Plus 6 own-order tables of 26k-61k elements: ja_JP, ko_KR, zh_CN, cmn_TW,
  lo_LA and km_KH. C.UTF-8 already exists.
* **localedef:** libniji's `LC_COLLATE` parser already takes `collating-symbol`, `collating-element` (contractions),
  `order_start` with `backward`/`position`, `reorder-after`, `UNDEFINED`, `IGNORE` and ellipses. `reorder-sections-after`
  is refused and is not needed for any order found here. No extension is known to be needed. Generated-source
  size, compiled size and libniji's localedef time per locale are to be measured in that story.
* **Work:** the generator (probe data to LC_COLLATE sections, on top of LOCALE-SOURCES-GEN-2 and LOCALE-PROBE-GEN-2),
  the expansion and three-character contraction probes, a root-vs-DUCET generator, tests per locale on the word
  lists plus a sampled full-order comparison, and plants (one tailoring dropped). Size L: about 250-350 tool calls
  in one run, or two runs (root and generator; then tailorings, own orders and tests).

## 7. The shared root generated from DUCET 15.1 (LOCALE-COLLATE-TAILORINGS-2, coordinator decision #7958)

Probe: `collorder en_US.UTF-8 1 D7FF E000 10FFFF` on the Zen 1 host (all of Unicode but NUL and the surrogates,
1,112,063 code points, about 8 s). Over the survey's range it reproduces section 3's en_US output exactly. This
settles section 4's open question and section 5's "planes 2 and above, U+0001-U+001F" item for the root.

* **Level-4 tie-break.** Listing the reference's order with DUCET 15.1 shifted weights gives the reference's level
  for every adjacent pair but 45 of 50,708. 23 of the 45 are the level-5 lines next to an expansion (section 3). The
  shifted-ignorable group (7,361 code points with no weight at levels 1-3) is in code point order at level 4, not in
  DUCET primary order. Among characters equal at levels 1-3, code point order holds wherever no rule says otherwise.
* **Two class rules explain almost all of section 4's 1,146 moves.**
  *han-variant-block*: the 1,508 CJK radicals, Kangxi radicals, compatibility, parenthesized, circled, squared and
  kanbun ideographs form one block after all other scripts and before U+4E00, in DUCET order among themselves.
  *format-first*: the 467 code points with all-zero DUCET weights that are Cc, Cf or Variation_Selector come first
  at level 4, in code point order.
* **Defined set.** 50,709 code points are defined. The DUCET single-code-point entries and the Unified_Ideograph
  code points the reference leaves undefined form 127 ranges. Of the unified ideographs, only U+4E00-U+9FA5 are
  defined: Extension A, Extension B and later, and U+9FA6-U+9FFF are undefined.
* **Placements.** 33 `after` lines: U+FFFD before the unified ideographs, Arabic high-hamza letters, Latin letters
  whose DUCET 15.1 weights differ (r rotunda, insular s, long s with dot), Tibetan and Sharada marks (level 4) and a Phaistos mark (level 2),
  Arabic prepended concatenation marks and invisible operators (Cf, outside format-first), and a few letters
  (Kharoshthi, Myanmar, Javanese, Hentaigana, Bopomofo). The single code point order cannot settle 3 `expand`,
  1 `same` and 2 `contract` lines (l·, L·). The generated_root test settled them with strings and fillers.

Result: `spec/locale/collate-root.txt` (228 lines: 2 classes, 127 undefined ranges, 33 after, 1 same, 3 expand,
2 contract) replaces the probed 120k-line table and section 4's estimate of about 1,150 rules.
`tools/probe/collate/root_rules.py` derives it from the probe output, and its simulation of the generator leaves no
order or level mismatch. Generated by `crates/collate-root` and compiled by libniji's localedef library, the root is
equal to the reference's compiled en_US. The check is `crates/locale/tests/reference.rs::
generated_root_collation_is_the_references_en_us`: all 1,112,063 code points are in the reference's strxfrm order,
every adjacent pair is at the reference's level, and the corpus matches for strcoll and wcscoll.

Still open:
* collseq (the order of bracket ranges in fnmatch and regex) differs. The reference puts U+03B1 before U+0041,
  although strcoll orders A first. The test prints the count without asserting it, and the difference needs its own
  probe.
* Contractions in the root beyond l·/L· are not probed. DUCET has more (Cyrillic, Tibetan and others); the corpus
  found only l·.
