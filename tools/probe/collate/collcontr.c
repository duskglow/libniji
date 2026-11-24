/* Black-box probe: find two-character contractions in a locale's collation.
 * No spec clause (probe; feeds the LC_COLLATE derivation).
 *
 * usage: collcontr <locale> <cp-hex>...
 * For each first character x of the candidate set C, without contractions the
 * strings x+y (y in C) sort exactly as the characters y do, since every level
 * compares x first and x is equal.  The strings out of that order (the
 * complement of a longest increasing subsequence of the y order) are printed as
 *     CONTRACTION U+xxxx U+yyyy
 * with, for each, its neighbours in the x+y order.  A contraction x+w can also
 * displace x+y; the neighbours let the reader tell which one moved. */
#define _GNU_SOURCE
#include <locale.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int enc(unsigned cp, char *o) {
    int n;
    if (cp < 0x80) { o[0] = cp; n = 1; }
    else if (cp < 0x800) { o[0] = 0xc0 | cp >> 6; o[1] = 0x80 | (cp & 63); n = 2; }
    else if (cp < 0x10000) {
        o[0] = 0xe0 | cp >> 12; o[1] = 0x80 | (cp >> 6 & 63); o[2] = 0x80 | (cp & 63); n = 3;
    } else {
        o[0] = 0xf0 | cp >> 18; o[1] = 0x80 | (cp >> 12 & 63); o[2] = 0x80 | (cp >> 6 & 63);
        o[3] = 0x80 | (cp & 63); n = 4;
    }
    o[n] = 0;
    return n;
}

static unsigned *C; static size_t nc; static unsigned X;

static int by_single(const void *a, const void *b) {
    char x[8], y[8];
    enc(C[*(const size_t *)a], x); enc(C[*(const size_t *)b], y);
    int c = strcoll(x, y);
    return c ? c : (*(const size_t *)a > *(const size_t *)b) - (*(const size_t *)a < *(const size_t *)b);
}
static int by_pair(const void *a, const void *b) {
    char x[16], y[16];
    int n = enc(X, x); enc(C[*(const size_t *)a], x + n);
    n = enc(X, y); enc(C[*(const size_t *)b], y + n);
    int c = strcoll(x, y);
    return c ? c : (*(const size_t *)a > *(const size_t *)b) - (*(const size_t *)a < *(const size_t *)b);
}

int main(int argc, char **argv) {
    if (argc < 3) { fprintf(stderr, "usage: collcontr <locale> <cp-hex>...\n"); return 2; }
    if (!setlocale(LC_ALL, argv[1])) { fprintf(stderr, "setlocale(%s) = NULL\n", argv[1]); return 3; }
    nc = argc - 2;
    C = calloc(nc, sizeof *C);
    for (size_t i = 0; i < nc; i++) C[i] = strtoul(argv[i + 2], 0, 16);
    size_t *single = calloc(nc, sizeof *single), *pair = calloc(nc, sizeof *pair);
    size_t *rank = calloc(nc, sizeof *rank), *tails = calloc(nc, sizeof *tails);
    size_t *tidx = calloc(nc, sizeof *tidx), *prev = calloc(nc, sizeof *prev);
    char *keep = calloc(nc, 1);
    for (size_t i = 0; i < nc; i++) single[i] = i;
    qsort(single, nc, sizeof *single, by_single);
    for (size_t i = 0; i < nc; i++) rank[single[i]] = i;
    size_t found = 0;
    for (size_t xi = 0; xi < nc; xi++) {
        X = C[xi];
        for (size_t i = 0; i < nc; i++) pair[i] = i;
        qsort(pair, nc, sizeof *pair, by_pair);
        /* longest increasing subsequence of rank[pair[i]] */
        size_t len = 0;
        for (size_t i = 0; i < nc; i++) {
            size_t v = rank[pair[i]], lo = 0, hi = len;
            while (lo < hi) { size_t m = (lo + hi) / 2; if (tails[m] < v) lo = m + 1; else hi = m; }
            tails[lo] = v; tidx[lo] = i; prev[i] = lo ? tidx[lo - 1] : (size_t)-1;
            if (lo == len) len++;
        }
        memset(keep, 0, nc);
        for (size_t i = len ? tidx[len - 1] : (size_t)-1; i != (size_t)-1; i = prev[i]) keep[i] = 1;
        for (size_t i = 0; i < nc; i++) {
            if (keep[i]) continue;
            found++;
            printf("CONTRACTION U+%04X U+%04X after U+%04X+U+%04X before U+%04X+U+%04X\n", X, C[pair[i]],
                   X, i ? C[pair[i - 1]] : 0, X, i + 1 < nc ? C[pair[i + 1]] : 0);
        }
    }
    fprintf(stderr, "%s: candidates=%zu out-of-order-pairs=%zu\n", argv[1], nc, found);
    return 0;
}
