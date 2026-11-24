/* MATCHER-WORDEXP-RETHINK: fnmatch on non-ASCII (UTF-8) file names, one call per pattern/name pair, and the
 * plain wordexp rows, per call. fnmatch_names_utf8 is the cbench_opt row of the same name (same names, same
 * patterns, same order); the fnmatch_utf8_* cases are single pairs for wl_trace. Run with --locale C.UTF-8,
 * the locale of cbench_opt's rows. */
#include "percall.h"
#include <fnmatch.h>
#include <wordexp.h>

static const char *const u8_names[8] = {
    "r\xc3\xa9sum\xc3\xa9.pdf", "Gr\xc3\xbc\xc3\x9f" "e.txt", "na\xc3\xafve.c", "\xc3\x9c" "bersicht.ods",
    "\xe6\x97\xa5\xe6\x9c\xac\xe8\xaa\x9e.txt", "\xce\x95\xce\xbb\xce\xbb\xce\xb7\xce\xbd\xce\xb9\xce\xba\xce\xac.c",
    "caf\xc3\xa9.h", "\xd0\x9e\xd1\x82\xd1\x87\xd1\x91\xd1\x82.txt",
};
static const struct { const char *pat; int flags; } u8_pats[7] = {
    {"*.c", 0}, {"*.[ch]", 0}, {"lib*.so*", 0}, {"*", FNM_PERIOD}, {"????.txt", 0}, {"[[:upper:]]*", 0},
    {"*.txt", FNM_CASEFOLD},
};

CASE(fnmatch_names_utf8, 100000, 1) {
    for (long i = 0; i < n; i++) pc_sink += fnmatch(u8_pats[i % 7].pat, u8_names[i % 8], u8_pats[i % 7].flags);
    PC_CHECKING {
        char r[57];
        for (int k = 0; k < 56; k++) r[k] = fnmatch(u8_pats[k % 7].pat, u8_names[k % 8], u8_pats[k % 7].flags) == 0 ? '1' : '0';
        r[56] = 0;
        pc_check("%s", r);
    }
}

#define PAIR(name, pat, str, flags)                                                                              \
    CASE(name, 100000, 1) {                                                                                       \
        const char *volatile p = pat, *volatile s = str;                                                          \
        for (long i = 0; i < n; i++) pc_sink += fnmatch(p, s, flags);                                             \
        PC_CHECKING { pc_check("%d", fnmatch(p, s, flags) == 0); }                                                \
    }
PAIR(fnmatch_utf8_ch, "*.[ch]", "na\xc3\xafve.c", 0)
PAIR(fnmatch_utf8_star_c, "*.c", "na\xc3\xafve.c", 0)
PAIR(fnmatch_utf8_period, "*", "Gr\xc3\xbc\xc3\x9f" "e.txt", FNM_PERIOD)
PAIR(fnmatch_utf8_question, "????.txt", "Gr\xc3\xbc\xc3\x9f" "e.txt", 0)
PAIR(fnmatch_utf8_class, "[[:upper:]]*", "\xc3\x9c" "bersicht.ods", 0)
PAIR(fnmatch_utf8_lib_so, "lib*.so*", "r\xc3\xa9sum\xc3\xa9.pdf", 0)
PAIR(fnmatch_utf8_casefold, "*.txt", "Gr\xc3\xbc\xc3\x9f" "e.txt", FNM_CASEFOLD)

static long we_words(const char *text)
{
    wordexp_t we;
    if (wordexp(text, &we, WRDE_NOCMD) != 0)
        return -1;
    long w = (long)we.we_wordc;
    wordfree(&we);
    return w;
}

#define WORDS(name, text)                                                                                         \
    CASE(name, 12500, 1) {                                                                                        \
        setenv("HOME", "/home/cbench", 1);                                                                        \
        setenv("WORDS", "one two three four", 1);                                                                 \
        const char *volatile t = text;                                                                            \
        for (long i = 0; i < n; i++) pc_sink += we_words(t);                                                      \
        PC_CHECKING { pc_check("%ld", we_words(t)); }                                                            \
    }
WORDS(wordexp_plain_8, "cc -O2 -Wall -o prog main.c util.c -lm")
WORDS(wordexp_quoted, "'a b' \"c $HOME\" d\\ e")
WORDS(wordexp_split_var, "$WORDS")
WORDS(wordexp_tilde_var, "~/a $HOME b")
