/* Spec §27.3 (s27_3_c_0: "The block is allocated on first access for modules loaded by dlopen";
 * s27_3_f_0: an allocation failure aborts the process) and the GNU C Library manual's
 * glibc.rtld.optional_static_tls tunable. Objects with global-dynamic or TLS-descriptor TLS and no
 * DF_STATIC_TLS do not need static TLS room:
 *  1. 64 copies of a 16 KiB object (1 MiB, far beyond any static surplus) open at once; the main
 *     thread, a thread created before the opens and one created after them each see the initial
 *     image then zeros in blocks of their own, and their writes stay their own. In the late
 *     thread, dlinfo's RTLD_DI_TLS_DATA and dl_iterate_phdr's dlpi_tls_data are NULL before its
 *     first access (the block does not exist yet) and the block after it.
 *  2. A small initial-exec object (DF_STATIC_TLS) still opens after them: they took no static room.
 *  3. 200 cycles of open, touch (the main thread and a new thread, which ends before or after the
 *     dlclose), close: the address space does not grow (dlclose and thread exit free the blocks).
 *  4. A 1 GiB block under a 512 MiB address-space limit: the first access ends the process with
 *     status 127 and "cannot allocate memory for thread-local data" (a forked child).
 *  5. GLIBC_TUNABLES=glibc.rtld.optional_static_tls=262144 lets a 128 KiB initial-exec object open
 *     (a re-executed child; without the tunable it fails, s27_dlopen_tls_surplus).
 * Each failed check prints its line; the exit status is the number of failed checks. */
/* DLOPEN-HELPERS: dld_gd dld_desc dld_ie dld_huge dlt_big */
/* HELPER-FLAGS: dld_desc -mtls-dialect=gnu2 */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <fcntl.h>
#include <link.h>
#include <pthread.h>
#include <semaphore.h>
#include <stdatomic.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/resource.h>
#include <sys/wait.h>
#include <unistd.h>

static atomic_int fails;
#define CHECK(who, c) do { if (!(c)) { fprintf(stderr, "s27_dlopen_tls_dynamic: %s: line %d: %s\n", (who), __LINE__, #c); atomic_fetch_add(&fails, 1); } } while (0)

#define N 64
#define SIZE (16 * 1024)
typedef char *(*getter)(void);
static getter get[N];
static char *main_block[N];
static char tmp[64];
static sem_t go, done;

static const char *at(const char *dir, const char *name)
{
    static char buf[4096];
    snprintf(buf, sizeof buf, "%s/%s", dir, name);
    return buf;
}

static int copy(const char *from, const char *to)
{
    int in = open(from, O_RDONLY | O_CLOEXEC);
    int out = open(to, O_WRONLY | O_CREAT | O_TRUNC | O_CLOEXEC, 0755);
    char buf[65536];
    ssize_t n = 0;
    int ok = in >= 0 && out >= 0;
    while (ok && (n = read(in, buf, sizeof buf)) > 0)
        ok = write(out, buf, (size_t)n) == n;
    ok = ok && n == 0;
    if (in >= 0)
        close(in);
    if (out >= 0)
        ok = close(out) == 0 && ok;
    return ok;
}

/* The calling thread's first look at every block: the image, then zeros; then mark it `mark`. */
static void fresh(const char *who, char mark)
{
    for (int i = 0; i < N; i++) {
        if (get[i] == NULL)
            continue;
        char *b = get[i]();
        int zeros = 1;
        for (int k = 2; k < SIZE; k++)
            zeros &= b[k] == 0;
        CHECK(who, b[0] == 7 && b[1] == 9 && zeros);
        CHECK(who, get[i]() == b);
        b[0] = mark;
        b[SIZE - 1] = mark;
    }
}

/* The calling thread's blocks still hold `mark` and are not the main thread's. */
static void kept(const char *who, char mark, int other)
{
    for (int i = 0; i < N; i++) {
        if (get[i] == NULL)
            continue;
        char *b = get[i]();
        CHECK(who, b[0] == mark && b[SIZE - 1] == mark);
        if (other)
            CHECK(who, b != main_block[i]);
    }
}

static void *early(void *arg)
{
    (void)arg;
    sem_wait(&go);
    fresh("early thread", 2);
    kept("early thread", 2, 1);
    return NULL;
}

static void *h[N];

struct seek { size_t modid; void *data; int found; };
static int seek_one(struct dl_phdr_info *i, size_t size, void *arg)
{
    struct seek *s = arg;
    (void)size;
    if (i->dlpi_tls_modid == s->modid) {
        s->data = i->dlpi_tls_data;
        s->found++;
    }
    return 0;
}

/* dlinfo's RTLD_DI_TLS_DATA and dl_iterate_phdr's dlpi_tls_data for copy `i` in the calling thread
 * are `want` (NULL before the thread's first access: the block does not exist yet). */
static void block_seen(const char *who, int i, void *want)
{
    size_t modid = 0;
    void *data = (void *)1;
    CHECK(who, h[i] != NULL && dlinfo(h[i], RTLD_DI_TLS_MODID, &modid) == 0 && modid != 0);
    CHECK(who, dlinfo(h[i], RTLD_DI_TLS_DATA, &data) == 0 && data == want);
    struct seek s = {modid, (void *)1, 0};
    dl_iterate_phdr(seek_one, &s);
    CHECK(who, s.found == 1 && s.data == want);
}

static void *late(void *arg)
{
    (void)arg;
    block_seen("late thread", 5, NULL);
    fresh("late thread", 3);
    block_seen("late thread", 5, get[5] ? get[5]() : NULL);
    kept("late thread", 3, 1);
    return NULL;
}

static long vm_pages(void)
{
    long pages = -1;
    FILE *f = fopen("/proc/self/statm", "r");
    if (f != NULL) {
        if (fscanf(f, "%ld", &pages) != 1)
            pages = -1;
        fclose(f);
    }
    return pages;
}

/* A thread of a growth cycle: touch the block, tell the main thread, wait for its word, end. */
static void *toucher(void *arg)
{
    char *b = ((getter)arg)();
    CHECK("cycle thread", b[0] == 7 && b[100] == 0);
    b[100] = 1;
    sem_post(&done);
    sem_wait(&go);
    return NULL;
}

static void growth(const char *dir)
{
    long base = -1;
    for (int c = 0; c < 200; c++) {
        if (c == 20)
            base = vm_pages();
        void *h = dlopen(at(dir, c % 2 ? "libdld_desc.so" : "libdld_gd.so"), RTLD_NOW);
        getter g = h ? (getter)dlsym(h, "dld_at") : NULL;
        CHECK("cycle", g != NULL);
        if (g == NULL)
            return;
        char *b = g();
        CHECK("cycle", b[0] == 7 && b[100] == 0);
        b[100] = 1;
        pthread_t t;
        CHECK("cycle", pthread_create(&t, NULL, toucher, (void *)g) == 0);
        sem_wait(&done);
        if (c % 4 < 2) {
            /* the thread ends first, then the object goes */
            sem_post(&go);
            pthread_join(t, NULL);
            CHECK("cycle", dlclose(h) == 0);
        } else {
            /* the object goes while the thread still has its block */
            CHECK("cycle", dlclose(h) == 0);
            sem_post(&go);
            pthread_join(t, NULL);
        }
    }
    long end = vm_pages();
    CHECK("cycle", base > 0 && end > 0 && end - base <= 64);
    if (end - base > 64)
        fprintf(stderr, "s27_dlopen_tls_dynamic: address space grew from %ld to %ld pages\n", base, end);
}

/* A 1 GiB block under a 512 MiB limit, in a child: its first access ends it with status 127. */
static void no_memory(const char *dir)
{
    int p[2];
    CHECK("oom", pipe(p) == 0);
    fflush(NULL);
    pid_t pid = fork();
    if (pid == 0) {
        dup2(p[1], 2);
        close(p[0]);
        struct rlimit lim = {512L << 20, 512L << 20};
        if (setrlimit(RLIMIT_AS, &lim) != 0)
            _exit(90);
        void *h = dlopen(at(dir, "libdld_huge.so"), RTLD_NOW);
        if (h == NULL) {
            fprintf(stderr, "open failed: %s\n", dlerror());
            _exit(91);
        }
        getter g = (getter)dlsym(h, "dld_huge_at");
        if (g == NULL)
            _exit(92);
        volatile char *b = g();
        b[0] = 1;
        _exit(93);
    }
    close(p[1]);
    char text[1024];
    size_t got = 0;
    ssize_t n;
    while ((n = read(p[0], text + got, sizeof text - 1 - got)) > 0)
        got += (size_t)n;
    text[got] = 0;
    close(p[0]);
    int st = 0;
    CHECK("oom", pid > 0 && waitpid(pid, &st, 0) == pid);
    CHECK("oom", WIFEXITED(st) && WEXITSTATUS(st) == 127);
    CHECK("oom", strstr(text, "cannot allocate memory for thread-local data") != NULL);
    if (!WIFEXITED(st) || WEXITSTATUS(st) != 127)
        fprintf(stderr, "s27_dlopen_tls_dynamic: oom child: status %#x, stderr [%s]\n", st, text);
}

/* The re-executed child of `raised`: the 128 KiB initial-exec object opens. */
static int raised_child(const char *dir)
{
    void *h = dlopen(at(dir, "libdlt_big.so"), RTLD_NOW);
    if (h == NULL)
        fprintf(stderr, "s27_dlopen_tls_dynamic: tunable child: %s\n", dlerror());
    getter g = h ? (getter)dlsym(h, "dlt_big_at") : NULL;
    CHECK("tunable child", g != NULL);
    if (g != NULL) {
        char *b = g();
        CHECK("tunable child", b[0] == 0 && b[128 * 1024 - 1] == 0);
        b[128 * 1024 - 1] = 1;
    }
    return atomic_load(&fails);
}

static void raised(void)
{
    fflush(NULL);
    pid_t pid = fork();
    if (pid == 0) {
        setenv("GLIBC_TUNABLES", "glibc.rtld.optional_static_tls=262144", 1);
        char *argv[] = {"s27_dlopen_tls_dynamic", "tunable", NULL};
        execv("/proc/self/exe", argv);
        _exit(95);
    }
    int st = 0;
    CHECK("tunable", pid > 0 && waitpid(pid, &st, 0) == pid);
    CHECK("tunable", WIFEXITED(st) && WEXITSTATUS(st) == 0);
}

int main(int argc, char **argv)
{
    const char *dir = getenv("CONF_HELPERS");
    if (dir == NULL) {
        fprintf(stderr, "s27_dlopen_tls_dynamic: CONF_HELPERS is not set\n");
        return 1;
    }
    if (argc > 1 && strcmp(argv[1], "tunable") == 0)
        return raised_child(dir);

    /* 1. 64 copies (distinct files) of the two 16 KiB objects, open at once. */
    const char *base = getenv("TMPDIR");
    snprintf(tmp, sizeof tmp, "%s/dldXXXXXX", base && strlen(base) < 40 ? base : "/tmp");
    CHECK("main", mkdtemp(tmp) != NULL);
    pthread_t t0, t1;
    CHECK("main", sem_init(&go, 0, 0) == 0 && sem_init(&done, 0, 0) == 0);
    CHECK("main", pthread_create(&t0, NULL, early, NULL) == 0);
    char path[4096];
    for (int i = 0; i < N; i++) {
        snprintf(path, sizeof path, "%s/copy%d.so", tmp, i);
        CHECK("main", copy(at(dir, i % 2 ? "libdld_desc.so" : "libdld_gd.so"), path));
        h[i] = dlopen(path, RTLD_NOW);
        if (h[i] == NULL)
            fprintf(stderr, "s27_dlopen_tls_dynamic: open %d: %s\n", i, dlerror());
        get[i] = h[i] ? (getter)dlsym(h[i], "dld_at") : NULL;
        CHECK("main", get[i] != NULL);
        unlink(path);
    }
    rmdir(tmp);
    fresh("main", 1);
    for (int i = 0; i < N; i++)
        main_block[i] = get[i] ? get[i]() : NULL;
    for (int i = 0; i + 1 < N; i++)
        CHECK("main", main_block[i] == NULL || main_block[i + 1] == NULL ||
                          main_block[i] + SIZE <= main_block[i + 1] || main_block[i + 1] + SIZE <= main_block[i]);
    sem_post(&go);
    pthread_join(t0, NULL);
    CHECK("main", pthread_create(&t1, NULL, late, NULL) == 0);
    pthread_join(t1, NULL);
    kept("main", 1, 0);

    /* 2. Static room is still there for an initial-exec object. */
    void *hie = dlopen(at(dir, "libdld_ie.so"), RTLD_NOW);
    if (hie == NULL)
        fprintf(stderr, "s27_dlopen_tls_dynamic: initial-exec object: %s\n", dlerror());
    int *(*ie)(void) = hie ? (int *(*)(void))dlsym(hie, "dld_ie_at") : NULL;
    CHECK("main", ie != NULL && *ie() == 5);
    if (hie != NULL)
        CHECK("main", dlclose(hie) == 0);
    for (int i = 0; i < N; i++)
        if (h[i] != NULL)
            CHECK("main", dlclose(h[i]) == 0);

    /* 3. to 5. */
    growth(dir);
    no_memory(dir);
    raised();
    return atomic_load(&fails);
}
