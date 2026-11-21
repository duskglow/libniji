/* Spec §27.1 (s27_1_c_5: dlerror() returns the text of the last failure on this thread, clears it,
 * NULL when none is pending; spec/behaviour/dynlink.md probe P-6). Checks, in order:
 *   1. the main thread's first dlerror() is NULL; a failed dlsym gives a text naming the symbol,
 *      once; a later successful dlsym clears a pending text;
 *   2. 1000 rounds of two threads in lock step (barriers, no sleeps): A fails a dlsym and does not
 *      read; B reads NULL, fails a dlopen of its own; A then reads its own text (not B's), once;
 *      B reads its own; a thread created while A's text is pending reads NULL; the main thread's
 *      text, set before the threads started, is still its own afterwards;
 *   3. a thread that fails and exits leaves nothing behind: 2000 such threads, created and joined
 *      one at a time, add less than 2 MiB to the address space (VmSize), which a per-thread text
 *      kept past the thread's exit would exceed.
 * Each failed check prints its line; the exit status is the number of failed checks. */
/* LINK-HELPERS: dlh_add */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <pthread.h>
#include <stdatomic.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static atomic_int fails;
#define CHECK(who, c) do { if (!(c)) { fprintf(stderr, "s27_dlerror_threads: %s: line %d: %s\n", (who), __LINE__, #c); atomic_fetch_add(&fails, 1); } } while (0)

#define ROUNDS 1000
#define EXITERS 2000

static pthread_barrier_t bar;

/* The text pending on the calling thread contains `want` (NULL: no text is pending), and nothing
 * is pending after it was read. */
static int took(const char *want)
{
    const char *e = dlerror();
    int ok = want == NULL ? e == NULL : e != NULL && strstr(e, want) != NULL;
    if (!ok)
        fprintf(stderr, "s27_dlerror_threads: want [%s], got [%s]\n", want ? want : "NULL", e ? e : "NULL");
    if (dlerror() != NULL)
        ok = 0;
    return ok;
}

static void *late_main(void *arg)
{
    (void)arg;
    CHECK("late", took(NULL));
    return NULL;
}

static void *a_main(void *arg)
{
    (void)arg;
    for (int i = 0; i < ROUNDS; i++) {
        CHECK("A", dlsym(RTLD_DEFAULT, "s27_dlerror_missing_a") == NULL);
        if (i % 100 == 0) {
            pthread_t l;
            CHECK("A", pthread_create(&l, NULL, late_main, NULL) == 0);
            pthread_join(l, NULL);
        }
        pthread_barrier_wait(&bar); /* A's text is pending */
        pthread_barrier_wait(&bar); /* B has read and failed */
        CHECK("A", took("s27_dlerror_missing_a"));
        pthread_barrier_wait(&bar); /* round done */
    }
    return NULL;
}

static void *b_main(void *arg)
{
    (void)arg;
    for (int i = 0; i < ROUNDS; i++) {
        pthread_barrier_wait(&bar);
        CHECK("B", took(NULL));
        CHECK("B", dlopen("/nonexistent/s27_dlerror_missing_b.so", RTLD_NOW) == NULL);
        pthread_barrier_wait(&bar);
        CHECK("B", took("s27_dlerror_missing_b.so"));
        pthread_barrier_wait(&bar);
    }
    return NULL;
}

static void *exiter(void *arg)
{
    (void)arg;
    (void)dlsym(RTLD_DEFAULT, "s27_dlerror_missing_exiter");
    return NULL;
}

static long vm_size_kib(void)
{
    FILE *f = fopen("/proc/self/status", "r");
    if (f == NULL)
        return -1;
    char line[256];
    long kib = -1;
    while (fgets(line, sizeof line, f) != NULL)
        if (sscanf(line, "VmSize: %ld kB", &kib) == 1)
            break;
    fclose(f);
    return kib;
}

static void churn(int n)
{
    for (int i = 0; i < n; i++) {
        pthread_t t;
        if (pthread_create(&t, NULL, exiter, NULL) != 0) {
            CHECK("main", !"pthread_create");
            return;
        }
        pthread_join(t, NULL);
    }
}

int main(void)
{
    /* 1. One thread. */
    CHECK("main", took(NULL));
    CHECK("main", dlsym(RTLD_DEFAULT, "s27_dlerror_missing_main") == NULL);
    CHECK("main", took("s27_dlerror_missing_main"));
    CHECK("main", dlsym(RTLD_DEFAULT, "s27_dlerror_missing_main") == NULL);
    CHECK("main", dlsym(RTLD_DEFAULT, "dlh_add") != NULL);
    CHECK("main", took(NULL));

    /* 2. Two threads in lock step; the main thread's own text pending throughout. */
    CHECK("main", dlsym(RTLD_DEFAULT, "s27_dlerror_missing_main") == NULL);
    CHECK("main", pthread_barrier_init(&bar, NULL, 2) == 0);
    pthread_t a, b;
    CHECK("main", pthread_create(&a, NULL, a_main, NULL) == 0);
    CHECK("main", pthread_create(&b, NULL, b_main, NULL) == 0);
    pthread_join(a, NULL);
    pthread_join(b, NULL);
    CHECK("main", took("s27_dlerror_missing_main"));

    /* 3. Threads that fail and exit: warm the thread and stack caches first. */
    churn(64);
    long before = vm_size_kib();
    churn(EXITERS);
    long after = vm_size_kib();
    CHECK("main", before > 0 && after > 0);
    CHECK("main", after - before < 2048);
    if (after - before >= 2048)
        fprintf(stderr, "s27_dlerror_threads: VmSize %ld kB -> %ld kB over %d exited threads\n", before, after, EXITERS);
    return atomic_load(&fails);
}
