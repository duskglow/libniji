# UTIL-12: `niji-locale` and `localedef` (issue #14's blocker)

**State.** Branch `task/util-12` (from origin/next `90390071c`), role mei `~/w/s1`. Head: see `git log -1`.
Leg 2 (refresh 1) done: `niji_locale::compile(locale, charmap, files)` (lib.rs) and
`crates/locale/tests/reference.rs` (6 tests: en_US/de_DE/fr_FR/ja_JP/tr_TR UTF-8, en_US ISO-8859-1; spawns the
reference `localedef -i X -c -f CS`, skips with a reason when absent; `NIJI_LOCALE_KEEP=1` keeps both trees under
`$TMPDIR` for inspection). **All 6 green on mei**: every item of the 10 simple categories byte-equal, LC_CTYPE fixed
items byte-equal, every class bit / map value / width (0..0x110000) equal through `niji::locale::table3`,
transliteration equal decoded. `cargo clippy -p niji-locale --all-targets -- -D warnings` clean (job
util-12-clippy1). LC_COLLATE is not written (compile() skips category 3; the test skips it). Workspace clippy,
`cargo xtask static`, gate and plants NOT run yet. Probe scripts: `mei:~/work/util-12/{ct.py,tl.py}` (tl.py
diffs translit + LC_TIME 51 of `<ref dir> <ours dir>`).

## Settled by the comparison (leg 2; replaces the leg-1 guesses they name)

* Lexer: the comment character outside a string/symbol starts a comment to end of line (sources end entries
  with `% NAME`; without this the emoji translit failed and trailing words became bogus entries).
* MB_CUR_MAX = the charmap's declared `<mb_cur_max>` (UTF-8: 6); `niji::charmap` clamps its own copy to 4, so
  `Charset` reads the header itself.
* Byte tables (items 0/1/3): entries −128..−2 repeat bytes 128..254 with the mapped byte stored UNSIGNED; −1 (EOF)
  is class 0, maps −1.
* CLASS32 (5), TOUPPER32 (15), TOLOWER32 (16) are indexed by CODE POINT 0..255 whatever the charmap; CLASS32 =
  16-bit `__ctype_b` mask << 16. (Not tested for a single-byte charmap whose bytes are not Latin-1: guess.)
* Items 10/11 name lists and 14 CODESET pad to 4. Items 31-40 (wide indigits) one word each, no terminator;
  67 DEFAULT_MISSING words without terminator.
* Each class table item is preceded by a 32-byte bitmap of that class over the charmap's single bytes, stored at
  the end of the previous item (`File::extend_last`); the reader ignores it.
* `charclass`/`charconv` declare table order (ja_JP: combining, combining_level3, jspace, jhira, ...).
* WIDTH: U+0000 has width 0; absent ≡ 0xff (the test compares what wcwidth reads).
* Item 71 NONASCII_CASE = the byte case tables differ from ASCII's (en_US.UTF-8 0, en_US.ISO-8859-1 1, tr_TR 1);
  70 MAP_TO_NONASCII = an ASCII char maps outside ASCII (tr_TR 1).
* Transliteration: the LAST definition of a from-string wins (statements in order, an include where it stands,
  so a file's own entries beat its includes and a locale's own include beats the copied i18n's); alternatives
  written as strings/words that the charmap cannot encode are dropped (an entry may keep none); symbol
  alternatives are kept regardless. libniji stores each distinct alternative list once (to-table ~half the
  reference's: 41k vs 80k words-bytes en_US); decoded equal. **Needs a one-line D row** (local detail,
  reader-equal, smaller file): reserve with `ops reserve --story UTIL-12 D-31` (not done yet).
* ERA dates: a negative year counts from 1 BC (`-0001` -> tm_year −1900).

## Design in force (leg 1)

* New workspace crate `crates/locale` (package `niji-locale`, lib `niji_locale`; no_std + alloc, S8 TryVec),
  depending on `niji` (the reader `niji::locale` is the oracle; `niji::charmap` parses charmaps;
  `niji::charmap::inflate::gunzip` reads `.gz` charmaps). Own crate so changes reach few tests.
  Plan says writer beside reader (`niji-core::locale`); a separate crate keeps the affected tier small:
  record as a one-line D row when a D-31 id is reserved (`ops reserve --story UTIL-12 D-31`).
* Modules: `lex` (comment_char/escape_char, `/` continuation, tokens Word/Sym/Str/;/,/(/)), `source`
  (sections, `copy` resolved recursively, depth 16), `build` (category file: magic, n, offsets; word and
  wide items aligned to 4 with the pad ending the previous item: matches the reference's files), `charset`
  (charmap name, mb_cur_max, encode, WIDTH section), `cats` (the 10 simple categories), `ctype` (LC_CTYPE).
* File access through `trait Files { read(Kind::Locale|Charmap, name) }` so tests run in-process.
* Planned front end: a freestanding static `localedef` like `ldconfig` (crates/ldconfig-bin pattern:
  product crate, boundary feature, `machine/entry.S` variant, arena allocator), argv[0]-chosen later for
  `locale-gen` (UTIL-13). The existing `locale` (UTIL-8) is C (`csys/util/locale.c`) and stays.

## Probe-settled (reference localedef 2.39 on mei, format inspection of its output)

Compiled into `mei:~/work/util-12/ref/{en_US,de_DE,fr_FR,ja_JP,tr_TR}.UTF-8` and `en_US.ISO-8859-1`
(`~/work/util-12/probe1.sh`; item dumper `~/work/util-12/items.py <file> [skip,list]`).
* Magics/counts: CTYPE 0x20090720 (89 items in en_US: 72 fixed + 14 class tables + 3 maps), NUMERIC
  0x20031114/6, TIME 0x20031117/159, COLLATE 0x20051017/19, MONETARY 0x20031111/46, MESSAGES 0x20031110/5,
  PAPER 0x20031112/3, NAME 0x2003111d/7, ADDRESS 0x2003111c/13, TELEPHONE 0x2003111f/5, MEASUREMENT
  0x2003111e/2, IDENTIFICATION 0x20031119/16. Item order = langinfo.tsv index order.
* Monetary: `int_*` default to national; `duo_*` to the single-currency ones; uno/duo valid from 10101 to
  99991231; conversion_rate words [1,1]; CRNCYSTR "-$" for p_cs_precedes 1 (rule for 0/-1 a guess).
* NAME: missing name_gen = "". ADDRESS: country_isbn number stored as its decimal string; country_num word.
  IDENTIFICATION: category item = 12 NUL-ended strings (order by category number: a guess; en_US all equal).
* TIME: era (44) = 0 bytes when none; alt_digits = 100 strings (empty ones too), walt_digits 100 wide;
  week ndays byte, 1stday word, 1stweek byte; first_weekday 1 / first_workday 2 / cal_direction 1 defaults
  (en_US); timezone ""; alt_mon/ab_alt_mon default to mon/abmon. ERA_ENTRIES (51), ja_JP: per era words
  dir char, offset, start (y-1900, m-1, d), end (`+*` = INT_MAX x3), MB name\0 format\0, pad 4, wide
  name\0 format\0 (implemented; `-*` = INT_MIN a guess).
* CTYPE fixed items: 0 CLASS u16[384], 1 TOUPPER, 3 TOLOWER i32[384], 5 CLASS32 u32[256], 2/4/6-9 empty,
  10/11 names (`toupper tolower totitle`, list ended by an empty name), 12 WIDTH, 13 MB_CUR_MAX, 14 CODESET,
  15/16 TOUPPER32/TOLOWER32, 17 CLASS_OFFSET 72, 18 MAP_OFFSET 86, 19 INDIGITS_MB_LEN 1, 20-29 "0".."9",
  30 =1, 31-40 wide, 41-50 OUTDIGIT MB, 51-60 OUTDIGIT words, 61 TRANSLIT_TAB_SIZE (en_US 6398), 62-65,
  66 DEFAULT_MISSING_LEN 1, 67 L"?", 68 0, 69 empty (0 bytes), 70 0, 71 word 0 (36 bytes in the file).

## Guesses still open (not reached by the six locales)

CLASS32/TOUPPER32 for a single-byte charmap that is not Latin-1 (e.g. KOI8-R: add one case to reference.rs),
INDIGITS with `outdigit`, unencodable characters with `-c` outside translit (written: dropped),
week_1stweek default (4). The plan row wants "all SUPPORTED locales": extend reference.rs with an
ignored-by-default sweep over `/usr/share/i18n/SUPPORTED` later (cost: ~1 s per locale on mei, debug).

## Exact next step

1. DONE (leg 2): reference.rs green. Reserve a D-31 id and record the translit-sharing row (above).
2. LC_COLLATE (largest piece; spec D.1.4 and `spec/tools/collate_proto.py` model the reader); compare
   strcoll/strxfrm/wcsxfrm order over a corpus via `niji::locale` (byte-equal keys reported, order required).
3. Archive writer (D.1.6; `niji::locale::archive` reads), the `localedef` program and options (spec §31.8),
   manifest row `system libc-bin usr/bin/localedef`, transcripts, plants (`docs/plants.md` form), gate.
4. RULING NEEDED to raise: shipping Devuan's locale sources/charmaps in `locales` (R5 says "Devuan's data
   compiled by libniji's localedef"; they come from glibc's source package; not committed to this tree).

## Budget notes

Leg 1: 80 calls, ~190k context (format probing and design took most). Keep probe output small.
Leg 2: ~115 calls, ~150k context: six comparison/fix rounds (each one jobrun `cargo test -p niji-locale`
on mei, ~10 s warm). Wall deadline 2026-11-22T08:02:37Z: leave >= 45 min for gate + plants + precheck.
