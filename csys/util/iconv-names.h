/* The built-in charset names of libniji's iconv(3) (crates/core/src/iconv/mod.rs, `builtin_names`), listed by
 * `iconv -l` beside the installed charmaps (UTIL-7). The core test `iconv_listing_names_match_the_builtins`
 * (crates/core/tests/iconv_cmd_names.rs) fails when this list and the library's table differ. */
static const char *const BUILTIN_NAMES[] = {
    "EUC-JP", "EUCJP", "UJIS", "CSEUCPKDFMTJAPANESE", "OSF00030010",
    "UTF-8", "UTF8", "ISO-10646/UTF8", "ISO-10646/UTF-8", "ISO-IR-193",
    "UTF-16", "UTF16", "UTF-16LE", "UTF16LE", "UTF-16BE", "UTF16BE",
    "UTF-32", "UTF32", "UTF-32LE", "UTF32LE", "UTF-32BE", "UTF32BE",
    "UCS-2", "UCS2", "ISO-10646/UCS2", "CSUNICODE", "UCS-2LE", "UNICODELITTLE", "UCS-2BE", "UNICODEBIG",
    "UCS-4", "UCS4", "ISO-10646", "ISO-10646/UCS4", "10646-1:1993", "10646-1:1993/UCS4", "CSUCS4",
    "UCS-4BE", "UCS-4LE", "WCHAR_T",
    "ANSI_X3.4-1968", "ANSI_X3.4-1986", "ISO-IR-6", "ISO_646.IRV:1991", "ASCII", "ISO646-US", "US-ASCII",
    "US", "IBM367", "CP367", "CSASCII",
    "ISO-8859-1", "ISO_8859-1", "ISO8859-1", "LATIN1", "L1", "CP819",
    "ISO-IR-100", "ISO_8859-1:1987", "IBM819", "CSISOLATIN1", "8859_1", "OSF00010001",
};
