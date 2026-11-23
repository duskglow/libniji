/* LIBC-SELF-BIND (D-27.76), STDIO-CALLER-ALLOC (D-12.31; no spec clause beyond the GNU C Library
 * manual's "Replacing malloc"): a stream the library opens for the program is allocated and released
 * through the program's malloc family (ruling #7583). A program that replaces malloc (a counting or
 * tracking allocator) sees each open (fopen, fdopen, fmemopen, open_memstream, fopencookie) allocate
 * through its malloc family and fclose release what the open allocated through its free, as for
 * strdup (x_self_bind). The program's family is a bump allocator over a static array that records
 * each block; it never calls the library's.
 * When the replacement fails (NULL, errno ENOMEM), each open returns NULL with errno ENOMEM and
 * leaves no descriptor behind; it never crashes.
 * Split from x_self_bind so that program keeps its strict checks.
 * Each failed check prints FAIL and the line; the exit status is the number of failures. */
/* LINK-HELPERS: sbind_lib */
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <stddef.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

static int failures;
#define CHECK(c) ((c) ? (void)0 : (void)(dprintf(2, "FAIL line %d: %s\n", __LINE__, #c), failures++))

static _Alignas(4096) char heap[1 << 22];
static size_t top;
/* volatile: the library's declarations may tell the compiler it never calls back into this file. */
static volatile int fail;
static volatile unsigned long mallocs, frees;
/* The blocks made while `watch` is set, and whether each has been freed since. */
static volatile int watch;
static void *made[16];
static int made_n, made_freed[16];

static void *take(size_t n, size_t al)
{
    if (fail) {
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
    mallocs++;
    if (watch && made_n < 16) {
        made_freed[made_n] = 0;
        made[made_n++] = p;
    }
    return p;
}
void *malloc(size_t n) { return take(n, 16); }
void free(void *p)
{
    if (!p)
        return;
    frees++;
    for (int i = 0; i < made_n; i++)
        if (made[i] == p)
            made_freed[i] = 1;
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
    void *p = take(n, 16);
    if (p && o) {
        size_t have = *(size_t *)((char *)o - 16);
        memcpy(p, o, n < have ? n : have);
        free(o);
    }
    return p;
}
void *aligned_alloc(size_t al, size_t n) { return take(n, al < 16 ? 16 : al); }
void *memalign(size_t al, size_t n) { return take(n, al < 16 ? 16 : al); }
int posix_memalign(void **r, size_t al, size_t n)
{
    *r = take(n, al < 16 ? 16 : al);
    return *r ? 0 : ENOMEM;
}

static ssize_t ck_read(void *c, char *b, size_t n) { (void)c; (void)b; (void)n; return 0; }
static ssize_t ck_write(void *c, const char *b, size_t n) { (void)c; (void)b; return (ssize_t)n; }

static char mem[64];
static char *ms;
static size_t msz;

enum { FOPEN, FDOPEN, FMEMOPEN, MEMSTREAM, COOKIE, KINDS };
static const char *const names[KINDS] = {"fopen", "fdopen", "fmemopen", "open_memstream", "fopencookie"};

static FILE *open_kind(int k)
{
    switch (k) {
    case FOPEN:
        return fopen("/dev/null", "r");
    case FDOPEN: {
        int fd = open("/dev/null", O_RDWR);
        FILE *f = fd < 0 ? NULL : fdopen(fd, "r+");
        if (!f && fd >= 0)
            close(fd);
        return f;
    }
    case FMEMOPEN:
        return fmemopen(mem, sizeof mem, "r+");
    case MEMSTREAM:
        return open_memstream(&ms, &msz);
    default:
        return fopencookie(NULL, "r+", (cookie_io_functions_t){ck_read, ck_write, NULL, NULL});
    }
}

/* The lowest free descriptor (what the next open returns). */
static int lowest_fd(void)
{
    int fd = open("/dev/null", O_RDONLY);
    if (fd >= 0)
        close(fd);
    return fd;
}

int main(void)
{
    for (int k = 0; k < KINDS; k++) {
        made_n = 0;
        watch = 1;
        unsigned long m0 = mallocs;
        FILE *f = open_kind(k);
        watch = 0;
        int opened = made_n;
        CHECK(f != NULL);
        dprintf(2, "x_self_bind_stdio: %s called the program's malloc family %lu times\n", names[k], mallocs - m0);
        CHECK(mallocs > m0);
        unsigned long f0 = frees;
        CHECK(f && fclose(f) == 0);
        int released = 0;
        for (int i = 0; i < opened; i++)
            released += made_freed[i];
        dprintf(2, "x_self_bind_stdio: fclose of %s called the program's free %lu times\n", names[k], frees - f0);
        /* The stream object goes back; open_memstream's buffer is the caller's to free. */
        CHECK(released >= 1);
        if (k != MEMSTREAM)
            CHECK(released == opened);
        if (k == MEMSTREAM)
            free(ms);
    }
    /* A replacement that fails: no stream, ENOMEM, no descriptor left open. */
    int low = lowest_fd();
    for (int k = 0; k < KINDS; k++) {
        fail = 1;
        errno = 0;
        FILE *f = open_kind(k);
        int e = errno;
        fail = 0;
        if (f) {
            dprintf(2, "FAIL: %s returned a stream with the allocator failing\n", names[k]);
            failures++;
            fclose(f);
        }
        if (e != ENOMEM) {
            dprintf(2, "FAIL: %s with the allocator failing: errno %d, expected ENOMEM\n", names[k], e);
            failures++;
        }
        CHECK(lowest_fd() == low);
    }
    return failures;
}
