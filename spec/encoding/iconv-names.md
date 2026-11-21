# Charset names in `iconv_open` (§6.4; issue #6)

libniji's rule for the name argument of `iconv_open` (and every caller that names a charset the
same way: `bind_textdomain_codeset`, `fopen`'s `ccs=`). Decision: `records/pending/ICONV-NAME-MATCH-1.toml`.

## The rule

1. **Name and suffixes.** The argument is `PART[/PART2][/SUFFIXES]`: the name ends at the second
   `/`. The suffixes (`TRANSLIT`, `IGNORE`, separated by `/` or `,`, any case) are unchanged by this rule.
2. **Dropped bytes.** In each part, only letters, digits and `, - . / : _` count. Every other byte
   is removed: space, tab and other controls, `! " # $ % & ' ( ) * + ; < = > ? @ [ \ ] ^` and the
   backquote, `{ | } ~`, and every byte from 0x7F up.
3. **Case.** Letters are uppercased.
4. **Commas and the second part.** Trailing commas of the second part are removed. If the second
   part is then empty, the name is the first part without its trailing commas. Otherwise the name
   is `PART/PART2`, and commas at the end of the first part stay.
5. **Match.** The result must equal a known name exactly. The known names are the built-in names
   and each installed charmap's `<code_set_name>`, aliases and file name, after steps 2 and 3. The
   punctuation that counts is not ignored: `CP-1252` is not `CP1252`. A spelling the system knows
   (for example `ISO88591` beside `ISO-8859-1`) is a listed name of its own.
   The known names also include libniji's table of listed names that stand for a charmap
   (`crates/core/src/iconv/aliases.rs`, 106 rows, for example `KOI8R` for `KOI8-R` and
   `ISO885915` for `ISO-8859-15`). These names work whichever charmap files are installed, as on
   the reference system, where the names do not come from the charmap files. The table holds every
   name `iconv -l` lists that is not a name of an installed charmap or a built-in, but has the same
   letters and digits as one. Each row passed a black-box check on the reference: both names decode
   every one- and two-byte input and encode every code point (all of the BMP and a stride of the
   rest) the same way. Listed names without such a partner (`BIG-FIVE`, `CP1140`, ...) are not in
   the table; they are an open item (`handoff/ICONV-NAME-MATCH.md`).
6. An empty result is the current locale's codeset. A result longer than 128 bytes names nothing (`EINVAL`).

Locale codeset names are matched differently (Appendix D.1, letters and digits only); that rule is unchanged.

## Probe table (black box, reference system, glibc 2.39, 2026-11-17)

Every spelling below was applied to each of the 1,180 names `iconv -l` prints. A spelling counts
as "opens" when `iconv_open` succeeds both to and from UTF-8. "Same codec" means the same
conversion fingerprint as the listed name: every byte decoded, eight characters encoded.

| spelling class | example | opens | same codec |
| --- | --- | ---: | ---: |
| as listed | `ISO-8859-1` | 1180 | 1180 |
| lowercase / mixed case | `iso-8859-1`, `iSo-8859-1` | 1180 | 1180 |
| leading, trailing or inner space | ` UTF-8`, `UTF-8 `, `UT F-8` | 1180 | 1180 |
| inner `+ ( ! @ * `, tab, byte 0xE9 | `UT+F-8` | 1180 | 1180 |
| all `- _ . : / ( )` removed | `ISO88591` | 899 | 899 (only where that is a listed name) |
| leading or trailing `-` `_` `.` `:` | `-UTF-8`, `UTF-8.` | 0 | - |
| inner `-` `_` `.` `:` `/` `,` added | `UT-F-8` | 77 / 1 / 0 / 0 / 0 / 0 | only where that is a listed name |
| `-` changed to `_` `.` `:` or doubled | `UTF_8` | 1 / 0 / 0 / 0 of 407 | only where that is a listed name |
| `-` changed to space | `UTF 8` | 179 of 407 | the dash-less form is listed |
| trailing `/` or `,` | `UTF-8/`, `UTF-8,,` | all | all |
| one-byte sweep: byte inserted inside, before or after `ISO88591`, all 255 values | | opens for every byte except `, - . / 0-9 : A-Z _ a-z` | same codec |
| `A,/B` versus `A/B,` | `ISO-10646,/UTF8`, `ISO-10646/UTF8,` | no / yes | |
| `A//B` | `ISO-10646//UTF8` | yes, as `ISO-10646` | |
| suffix after a respelled name | `iso 8859 1//TRANSLIT`, `ISO88591/ /TRANSLIT` | yes, transliterating | |

**Collisions.** Under this rule, the only two listed names that reduce to the same form are
`NF_Z_62-010_(1973)` and `NF_Z_62-010_1973`, and they denote the same conversion. Dropping all
punctuation (a looser rule) would group 169 sets of listed names. Each of those sets also denotes
one conversion, but that rule accepts spellings no system name has, such as `cp-1252` or `-utf8`.

Suffix spelling is outside this rule. Examples: `//translit ` with a trailing space transliterates,
while `// TRANSLIT` and `//TRANS LIT` do not.

## libniji before and after (generated charmaps only, every spelling above whose listed name libniji opens)

| | spellings that open or fail as on the reference |
| --- | ---: |
| before (built-ins: case only; charmaps: letters and digits only) | 5328 of 9719 |
| after | see `handoff/ICONV-NAME-MATCH.md` |
