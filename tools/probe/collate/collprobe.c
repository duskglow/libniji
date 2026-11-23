/* Black-box probe: per locale, sort a word list with strcoll and report
 * whether the order matches byte order, the tailored order expected from
 * CLDR's documented tailoring, or neither.  No spec clause (probe). */
#include <locale.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int by_coll(const void *a, const void *b) {
    return strcoll(*(const char *const *)a, *(const char *const *)b);
}
static int by_byte(const void *a, const void *b) {
    return strcmp(*(const char *const *)a, *(const char *const *)b);
}

struct cas {
    const char *loc;
    const char *note;
    /* words listed in the TAILORED order CLDR documents for the locale */
    const char *tail[12];
    /* the same words in root (untailored UCA) order */
    const char *root[12];
};

static const struct cas cases[] = {
    {"sv_SE.UTF-8", "a..z then å ä ö",
     {"az", "zz", "åa", "äa", "öa", 0},
     {"az", "åa", "äa", "öa", "zz", 0}},
    {"es_ES.UTF-8", "ñ a letter after n",
     {"nz", "ña", "oa", 0},
     {"ña", "nz", "oa", 0}},
    {"tr_TR.UTF-8", "ı before i; I/ı and İ/i pairs",
     {"ıa", "Ia", "ia", "İa", "iz", "ja", 0},
     {"ia", "Ia", "İa", "iz", "ıa", "ja", 0}},
    {"pl_PL.UTF-8", "ą ł ś ż letters of their own",
     {"az", "ąa", "lz", "ła", "sz", "śa", "zz", "źa", "ża", 0},
     {"az", "ąa", "lz", "ła", "sz", "śa", "zz", "źa", "ża", 0}},
    {"pl_PL.UTF-8", "ą after az (tailored) vs between (root)",
     {"az", "ąa", 0},
     {"ąa", "az", 0}},
    {"zh_CN.UTF-8", "Han in pinyin order",
     {"阿", "八", "你", "我", "中", 0},
     {"中", "你", "八", "我", "阿", 0}},
    {"ja_JP.UTF-8", "kana before Han; hiragana/katakana pairs",
     {"あ", "ア", "か", "カ", "亜", "日", 0},
     {"あ", "ア", "か", "カ", "日", "亜", 0}},
    {"ko_KR.UTF-8", "Hanja next to its Hangul reading",
     {"가", "伽", "나", "羅", 0},
     {"가", "나", "伽", "羅", 0}},
};

static int same(const char **got, const char *const *want, int n) {
    for (int i = 0; i < n; i++)
        if (strcmp(got[i], want[i]) != 0) return 0;
    return 1;
}

int main(void) {
    for (size_t c = 0; c < sizeof cases / sizeof cases[0]; c++) {
        const struct cas *k = &cases[c];
        const char *r = setlocale(LC_ALL, k->loc);
        const char *w[12];
        int n = 0;
        while (k->tail[n]) { w[n] = k->tail[n]; n++; }
        const char *b[12];
        memcpy(b, w, sizeof w);
        qsort(b, n, sizeof *b, by_byte);
        if (r) qsort(w, n, sizeof *w, by_coll);
        printf("%-12s setlocale=%s (%s)\n  strcoll:", k->loc, r ? r : "NULL", k->note);
        for (int i = 0; i < n; i++) printf(" %s", w[i]);
        int t = r && same(w, k->tail, n), o = r && same(w, k->root, n), y = r && same(w, b, n);
        printf("\n  verdict: %s%s%s%s\n", t ? "TAILORED " : "", o ? "ROOT " : "", y ? "BYTE " : "",
               (!t && !o && !y) ? "OTHER" : "");
    }
    return 0;
}
