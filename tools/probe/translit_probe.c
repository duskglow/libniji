/* LOCALE-TRANSLIT-TABLE: the transliterations a locale gives, seen through the C library's public
 * iconv(3) only (a black box): `//TRANSLIT` consults the current locale's LC_CTYPE table.
 *
 * Usage:
 *   translit_probe first <locale name> <codeset>
 *       Under setlocale(LC_ALL, <locale name>), for every code point 1..0x10FFFF (surrogates left
 *       out) that <codeset> does not encode, the result of converting it alone to
 *       `<codeset>//TRANSLIT`, read back as code points: one line `<cp>\t<cp> <cp> ...` (hex),
 *       none when the conversion fails or gives nothing or what U+E000 (private use, no entry)
 *       gives, the locale's `default_missing`; the first line is `default\t<its bytes, hex>`.
 *       A name setlocale refuses prints
 *       `refused <name>` and exits 1.
 *   translit_probe enc <codeset>...
 *       For each stdin line of hex code points (a string), one line of `0`/`1` per codeset:
 *       whether the codeset encodes the whole string (no transliteration).
 *
 * tools/probe/translit_table.py runs it over the reference's legacy-codeset locales.
 */
#define _GNU_SOURCE
#include <errno.h>
#include <iconv.h>
#include <locale.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* Convert `n` code points with `cd` into `out` (at most `cap` bytes). The byte count, or -1
 * when the conversion fails anywhere. */
static long conv(iconv_t cd, const uint32_t *s, size_t n, char *out, size_t cap)
{
    char *in = (char *)s, *o = out;
    size_t il = n * 4, ol = cap;
    iconv(cd, NULL, NULL, NULL, NULL);
    if (iconv(cd, &in, &il, &o, &ol) == (size_t)-1 || il != 0)
        return -1;
    if (iconv(cd, NULL, NULL, &o, &ol) == (size_t)-1)
        return -1;
    return (long)(o - out);
}

static iconv_t open_or_die(const char *to, const char *from)
{
    iconv_t cd = iconv_open(to, from);
    if (cd == (iconv_t)-1) {
        fprintf(stderr, "iconv_open(%s, %s): %s\n", to, from, strerror(errno));
        exit(2);
    }
    return cd;
}

static int first(const char *name, const char *cs)
{
    if (!setlocale(LC_ALL, name)) {
        printf("refused %s\n", name);
        return 1;
    }
    char tr[128];
    snprintf(tr, sizeof tr, "%s//TRANSLIT", cs);
    iconv_t plain = open_or_die(cs, "UTF-32LE"), tl = open_or_die(tr, "UTF-32LE"), back = open_or_die("UTF-32LE", cs);
    char buf[256], dflt[256];
    uint32_t wide[64];
    /* A private-use code point has no transliteration: what it gives is `default_missing`,
     * which iconv applies to every character without one. Results equal to it are left out
     * (a table entry that gives the same text cannot be told apart here). */
    uint32_t pua = 0xe000;
    long nd = conv(tl, &pua, 1, dflt, sizeof dflt);
    printf("default\t");
    for (long i = 0; i < nd; i++)
        printf("%02X", (unsigned char)dflt[i]);
    printf("\n");
    for (uint32_t c = 1; c < 0x110000; c++) {
        if (c >= 0xd800 && c < 0xe000)
            continue;
        if (conv(plain, &c, 1, buf, sizeof buf) >= 0)
            continue;
        long n = conv(tl, &c, 1, buf, sizeof buf);
        if (n <= 0 || (n == nd && !memcmp(buf, dflt, (size_t)n)))
            continue;
        char *in = buf, *o = (char *)wide;
        size_t il = (size_t)n, ol = sizeof wide;
        iconv(back, NULL, NULL, NULL, NULL);
        if (iconv(back, &in, &il, &o, &ol) == (size_t)-1 || il != 0) {
            printf("%04X\tunreadable\n", c);
            continue;
        }
        printf("%04X\t", c);
        for (size_t i = 0; i < (size_t)(o - (char *)wide) / 4; i++)
            printf(i ? " %04X" : "%04X", wide[i]);
        printf("\n");
    }
    return 0;
}

static int enc(int n, char **cs)
{
    iconv_t *cd = calloc((size_t)n, sizeof *cd);
    for (int i = 0; i < n; i++)
        cd[i] = open_or_die(cs[i], "UTF-32LE");
    char line[4096], buf[1024];
    while (fgets(line, sizeof line, stdin)) {
        uint32_t s[256];
        size_t k = 0;
        for (char *p = strtok(line, " \n"); p && k < 256; p = strtok(NULL, " \n"))
            s[k++] = (uint32_t)strtoul(p, NULL, 16);
        for (int i = 0; i < n; i++)
            putchar(conv(cd[i], s, k, buf, sizeof buf) >= 0 ? '1' : '0');
        putchar('\n');
    }
    return 0;
}

int main(int argc, char **argv)
{
    if (argc == 4 && !strcmp(argv[1], "first"))
        return first(argv[2], argv[3]);
    if (argc >= 3 && !strcmp(argv[1], "enc"))
        return enc(argc - 2, argv + 2);
    fprintf(stderr, "usage: translit_probe first <locale> <codeset> | enc <codeset>...\n");
    return 2;
}
