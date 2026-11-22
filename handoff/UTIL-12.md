# UTIL-12: `niji-locale` and `localedef` (issue #14's blocker)

**State.** Branch `task/util-12` (from origin/next `90390071c`), role mei `~/w/s1`. Head: see `git log -1`
(leg 1 ended at "niji-locale compiles"). `cargo clippy -p niji-locale --all-targets -D warnings` clean and
`cargo test -p niji-locale` green on mei (2 unit tests). Nothing compared with the reference yet. NOT VERIFIED:
every format detail below marked (guess).

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

## Guesses to settle by comparison (next leg)

CLASS32 bit layout (written as 1<<k), the byte tables at −1 (EOF) and −128..−2, TOUPPER32 for bytes with no
character, which code points get a WIDTH (written: `print` and in the charmap), translit precedence for
duplicate from-strings (written: first definition wins) and include order, MAP_TO_NONASCII/NONASCII_CASE,
INDIGITS with `outdigit`, unencodable characters with `-c` (written: dropped), week_1stweek default (4).

## Exact next step

1. `crates/locale/tests/reference.rs`: a `Files` over `/usr/share/i18n/{locales,charmaps}` (gunzip `.gz`);
   for en_US/de_DE/fr_FR/ja_JP/tr_TR UTF-8 and en_US ISO-8859-1: compile the reference with
   `localedef -i X -c -f CS <tmp>/X` (skip with a printed reason when absent), compile ours in-process,
   compare every simple category item byte for byte (via `niji::locale::file::Category::parse` on leaked
   bytes), and LC_CTYPE semantically: fixed items 0-71 bytes (except table items 12) and, for all
   0..0x110000, every class bit, map value and width through `niji::locale::table3::Table3`. Print the first
   10 differences per item, then fix the writers until equal.
2. LC_COLLATE (largest piece; spec D.1.4 and `spec/tools/collate_proto.py` model the reader); compare
   strcoll/strxfrm/wcsxfrm order over a corpus via `niji::locale` (byte-equal keys reported, order required).
3. Archive writer (D.1.6; `niji::locale::archive` reads), the `localedef` program and options (spec §31.8),
   manifest row `system libc-bin usr/bin/localedef`, transcripts, plants (`docs/plants.md` form), gate.
4. RULING NEEDED to raise: shipping Devuan's locale sources/charmaps in `locales` (R5 says "Devuan's data
   compiled by libniji's localedef"; they come from glibc's source package; not committed to this tree).

## Budget notes

Leg 1: 80 calls, ~190k context (format probing and design took most). Keep probe output small.
