/* Black-box probe: reconstruct a locale's collation order over single code
 * points, with the weight level at which each element differs from the one
 * before it.  No spec clause (probe; feeds the LC_COLLATE derivation).
 *
 * usage: collorder <locale> <lo-hex> <hi-hex> [<lo-hex> <hi-hex>]...
 * output (stdout), one line per code point in collation order:
 *     U+XXXX<TAB><level>
 *   level = 1, 2 or 3: first level at which it differs from the previous line;
 *           4: differs only beyond level 3; =: strcoll returns 0;
 *           I: ignorable (strcoll against "" returns 0; listed first).
 * stderr: a summary (counts, filler checks, strcoll/strxfrm/wcscoll consistency).
 *
 * Level test (each relative to the previous element A and this one B, A < B):
 *   primary   iff A+P2 < B+P1   (fillers P1 < P2 differ at level 1)
 *   secondary iff A+S2 < B+S1   (fillers equal at level 1, differ at level 2)
 *   tertiary  iff A+T2 < B+T1   (fillers equal at levels 1-2, differ at level 3)
 * where X+F is the string X followed by the filler F.  A difference at a lower
 * level of the first element outweighs one at a higher level of the second.
 * Backward secondary ordering would invert the secondary test; the filler
 * check on stderr shows it (sec-fillers-forward=0). */
#define _GNU_SOURCE
#include <locale.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <wchar.h>

struct el {
    unsigned cp;
    char s[5];
    unsigned char *key;
    size_t klen;
};

static int enc(unsigned cp, char *o) {
    if (cp < 0x80) { o[0] = cp; o[1] = 0; return 1; }
    if (cp < 0x800) { o[0] = 0xc0 | cp >> 6; o[1] = 0x80 | (cp & 63); o[2] = 0; return 2; }
    if (cp < 0x10000) {
        o[0] = 0xe0 | cp >> 12; o[1] = 0x80 | (cp >> 6 & 63); o[2] = 0x80 | (cp & 63); o[3] = 0;
        return 3;
    }
    o[0] = 0xf0 | cp >> 18; o[1] = 0x80 | (cp >> 12 & 63); o[2] = 0x80 | (cp >> 6 & 63);
    o[3] = 0x80 | (cp & 63); o[4] = 0;
    return 4;
}

static int cmpkey(const void *x, const void *y) {
    const struct el *a = x, *b = y;
    size_t n = a->klen < b->klen ? a->klen : b->klen;
    int c = memcmp(a->key, b->key, n);
    if (c) return c;
    if (a->klen != b->klen) return a->klen < b->klen ? -1 : 1;
    return a->cp < b->cp ? -1 : a->cp > b->cp;
}

static int coll2(const char *a, const char *fa, const char *b, const char *fb) {
    char x[32], y[32];
    snprintf(x, sizeof x, "%s%s", a, fa);
    snprintf(y, sizeof y, "%s%s", b, fb);
    return strcoll(x, y);
}

static int sgn(int v) { return (v > 0) - (v < 0); }

int main(int argc, char **argv) {
    if (argc < 4 || argc % 2) {
        fprintf(stderr, "usage: collorder <locale> <lo> <hi> [<lo> <hi>]...\n");
        return 2;
    }
    const char *r = setlocale(LC_ALL, argv[1]);
    if (!r) { fprintf(stderr, "setlocale(%s) = NULL\n", argv[1]); return 3; }
    /* fillers: P1<P2 primary; S1,S2 secondary; T1,T2 tertiary (sorted per locale) */
    const char *P1 = "1", *P2 = "2", *S1 = "a", *S2 = "\xc3\xa1" /* á */, *T1 = "a", *T2 = "A";
    if (strcoll(T1, T2) > 0) { const char *t = T1; T1 = T2; T2 = t; }
    /* level-4 fillers: MATHEMATICAL BOLD/ITALIC SMALL A, equal through level 3 */
    const char *Q1 = "\xf0\x9d\x90\x9a", *Q2 = "\xf0\x9d\x91\x8e";
    if (strcoll(Q1, Q2) > 0) { const char *t = Q1; Q1 = Q2; Q2 = t; }
    int q_ok = strcoll(Q1, Q2) < 0 && coll2(Q1, "2", Q2, "1") > 0 && coll2(Q1, S2, Q2, S1) > 0 &&
           coll2(Q1, T2, Q2, T1) > 0;
    int sec_fwd = coll2("a", "\xc3\xa1", "\xc3\xa1", "a") < 0; /* a+á < á+a when forward */
    int fill_ok = strcoll(P1, P2) < 0 && coll2("x", P2, "y", P1) < 0 && strcoll(S1, S2) < 0 &&
                  coll2(S1, "2", S2, "1") > 0 && strcoll(T1, T2) < 0 && coll2(T1, "2", T2, "1") > 0 &&
                  coll2(T1, "a", T2, "\xc3\xa1") < 0;
    size_t cap = 0, n = 0, nign = 0;
    for (int i = 2; i < argc; i += 2) cap += strtoul(argv[i + 1], 0, 16) - strtoul(argv[i], 0, 16) + 1;
    struct el *v = calloc(cap, sizeof *v);
    unsigned *ign = calloc(cap, sizeof *ign);
    for (int i = 2; i < argc; i += 2) {
        unsigned lo = strtoul(argv[i], 0, 16), hi = strtoul(argv[i + 1], 0, 16);
        for (unsigned cp = lo; cp <= hi; cp++) {
            if (cp >= 0xd800 && cp <= 0xdfff) continue;
            struct el *e = &v[n];
            e->cp = cp;
            enc(cp, e->s);
            if (strcoll(e->s, "") == 0) { ign[nign++] = cp; continue; }
            size_t k = strxfrm(NULL, e->s, 0);
            e->key = malloc(k + 1);
            strxfrm((char *)e->key, e->s, k + 1);
            e->klen = k;
            n++;
        }
    }
    qsort(v, n, sizeof *v, cmpkey);
    size_t cnt[6] = {0}, bad_coll = 0, bad_wcs = 0;
    for (size_t i = 0; i < nign; i++) printf("U+%04X\tI\n", ign[i]);
    for (size_t i = 0; i < n; i++) {
        char lv = '1';
        if (i > 0) {
            struct el *a = &v[i - 1], *b = &v[i];
            int c = strcoll(a->s, b->s);
            wchar_t wa[2] = {a->cp, 0}, wb[2] = {b->cp, 0};
            int kc = cmpkey(a, b);
            int keq = a->klen == b->klen && !memcmp(a->key, b->key, a->klen);
            if (c > 0 || (c == 0) != keq) bad_coll++;
            if (sgn(wcscoll(wa, wb)) != sgn(c) && bad_wcs++ < 5)
                fprintf(stderr, "wcscoll-mismatch U+%04X U+%04X strcoll=%d wcscoll=%d\n", a->cp, b->cp,
                        sgn(c), sgn(wcscoll(wa, wb)));
            (void)kc;
            if (c == 0) lv = '=';
            else if (coll2(a->s, P2, b->s, P1) < 0) lv = '1';
            else if (coll2(a->s, S2, b->s, S1) < 0) lv = '2';
            else if (coll2(a->s, T2, b->s, T1) < 0) lv = '3';
            else if (coll2(a->s, Q2, b->s, Q1) < 0) lv = '4';
            else lv = '5';
        }
        cnt[lv == '=' ? 0 : lv - '0']++;
        printf("U+%04X\t%c\n", v[i].cp, lv);
    }
    fprintf(stderr,
            "%s: setlocale=%s n=%zu ignorable=%zu L1=%zu L2=%zu L3=%zu L4=%zu L5+=%zu eq=%zu fillers-ok=%d "
            "l4-fillers-ok=%d sec-fillers-forward=%d case-first=%s strcoll-vs-strxfrm-mismatch=%zu "
            "wcscoll-mismatch=%zu\n",
            argv[1], r, n, nign, cnt[1], cnt[2], cnt[3], cnt[4], cnt[5], cnt[0], fill_ok, q_ok, sec_fwd,
            T1[0] == 'A' ? "upper" : "lower", bad_coll, bad_wcs);
    return 0;
}
