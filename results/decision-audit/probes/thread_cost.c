/* DECISION-EVIDENCE-AUDIT probe: pthread_create+join cost and the thread mapping's shape.
 * Usage: thread_cost N      run N create+join pairs (count instructions for N and 2N; the difference / N
 *                           is the per-pair cost with start-up cancelled)
 *        thread_cost maps   print the new thread's mapping (size, where TP and the stack sit in it)  */
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>

static void *nothing(void *a) { return a; }

static uintptr_t g_sp, g_tp;
static void *where(void *a) {
    int local;
    g_sp = (uintptr_t)&local;
    g_tp = (uintptr_t)pthread_self();
    return a;
}

int main(int argc, char **argv) {
    if (argc > 1 && strcmp(argv[1], "maps") == 0) {
        pthread_t t;
        pthread_attr_t at;
        size_t ss = 0, gs = 0;
        pthread_attr_init(&at);
        pthread_attr_getstacksize(&at, &ss);
        pthread_attr_getguardsize(&at, &gs);
        if (pthread_create(&t, 0, where, 0) || pthread_join(t, 0)) return 1;
        /* a second thread reuses the first's (cached) mapping on both libraries */
        FILE *f = fopen("/proc/self/maps", "r");
        char line[512];
        printf("default stacksize %zu guard %zu\n", ss, gs);
        while (f && fgets(line, sizeof line, f)) {
            unsigned long lo, hi;
            if (sscanf(line, "%lx-%lx", &lo, &hi) == 2 && g_tp >= lo && g_tp < hi)
                printf("tp mapping %lu KiB; tp is %lu bytes below its end; sp-to-tp %lu bytes\n",
                       (hi - lo) >> 10, hi - g_tp, g_tp - g_sp);
        }
        return 0;
    }
    long n = argc > 1 ? atol(argv[1]) : 1000;
    for (long i = 0; i < n; i++) {
        pthread_t t;
        if (pthread_create(&t, 0, nothing, 0) || pthread_join(t, 0)) return 1;
    }
    return 0;
}
