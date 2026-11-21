/* HEAP-RSEQ-NODE-2: a new thread's first allocation, the heap slow path that reads the thread's NUMA node on a
 * multi-node policy (D-3.12, D-3.21: the rseq area's node_id, else the vDSO getcpu). Compare on a host with two
 * or more allowed nodes; on one node the node is never read (D-3.20) and the row only tracks thread creation. */
#include "percall.h"
#include <pthread.h>

static void *first_malloc(void *a) {
    void *volatile p = malloc(64);
    free(p);
    return a;
}

CASE(thread_first_malloc, 1000, 1) {
    (void)dir;
    for (long i = 0; i < n; i++) {
        pthread_t t;
        if (pthread_create(&t, 0, first_malloc, 0) != 0 || pthread_join(t, 0) != 0) abort();
    }
}
