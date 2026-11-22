/* LIBC-SELF-BIND (D-27.76; no spec clause beyond the GNU C Library manual's "Replacing malloc"): a stream
 * the library opens for the program is allocated and released through the program's malloc family. A
 * program that replaces malloc (a counting or tracking allocator) sees fopen allocate through its malloc
 * and fclose release through its free, as for strdup (x_self_bind). The program's family is a bump
 * allocator over a static array with call counters; it never calls the library's.
 * Split from x_self_bind so that program keeps its strict checks: on the reference fopen and fclose reach
 * the program's family; libniji's stream objects do not come from it (the same before and after
 * D-27.76's link change), which needs a ruling.
 * Each failed check prints FAIL and the line; the exit status is the number of failures. */
/* LINK-HELPERS: sbind_lib */
/* PENDING-NIJI: a ruling on where stdio allocates its stream objects (LIBC-SELF-BIND hand-back) */
#define _POSIX_C_SOURCE 200809L
#include <stddef.h>
#include <stdio.h>
#include <string.h>

static int failures;
#define CHECK(c) ((c) ? (void)0 : (void)(dprintf(2, "FAIL line %d: %s\n", __LINE__, #c), failures++))

static _Alignas(4096) char heap[1 << 22];
static size_t top;
static volatile unsigned long mallocs, frees;
static void *take(size_t n, size_t al)
{
    top = (top + al - 1) & ~(al - 1);
    n = (n + 15) & ~(size_t)15;
    if (n > sizeof heap - top)
        return NULL;
    void *p = heap + top;
    top += n;
    mallocs++;
    return p;
}
void *malloc(size_t n) { return take(n, 16); }
void free(void *p) { if (p) frees++; }
void *calloc(size_t a, size_t b)
{
    if (b && a > (size_t)-1 / b)
        return NULL;
    void *p = take(a * b, 16);
    if (p)
        memset(p, 0, a * b);
    return p;
}
void *realloc(void *o, size_t n)
{
    void *p = take(n, 16);
    if (p && o) {
        size_t have = (size_t)(heap + top - (char *)o);
        memcpy(p, o, n < have ? n : have);
    }
    return p;
}
void *aligned_alloc(size_t al, size_t n) { return take(n, al < 16 ? 16 : al); }
void *memalign(size_t al, size_t n) { return take(n, al < 16 ? 16 : al); }
int posix_memalign(void **r, size_t al, size_t n)
{
    *r = take(n, al < 16 ? 16 : al);
    return *r ? 0 : 12;
}

int main(void)
{
    unsigned long m0 = mallocs;
    FILE *f = fopen("/dev/null", "r");
    unsigned long m1 = mallocs, f0 = frees;
    CHECK(f != NULL);
    dprintf(2, "x_self_bind_stdio: fopen called the program's malloc family %lu times\n", m1 - m0);
    CHECK(m1 > m0);
    CHECK(f && fclose(f) == 0);
    dprintf(2, "x_self_bind_stdio: fclose called the program's free %lu times\n", frees - f0);
    CHECK(frees > f0);
    return failures;
}
