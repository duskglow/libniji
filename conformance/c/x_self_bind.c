/* LIBC-SELF-BIND (D-27.76; no spec clause beyond the GNU C Library manual's "Replacing malloc"): which of
 * libc.so.6's own calls reach a definition the program makes of a name libc.so.6 also defines, and which
 * exported data the library reads from the program's COPY relocations. Run on the reference and on libniji
 * alike; both must give the same answers.
 *
 * - Malloc replacement (manual, "Replacing malloc"): the program's malloc family serves the library's own
 *   allocations: strdup, fopen and fclose, asprintf and getline allocate and free through it.
 * - Every other family libc.so.6 used to look up in the whole scope: the program defines one function of the
 *   family (counting its calls, then forwarding to the library's through RTLD_NEXT) and calls the library's
 *   other entry that reaches it in libniji: the compat version (dlvsym) of pthread, aio, sem, mq, clock,
 *   timer, C11 threads, dl*, ns*, dn*, the stdio _IO_ names, llseek, pthread_yield, __secure_getenv and
 *   openpty; the library's internal users of memcmp and bcmp (setlocale, newlocale, getaddrinfo, select) and
 *   the XDR stdio stream's fwrite, ftell and fflush. The reference reaches none of them (0 calls): the
 *   library's own entry runs.
 * - COPY relocations (the program's PIE text reads stdout and optind directly): the library's printf writes
 *   to the stream the program stored in its copy of stdout, getopt advances the program's optind, and a
 *   library loaded ahead of libc.so.6 (sbind_lib) sees the same copies.
 * The positive control for every family: the program's definition is the one the scope finds first
 * (dlsym(RTLD_DEFAULT)), so a library call that looked the name up would reach it.
 * Each failed check prints FAIL and the line; the exit status is the number of failures. */
/* LINK-HELPERS: sbind_lib */
#define _GNU_SOURCE
#include <aio.h>
#include <dlfcn.h>
#include <locale.h>
#include <mqueue.h>
#include <netdb.h>
#include <pthread.h>
#include <pty.h>
#include <sched.h>
#include <semaphore.h>
#include <signal.h>
#include <stdarg.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/select.h>
#include <threads.h>
#include <time.h>
#include <unistd.h>

static int failures;
#define CHECK(cond)                                                                                \
    do {                                                                                           \
        if (!(cond)) {                                                                             \
            failures++;                                                                            \
            dprintf(2, "FAIL line %d: %s\n", __LINE__, #cond);                                     \
        }                                                                                          \
    } while (0)

/* ---- The malloc family: a bump arena, blocks with their size in front. Pointers it did not make go to
 * the library's own allocator (a library may hold blocks made before or beside the program's). ---- */
#define ARENA (8u << 20)
static _Alignas(64) unsigned char arena[ARENA];
static size_t used;
/* volatile: the compiler takes builtin strdup to change no global */
static volatile unsigned long mallocs, frees;
static void *(*real_realloc)(void *, size_t);
static void (*real_free)(void *);

/* By address, out of the compiler's sight: a pointer a malloc-attributed function returned is not
 * assumed to lie outside the arena. */
static int ours(const void *p)
{
    uintptr_t v = (uintptr_t)p, a = (uintptr_t)arena;
    __asm__("" : "+r"(v));
    return v >= a && v < a + ARENA;
}
static void *take(size_t n, size_t align)
{
    if (align < 16)
        align = 16;
    size_t at = (used + 16 + align - 1) & ~(align - 1);
    if (at + n > ARENA)
        return NULL;
    used = at + n;
    *(size_t *)(arena + at - 16) = n;
    mallocs++;
    return arena + at;
}
static size_t size_of(const void *p) { return *(const size_t *)((const unsigned char *)p - 16); }
void *malloc(size_t n) { return take(n, 16); }
void *calloc(size_t k, size_t n)
{
    if (n && k > SIZE_MAX / n)
        return NULL;
    void *p = take(k * n, 16); /* the arena is zero and never reused */
    return p;
}
void free(void *p)
{
    frees++;
    if (p && !ours(p) && real_free)
        real_free(p);
}
void *realloc(void *p, size_t n)
{
    if (p && !ours(p))
        return real_realloc ? real_realloc(p, n) : NULL;
    void *q = take(n, 16);
    if (q && p) {
        size_t m = size_of(p) < n ? size_of(p) : n;
        for (size_t i = 0; i < m; i++)
            ((volatile unsigned char *)q)[i] = ((unsigned char *)p)[i];
    }
    return q;
}
void *aligned_alloc(size_t align, size_t n) { return take(n, align); }
void *memalign(size_t align, size_t n) { return take(n, align); }
int posix_memalign(void **out, size_t align, size_t n)
{
    void *p = take(n, align);
    if (!p)
        return 12;
    *out = p;
    return 0;
}
void *valloc(size_t n) { return take(n, 4096); }
void *pvalloc(size_t n) { return take((n + 4095) & ~(size_t)4095, 4096); }
size_t malloc_usable_size(void *p) { return p && ours(p) ? size_of(p) : 0; }

/* ---- memcmp and bcmp: self-contained (no forwarding), counted while armed. ---- */
static int armed;
static unsigned long cmp_hits;
__attribute__((optimize("no-tree-loop-distribute-patterns"))) static int cmp_bytes(const void *a, const void *b, size_t n)
{
    const volatile unsigned char *x = a, *y = b;
    for (size_t i = 0; i < n; i++)
        if (x[i] != y[i])
            return x[i] < y[i] ? -1 : 1;
    return 0;
}
int memcmp(const void *a, const void *b, size_t n)
{
    cmp_hits += armed;
    return cmp_bytes(a, b, n);
}
int bcmp(const void *a, const void *b, size_t n)
{
    cmp_hits += armed;
    return cmp_bytes(a, b, n);
}

/* ---- One counted definition per family, forwarding to the library's (RTLD_NEXT, resolved in main). ---- */
#define WRAP(ret, name, params, args)                                                              \
    static ret(*real_##name) params;                                                               \
    static unsigned long hits_##name;                                                              \
    ret name params                                                                                \
    {                                                                                              \
        hits_##name++;                                                                             \
        return real_##name args;                                                                   \
    }
WRAP(int, pthread_mutex_lock, (pthread_mutex_t * m), (m))
WRAP(int, aio_error, (const struct aiocb *cb), (cb))
WRAP(int, sem_post, (sem_t * s), (s))
WRAP(int, mq_getattr, (mqd_t q, struct mq_attr *a), (q, a))
WRAP(int, clock_gettime, (clockid_t c, struct timespec *t), (c, t))
WRAP(int, timer_gettime, (timer_t t, struct itimerspec *v), (t, v))
WRAP(int, mtx_lock, (mtx_t * m), (m))
WRAP(char *, dlerror, (void), ())
int ns_name_skip(const unsigned char **p, const unsigned char *end);
WRAP(int, ns_name_skip, (const unsigned char **p, const unsigned char *end), (p, end))
int dn_skipname(const unsigned char *p, const unsigned char *end);
WRAP(int, dn_skipname, (const unsigned char *p, const unsigned char *end), (p, end))
WRAP(off64_t, lseek64, (int fd, off64_t o, int w), (fd, o, w))
WRAP(int, sched_yield, (void), ())
WRAP(char *, secure_getenv, (const char *n), (n))
WRAP(int, fputs, (const char *restrict s, FILE *restrict f), (s, f))
WRAP(int, openpty, (int *m, int *s, char *n, const struct termios *t, const struct winsize *w), (m, s, n, t, w))
WRAP(size_t, fwrite, (const void *restrict p, size_t k, size_t n, FILE *restrict f), (p, k, n, f))
WRAP(long, ftell, (FILE * f), (f))
WRAP(int, fflush, (FILE * f), (f))

#define RESOLVE(name)                                                                              \
    do {                                                                                           \
        real_##name = dlsym(RTLD_NEXT, #name);                                                     \
        CHECK(real_##name != NULL);                                                                \
        /* the positive control: the scope's first definition is the program's */                 \
        CHECK(dlsym(RTLD_DEFAULT, #name) == (void *)name);                                         \
    } while (0)

static void *libc;
static void *entry(const char *name, const char *version)
{
    void *p = dlvsym(libc, name, version);
    if (!p)
        dprintf(2, "FAIL: libc.so.6 has no %s@%s\n", name, version), failures++;
    return p;
}
static void reached(const char *what, unsigned long hits)
{
    dprintf(2, "x_self_bind: %s reached the program's definition %lu times\n", what, hits);
    CHECK(hits == 0);
}

/* The XDR stream (the Sun RPC interface's layout). */
struct xdr_s;
struct xdr_ops_s {
    void *getlong, *putlong, *getbytes, *putbytes;
    unsigned (*getpostn)(const struct xdr_s *);
    void *setpostn, *inline_;
    void (*destroy)(struct xdr_s *);
    void *getint32, *putint32;
};
struct xdr_s {
    int op;
    const struct xdr_ops_s *ops;
    char *pub, *priv, *base;
    unsigned handy;
};

FILE *sbind_lib_stdout(void);
int *sbind_lib_optind(void);

int main(void)
{
    real_free = dlsym(RTLD_NEXT, "free");
    real_realloc = dlsym(RTLD_NEXT, "realloc");
    libc = dlopen("libc.so.6", RTLD_NOW | RTLD_NOLOAD);
    CHECK(libc != NULL);
    if (!libc || !real_free || !real_realloc)
        return 100;
    RESOLVE(pthread_mutex_lock);
    RESOLVE(aio_error);
    RESOLVE(sem_post);
    RESOLVE(mq_getattr);
    RESOLVE(clock_gettime);
    RESOLVE(timer_gettime);
    RESOLVE(mtx_lock);
    RESOLVE(dlerror);
    RESOLVE(ns_name_skip);
    RESOLVE(dn_skipname);
    RESOLVE(lseek64);
    RESOLVE(sched_yield);
    RESOLVE(secure_getenv);
    RESOLVE(fputs);
    RESOLVE(openpty);
    RESOLVE(fwrite);
    RESOLVE(ftell);
    RESOLVE(fflush);
    CHECK(dlsym(RTLD_DEFAULT, "memcmp") == (void *)memcmp);
    CHECK(dlsym(RTLD_DEFAULT, "malloc") == (void *)malloc);

    /* Malloc replacement: the library's own allocations are the program's. */
    unsigned long m0 = mallocs, f0 = frees;
    char *s = strdup("abc");
    CHECK(s && ours(s) && mallocs > m0);
    m0 = mallocs;
    FILE *n = fopen("/dev/null", "r");
    CHECK(n && mallocs > m0);
    CHECK(n && fclose(n) == 0 && frees > f0);
    char *as = NULL;
    CHECK(asprintf(&as, "%d-%s", 7, "x") == 3 && as && ours(as));
    FILE *mem = fmemopen((char[]){"one line\n"}, 9, "r");
    char *line = NULL;
    size_t cap = 0;
    CHECK(mem && getline(&line, &cap, mem) == 9 && line && ours(line));
    if (mem)
        fclose(mem);

    /* pthread: the GLIBC_2.2.5 name of pthread_mutex_lock. */
    pthread_mutex_t mu = PTHREAD_MUTEX_INITIALIZER;
    int (*mlock)(pthread_mutex_t *) = entry("__pthread_mutex_lock", "GLIBC_2.2.5");
    if (mlock) {
        hits_pthread_mutex_lock = 0;
        CHECK(mlock(&mu) == 0);
        reached("pthread: __pthread_mutex_lock -> pthread_mutex_lock", hits_pthread_mutex_lock);
        pthread_mutex_unlock(&mu);
    }
    /* aio */
    struct aiocb cb;
    for (size_t i = 0; i < sizeof cb; i++)
        ((volatile unsigned char *)&cb)[i] = 0;
    int (*aerr)(const struct aiocb *) = entry("aio_error", "GLIBC_2.2.5");
    if (aerr) {
        aerr(&cb);
        reached("aio: aio_error@GLIBC_2.2.5 -> aio_error", hits_aio_error);
    }
    /* sem */
    sem_t sem;
    CHECK(sem_init(&sem, 0, 0) == 0);
    int (*spost)(sem_t *) = entry("sem_post", "GLIBC_2.2.5");
    if (spost) {
        CHECK(spost(&sem) == 0);
        reached("sem: sem_post@GLIBC_2.2.5 -> sem_post", hits_sem_post);
    }
    /* mq: a descriptor that is not a queue (the call fails either way; only the path matters) */
    struct mq_attr qa;
    int (*qget)(mqd_t, struct mq_attr *) = entry("mq_getattr", "GLIBC_2.3.4");
    if (qget) {
        qget((mqd_t)-1, &qa);
        reached("mq: mq_getattr@GLIBC_2.3.4 -> mq_getattr", hits_mq_getattr);
    }
    /* clock and timer */
    struct timespec ts;
    int (*cget)(clockid_t, struct timespec *) = entry("clock_gettime", "GLIBC_2.2.5");
    if (cget) {
        CHECK(cget(CLOCK_MONOTONIC, &ts) == 0);
        reached("clock: clock_gettime@GLIBC_2.2.5 -> clock_gettime", hits_clock_gettime);
    }
    struct sigevent ev = {.sigev_notify = SIGEV_NONE};
    timer_t tm;
    CHECK(timer_create(CLOCK_MONOTONIC, &ev, &tm) == 0);
    struct itimerspec its;
    int (*tget)(timer_t, struct itimerspec *) = entry("timer_gettime", "GLIBC_2.3.3");
    if (tget) {
        CHECK(tget(tm, &its) == 0);
        reached("timer: timer_gettime@GLIBC_2.3.3 -> timer_gettime", hits_timer_gettime);
    }
    timer_delete(tm);
    /* C11 threads */
    mtx_t cm;
    CHECK(mtx_init(&cm, mtx_plain) == thrd_success);
    int (*clock_)(mtx_t *) = entry("mtx_lock", "GLIBC_2.28");
    if (clock_) {
        CHECK(clock_(&cm) == thrd_success);
        reached("C11 threads: mtx_lock@GLIBC_2.28 -> mtx_lock", hits_mtx_lock);
        mtx_unlock(&cm);
    }
    /* dl* */
    char *(*derr)(void) = entry("dlerror", "GLIBC_2.2.5");
    if (derr) {
        derr();
        reached("dl: dlerror@GLIBC_2.2.5 -> dlerror", hits_dlerror);
    }
    /* ns* and dn*: the encoded name "a" then the root */
    static const unsigned char name[] = {1, 'a', 0};
    const unsigned char *np = name;
    int (*nskip)(const unsigned char **, const unsigned char *) = entry("ns_name_skip", "GLIBC_2.9");
    if (nskip) {
        CHECK(nskip(&np, name + sizeof name) == 0 && np == name + 3);
        reached("ns: ns_name_skip@GLIBC_2.9 -> ns_name_skip", hits_ns_name_skip);
    }
    int (*dskip)(const unsigned char *, const unsigned char *) = entry("__dn_skipname", "GLIBC_2.2.5");
    if (dskip) {
        CHECK(dskip(name, name + sizeof name) == 3);
        reached("dn: __dn_skipname -> dn_skipname", hits_dn_skipname);
    }
    /* the rest */
    off64_t (*llseek_)(int, off64_t, int) = entry("llseek", "GLIBC_2.2.5");
    if (llseek_) {
        llseek_(0, 0, SEEK_CUR);
        reached("file: llseek -> lseek64", hits_lseek64);
    }
    int (*pyield)(void) = entry("pthread_yield", "GLIBC_2.2.5");
    if (pyield) {
        CHECK(pyield() == 0);
        reached("sched: pthread_yield -> sched_yield", hits_sched_yield);
    }
    char *(*sgetenv)(const char *) = entry("__secure_getenv", "GLIBC_2.2.5");
    if (sgetenv) {
        sgetenv("PATH");
        reached("env: __secure_getenv -> secure_getenv", hits_secure_getenv);
    }
    FILE *t = tmpfile();
    CHECK(t != NULL);
    int (*iofputs)(const char *, FILE *) = entry("_IO_fputs", "GLIBC_2.2.5");
    if (iofputs && t) {
        CHECK(iofputs("x", t) >= 0);
        reached("stdio: _IO_fputs -> fputs", hits_fputs);
    }
    int (*opty)(int *, int *, char *, const struct termios *, const struct winsize *) = entry("openpty", "GLIBC_2.2.5");
    if (opty) {
        int pm = -1, ps = -1;
        if (opty(&pm, &ps, NULL, NULL, NULL) == 0)
            close(pm), close(ps);
        reached("pty: openpty@GLIBC_2.2.5 -> openpty", hits_openpty);
    }
    /* memcmp and bcmp inside the library */
    armed = 1;
    locale_t l = newlocale(LC_ALL_MASK, "C.UTF-8", (locale_t)0);
    if (l)
        freelocale(l);
    setlocale(LC_ALL, "C.UTF-8");
    setlocale(LC_CTYPE, "C");
    setlocale(LC_ALL, "C");
    struct addrinfo hints = {.ai_flags = AI_NUMERICHOST, .ai_family = AF_UNSPEC};
    struct addrinfo *ai = NULL;
    if (getaddrinfo("127.0.0.1", NULL, &hints, &ai) == 0)
        freeaddrinfo(ai);
    struct timeval zero = {0, 0};
    select(0, NULL, NULL, NULL, &zero);
    armed = 0;
    reached("string: setlocale, newlocale, getaddrinfo, select -> memcmp, bcmp", cmp_hits);
    /* the XDR stdio stream */
    void (*xcreate)(struct xdr_s *, FILE *, int) = entry("xdrstdio_create", "GLIBC_2.2.5");
    int (*xint)(struct xdr_s *, int *) = entry("xdr_int", "GLIBC_2.2.5");
    if (xcreate && xint && t) {
        struct xdr_s x;
        int v = 0x01020304;
        hits_fwrite = hits_ftell = hits_fflush = 0;
        xcreate(&x, t, 0 /* XDR_ENCODE */);
        CHECK(xint(&x, &v) == 1);
        x.ops->getpostn(&x);
        x.ops->destroy(&x);
        reached("rpc: xdrstdio stream -> fwrite", hits_fwrite);
        reached("rpc: xdrstdio stream -> ftell", hits_ftell);
        reached("rpc: xdrstdio stream -> fflush", hits_fflush);
    }

    /* COPY relocations: the program's stdout and optind are the library's. */
    Dl_info prog, dout, dind;
    CHECK(dladdr((void *)main, &prog) && dladdr((void *)&stdout, &dout) && dladdr((void *)&optind, &dind));
    CHECK(dout.dli_fbase == prog.dli_fbase); /* the copy lies in the program */
    CHECK(dind.dli_fbase == prog.dli_fbase);
    FILE *old = stdout;
    if (t) {
        rewind(t);
        stdout = t;
        CHECK(sbind_lib_stdout() == t);
        printf("%d\n", 42);
        stdout = old;
        fflush(t);
        rewind(t);
        char buf[8] = {0};
        CHECK(fgets(buf, sizeof buf, t) && buf[0] == '4' && buf[1] == '2' && buf[2] == '\n');
        fclose(t);
    }
    char *av[] = {"p", "-x", "arg", NULL};
    optind = 1;
    opterr = 0;
    CHECK(sbind_lib_optind() == &optind);
    CHECK(getopt(3, av, "x") == 'x' && optind == 2);
    return failures;
}
