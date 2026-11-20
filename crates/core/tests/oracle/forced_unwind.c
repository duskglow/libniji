/* pthread_cleanup_push handlers and __attribute__((cleanup)) when a thread calls pthread_exit or
 * acts on cancellation (spec §22.1 s22_1_c_2, §22.3 s22_3_c_4; D-22.19): built dynamically by
 * forced_unwind_products.rs twice, plainly (no unwinder in the process: only the cleanup buffers
 * run) and with -fexceptions (the handlers are cleanups of the unwind, and run in turn with the
 * cleanup attribute's). Each line gives the order the cleanups ran in ('3', 'a', '1' from the
 * innermost frame out; 'K' the key destructor, after them) and the thread's result. */
#include <pthread.h>
#include <stdio.h>
#include <time.h>
#include <unistd.h>

static char seq[32];
static int nseq;
static pthread_key_t key;
static void add(char c) { if (nseq < 31) seq[nseq++] = c; }
static void handler(void *a) { add(*(char *)a); }
static void cleanup(char *p) { add(*p); }
static void key_destructor(void *v) { (void)v; add('K'); }

static void f3(int cancel)
{
    char c = '3';
    pthread_cleanup_push(handler, &c);
    if (cancel)
        pause();
    else
        pthread_exit((void *)9);
    pthread_cleanup_pop(0);
}
static void f2(int cancel)
{
    char c __attribute__((cleanup(cleanup))) = 'a';
    f3(cancel);
}
static void *thread(void *arg)
{
    char c = '1';
    pthread_setspecific(key, (void *)1);
    pthread_cleanup_push(handler, &c);
    f2((int)(long)arg);
    pthread_cleanup_pop(0);
    return NULL;
}

static void run(const char *name, int cancel)
{
    pthread_t t;
    void *r;
    nseq = 0;
    pthread_create(&t, NULL, thread, (void *)(long)cancel);
    if (cancel) {
        struct timespec ts = {0, 20 * 1000000L};
        nanosleep(&ts, NULL);
        pthread_cancel(t);
    }
    pthread_join(t, &r);
    printf("%s: order=%.*s result=%ld\n", name, nseq, seq, r == PTHREAD_CANCELED ? -1L : (long)r);
}

int main(void)
{
    setvbuf(stdout, NULL, _IOLBF, 0);
    pthread_key_create(&key, key_destructor);
    run("exit", 0);
    run("cancel", 1);
    return 0;
}
