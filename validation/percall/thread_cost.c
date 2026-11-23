/* THREAD-CREATE-COST: one thread's whole life, counted from the creator: pthread_create, the start routine's
 * return or pthread_exit, pthread_join. Every thread of the process is counted (perf stat follows the threads).
 * thread_create_join: create + return + join, one thread at a time (the serial create-heavy workload).
 * thread_create_exit: create + pthread_exit + join. thread_create_join8: 8 created, then 8 joined (per = 8: the
 * cost of one thread when several are alive at once). thread_exit_deep: pthread_exit five frames down. */
#include "percall.h"
#include <pthread.h>

static void *ret_arg(void *a) { return a; }

static void *exit_arg(void *a) { pthread_exit(a); }

__attribute__((noinline)) static void deep(void *a, int k) {
    if (k == 0) pthread_exit(a);
    deep(a, k - 1);
    pc_sink++;
}

static void *exit_deep(void *a) {
    deep(a, 4);
    return 0;
}

static void one(void *(*f)(void *), long i) {
    pthread_t t;
    void *r = 0;
    if (pthread_create(&t, 0, f, (void *)i) != 0 || pthread_join(t, &r) != 0 || r != (void *)i) abort();
}

CASE(thread_create_join, 1000, 1) {
    (void)dir;
    for (long i = 0; i < n; i++) one(ret_arg, i);
    PC_CHECKING { one(ret_arg, 7); pc_check("ok"); }
}

CASE(thread_create_exit, 1000, 1) {
    (void)dir;
    for (long i = 0; i < n; i++) one(exit_arg, i);
    PC_CHECKING { one(exit_arg, 7); pc_check("ok"); }
}

CASE(thread_exit_deep, 100, 1) {
    (void)dir;
    for (long i = 0; i < n; i++) one(exit_deep, i);
    PC_CHECKING { one(exit_deep, 7); pc_check("ok"); }
}

CASE(thread_create_join8, 1000, 8) {
    (void)dir;
    for (long i = 0; i < n; i++) {
        pthread_t t[8];
        void *r;
        for (long k = 0; k < 8; k++)
            if (pthread_create(&t[k], 0, ret_arg, (void *)k) != 0) abort();
        for (long k = 0; k < 8; k++)
            if (pthread_join(t[k], &r) != 0 || r != (void *)k) abort();
    }
    PC_CHECKING { one(ret_arg, 7); pc_check("ok"); }
}
