/* LOCALE-PROBE-GEN: fingerprints of a locale's LC_CTYPE and LC_COLLATE behaviour, as seen through
 * the C library's public functions only (a black box), so locales can be grouped by behaviour.
 *
 * Usage: locale_fingerprint <locale name>...   (one output line per name)
 *
 * Each line: the name, then `key=hash` fields (FNV-1a, 64 bits, hex):
 *  - ctype.<class>: iswctype(c, wctype(<class>)) over every code point 0..0x10FFFF, for the twelve
 *    POSIX classes; ctype.toupper / ctype.tolower: towupper / towlower; ctype.width: wcwidth;
 *  - coll.<group>: the order wcscoll gives a corpus of strings from one script group (each code
 *    point of the group's ranges alone, and followed by the group's first letter), with ties
 *    marked, sorted by a stable merge sort so the result does not depend on the sort's own order.
 * A locale setlocale refuses prints `<name> refused`.
 */
#define _GNU_SOURCE
#include <locale.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <wchar.h>
#include <wctype.h>

static uint64_t fnv(uint64_t h, uint64_t v)
{
    for (int i = 0; i < 8; i++) {
        h ^= (v >> (8 * i)) & 0xff;
        h *= 1099511628211ULL;
    }
    return h;
}

#define FNV0 14695981039346656037ULL

struct group { const char *name; unsigned ranges[8][2]; };

static const struct group groups[] = {
    { "ascii", { { 0x21, 0x7e } } },
    { "latin", { { 0xc0, 0x24f }, { 0x1e00, 0x1eff } } },
    { "greek", { { 0x370, 0x3ff }, { 0x1f00, 0x1fff } } },
    { "cyrillic", { { 0x400, 0x52f } } },
    { "armenian_hebrew", { { 0x530, 0x5ff } } },
    { "arabic_syriac_thaana", { { 0x600, 0x7bf } } },
    { "indic", { { 0x900, 0xdff } } },
    { "thai_lao_tibetan", { { 0xe00, 0xfff } } },
    { "myanmar_georgian", { { 0x1000, 0x10ff } } },
    { "ethiopic_cherokee", { { 0x1200, 0x13ff } } },
    { "syllabics_khmer", { { 0x1400, 0x167f }, { 0x1780, 0x17ff } } },
    { "punct_currency", { { 0x2000, 0x206f }, { 0x20a0, 0x20cf } } },
    { "kana_han_hangul", { { 0x3040, 0x30ff }, { 0x4e00, 0x4eff }, { 0xac00, 0xacff } } },
};

static wchar_t (*corpus)[4];

static void msort(int *a, int *tmp, int n)
{
    if (n < 2) return;
    int m = n / 2;
    msort(a, tmp, m);
    msort(a + m, tmp, n - m);
    int i = 0, j = m, k = 0;
    while (i < m && j < n) tmp[k++] = wcscoll(corpus[a[j]], corpus[a[i]]) < 0 ? a[j++] : a[i++];
    while (i < m) tmp[k++] = a[i++];
    while (j < n) tmp[k++] = a[j++];
    memcpy(a, tmp, n * sizeof *a);
}

static uint64_t coll(const struct group *g)
{
    int n = 0;
    for (int r = 0; r < 8 && g->ranges[r][1]; r++) n += 2 * (g->ranges[r][1] - g->ranges[r][0] + 1);
    corpus = calloc(n, sizeof *corpus);
    int *idx = calloc(n, sizeof *idx), *tmp = calloc(n, sizeof *tmp);
    if (!corpus || !idx || !tmp) exit(2);
    wchar_t first = g->ranges[0][0] == 0x21 ? L'a' : (wchar_t)g->ranges[0][0];
    int k = 0;
    for (int r = 0; r < 8 && g->ranges[r][1]; r++)
        for (unsigned c = g->ranges[r][0]; c <= g->ranges[r][1]; c++) {
            corpus[k][0] = (wchar_t)c; k++;
            corpus[k][0] = (wchar_t)c; corpus[k][1] = first; k++;
        }
    for (int i = 0; i < n; i++) idx[i] = i;
    msort(idx, tmp, n);
    uint64_t h = FNV0;
    for (int i = 0; i < n; i++) {
        h = fnv(h, (uint64_t)corpus[idx[i]][0] | (uint64_t)corpus[idx[i]][1] << 32);
        if (i + 1 < n) h = fnv(h, wcscoll(corpus[idx[i]], corpus[idx[i + 1]]) == 0);
    }
    free(corpus); free(idx); free(tmp);
    return h;
}

int main(int argc, char **argv)
{
    static const char *classes[] = { "alpha", "upper", "lower", "digit", "xdigit", "space", "print", "graph", "blank", "cntrl", "punct", "alnum" };
    for (int a = 1; a < argc; a++) {
        if (!setlocale(LC_ALL, argv[a])) { printf("%s refused\n", argv[a]); continue; }
        printf("%s", argv[a]);
        for (int k = 0; k < 12; k++) {
            wctype_t t = wctype(classes[k]);
            uint64_t h = FNV0;
            for (unsigned c = 0; c < 0x110000; c++) h = fnv(h, iswctype((wint_t)c, t) != 0);
            printf(" ctype.%s=%016llx", classes[k], (unsigned long long)h);
        }
        uint64_t u = FNV0, l = FNV0, w = FNV0;
        for (unsigned c = 0; c < 0x110000; c++) {
            u = fnv(u, towupper((wint_t)c));
            l = fnv(l, towlower((wint_t)c));
            w = fnv(w, (uint64_t)(int64_t)wcwidth((wchar_t)c));
        }
        printf(" ctype.toupper=%016llx ctype.tolower=%016llx ctype.width=%016llx", (unsigned long long)u, (unsigned long long)l, (unsigned long long)w);
        for (size_t g = 0; g < sizeof groups / sizeof *groups; g++) printf(" coll.%s=%016llx", groups[g].name, (unsigned long long)coll(&groups[g]));
        printf("\n");
        fflush(stdout);
    }
    return 0;
}
