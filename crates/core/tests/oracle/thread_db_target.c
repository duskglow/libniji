/* The program thread_db_products.rs debugs (spec §22.10): the initial thread and three
   threads it creates, each with its own value of the thread-local `tv` (7, 111, 122, 133), all
   held at a barrier while the initial thread calls `niji_tdb_stop`, where the debugger stops. */
#include <pthread.h>
#include <stdlib.h>

__thread int tv = -1;
static pthread_barrier_t bar;

/* The debugger's breakpoint: every thread is set up and waiting. */
void __attribute__((noinline)) niji_tdb_stop(void) {
    __asm__ volatile("" ::: "memory");
}

static void *__attribute__((noinline)) niji_tdb_worker(void *arg) {
    tv = 100 + 11 * (int)(long)arg;
    pthread_barrier_wait(&bar); /* every value is set */
    pthread_barrier_wait(&bar); /* held while the debugger looks */
    return NULL;
}

int main(void) {
    pthread_t t[3];
    tv = 7;
    if (pthread_barrier_init(&bar, NULL, 4) != 0) return 2;
    for (long i = 0; i < 3; i++)
        if (pthread_create(&t[i], NULL, niji_tdb_worker, (void *)(i + 1)) != 0) return 3;
    pthread_barrier_wait(&bar);
    niji_tdb_stop();
    pthread_barrier_wait(&bar);
    for (int i = 0; i < 3; i++)
        if (pthread_join(t[i], NULL) != 0) return 4;
    return tv == 7 ? 0 : 5;
}
