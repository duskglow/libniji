/* INTERNAL-ALLOC-ROUTING (ruling #7583; no spec clause beyond the GNU C Library manual's "Replacing
 * malloc", which names malloc, free, calloc and realloc as the functions the library itself uses): when a
 * program replaces the malloc family, the library's own allocations go through the replacement. A
 * program that replaces it (a counting, tracking or fault-injecting allocator) therefore sees
 * opendir and setlocale allocate through its family, and every block the library hands to
 * its free is one of its own (a free of anything else is a failure here: private memory released
 * through the program's free).
 * When the replacement fails, each call reports its documented failure and never crashes: the Nth
 * allocation of a call failing, for every N up to the call's own count, leaves either a success or the
 * failure (opendir NULL with ENOMEM, setlocale NULL with the locale unchanged, iconv_open (iconv_t)-1
 * with ENOMEM), and the same call succeeds again once the replacement does. qsort still sorts when every
 * allocation fails. The program's family is a bump allocator over a static array; it never calls the
 * library's.
 * Each failed check prints FAIL and the line; the exit status is the number of failures. */
/* LINK-HELPERS: sbind_lib */
#define _GNU_SOURCE
#include <dirent.h>
#include <errno.h>
#include <iconv.h>
#include <locale.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int failures;
#define CHECK(c) ((c) ? (void)0 : (void)(dprintf(2, "FAIL line %d: %s\n", __LINE__, #c), failures++))

static _Alignas(4096) char heap[64 << 20];
static size_t top;
/* volatile: the library's declarations may tell the compiler it never calls back into this file. */
static volatile unsigned long calls, foreign;
/* 0: every allocation succeeds; n: the nth allocation counted from `calls` = 0 fails, and every later one
 * too when `sticky`. */
static volatile unsigned long fail_at;
static volatile int sticky;

static void *take(size_t n, size_t al)
{
    calls++;
    if (fail_at && (calls == fail_at || (sticky && calls > fail_at))) {
        errno = ENOMEM;
        return NULL;
    }
    top = (top + al - 1) & ~(al - 1);
    size_t m = (n + 15) & ~(size_t)15;
    if (m + 16 > sizeof heap - top) {
        errno = ENOMEM;
        return NULL;
    }
    *(size_t *)(heap + top) = n;
    top += 16;
    void *p = heap + top;
    top += m;
    return p;
}
static int ours(const void *p) { return (const char *)p >= heap && (const char *)p < heap + sizeof heap; }
void *malloc(size_t n) { return take(n, 16); }
void free(void *p)
{
    if (p && !ours(p))
        foreign++;
}
void *calloc(size_t a, size_t b)
{
    if (b && a > (size_t)-1 / b) {
        errno = ENOMEM;
        return NULL;
    }
    void *p = take(a * b, 16);
    if (p)
        memset(p, 0, a * b);
    return p;
}
void *realloc(void *o, size_t n)
{
    if (o && !ours(o))
        foreign++;
    void *p = take(n, 16);
    if (p && o && ours(o)) {
        size_t have = *(size_t *)((char *)o - 16);
        memcpy(p, o, n < have ? n : have);
    }
    return p;
}
void *aligned_alloc(size_t al, size_t n) { return take(n, al < 16 ? 16 : al); }
void *memalign(size_t al, size_t n) { return take(n, al < 16 ? 16 : al); }
int posix_memalign(void **r, size_t al, size_t n)
{
    void *p = take(n, al < 16 ? 16 : al);
    if (!p)
        return ENOMEM;
    *r = p;
    return 0;
}

enum { OPENDIR, SETLOCALE, ICONV, KINDS };
static const char *const names[KINDS] = {"opendir", "setlocale", "iconv_open"};
/* Whether the call must allocate (and so fail when every allocation does): libniji's iconv_open makes
 * no allocation (docs/differences-from-glibc.md, D-X.156), so there it only has to succeed or fail cleanly. */
static const int allocates[KINDS] = {1, 1, 0};

/* One call of kind k: 1 when it succeeded (and did its work), 0 when it reported its documented failure. */
static int run(int k)
{
    switch (k) {
    case OPENDIR: {
        DIR *d = opendir("/");
        if (!d) {
            CHECK(errno == ENOMEM);
            return 0;
        }
        int seen = 0;
        struct dirent *e;
        while ((e = readdir(d)))
            seen += strcmp(e->d_name, ".") == 0;
        CHECK(seen == 1);
        CHECK(closedir(d) == 0);
        return 1;
    }
    case SETLOCALE: {
        char *r = setlocale(LC_ALL, "C.UTF-8");
        if (!r) {
            /* The failure leaves the locale as it was. */
            CHECK(strcmp(setlocale(LC_ALL, NULL), "C") == 0);
            return 0;
        }
        CHECK(MB_CUR_MAX == 6 || MB_CUR_MAX == 4);
        unsigned long at = fail_at;
        fail_at = 0;
        CHECK(setlocale(LC_ALL, "C") != NULL);
        fail_at = at;
        return 1;
    }
    default: {
        iconv_t c = iconv_open("UTF-8", "ISO-8859-1");
        if (c == (iconv_t)-1) {
            CHECK(errno == ENOMEM);
            return 0;
        }
        char in[] = "\xe9t\xe9", out[16] = {0};
        char *ip = in, *op = out;
        size_t il = 3, ol = sizeof out;
        CHECK(iconv(c, &ip, &il, &op, &ol) == 0);
        CHECK(strcmp(out, "\xc3\xa9t\xc3\xa9") == 0);
        CHECK(iconv_close(c) == 0);
        return 1;
    }
    }
}

static int cmpint(const void *a, const void *b) { return *(const int *)a - *(const int *)b; }
static int v[200000];

int main(void)
{
    for (int k = 0; k < KINDS; k++) {
        /* Every allocation failing, before the call has ever succeeded: its documented failure. */
        calls = 0;
        fail_at = 1;
        sticky = 1;
        int r = run(k);
        fail_at = 0;
        sticky = 0;
        if (r != 0 && allocates[k])
            dprintf(2, "FAIL %s: succeeded with every allocation failing (%lu calls)\n", names[k], calls);
        failures += r != 0 && allocates[k];
        /* The call as usual: it reaches the replacement. */
        calls = 0;
        CHECK(run(k) == 1);
        unsigned long n = calls;
        if (n == 0 && allocates[k])
            dprintf(2, "FAIL %s: the replacement saw no allocation\n", names[k]);
        failures += n == 0 && allocates[k];
        /* Its nth allocation failing, each n: a success or the documented failure, then a success. */
        for (unsigned long i = 1; i <= n && i <= 64; i++) {
            calls = 0;
            fail_at = i;
            (void)run(k);
            fail_at = 0;
            calls = 0;
            if (run(k) != 1) {
                dprintf(2, "FAIL %s: no success after its allocation %lu failed\n", names[k], i);
                failures++;
            }
        }
    }
    /* printf's common conversions make no call to the replacement (INTERNAL-ALLOC-ROUTING-2; the text
     * is C11 7.21.6.1's): with every allocation failing, snprintf still succeeds with the full text. The
     * format is read through a volatile pointer so the compiler cannot fold the call's result. */
    static const char *volatile pf_fmt = "%d|%5s%-3c|%#lx %o|%*d|%.2f|%ls";
    static const char pf_want[] = "-42|   abz  |0x1000 10|   7|2.50|w";
    char pf_out[96];
    calls = 0;
    fail_at = 1;
    sticky = 1;
    int pf_n = snprintf(pf_out, sizeof pf_out, pf_fmt, -42, "ab", 'z', 4096ul, 8u, 4, 7, 2.5, L"w");
    unsigned long pf_calls = calls;
    fail_at = 0;
    sticky = 0;
    if (pf_calls != 0)
        dprintf(2, "FAIL snprintf: %lu calls to the replacement for common conversions\n", pf_calls);
    failures += pf_calls != 0;
    CHECK(pf_n == (int)strlen(pf_want));
    CHECK(strcmp(pf_out, pf_want) == 0);
    /* qsort sorts with every allocation failing. */
    for (int i = 0; i < 200000; i++)
        v[i] = (i * 7919) % 200000;
    fail_at = 1;
    sticky = 1;
    qsort(v, 200000, sizeof v[0], cmpint);
    fail_at = 0;
    sticky = 0;
    for (int i = 0; i < 200000; i++)
        if (v[i] != i) {
            CHECK(v[i] == i);
            break;
        }
    if (foreign)
        dprintf(2, "FAIL %lu frees of memory the replacement did not make\n", foreign);
    failures += foreign != 0;
    return failures;
}
