/* TLS-SURPLUS-PROBE: how much TLS can objects opened by dlopen after start-up use?
 *
 *   probe <threads> <object.so>...
 *
 * <threads> 1: one thread is created before the first dlopen and kept alive; after each open it
 * touches the new object's block too (a block of its own, distinct from the main thread's).
 * Each object (built from tls_obj.c) is opened in order with RTLD_NOW; the first failure stops.
 * Prints "libc=<host|libniji> loaded=<n> ok" or "libc=... loaded=<n> fail: <dlerror text>".
 * Exit 0: all opened; 1: an open failed; 2: a check failed (a wrong block address).
 * No spec clause: a black-box measurement probe (audit row HL-33), not a test. */
#include <dlfcn.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

typedef void *(*touch_fn)(void);

static int to_thread[2], from_thread[2];

static void *worker(void *arg) {
    (void)arg;
    for (;;) {
        touch_fn f;
        if (read(to_thread[0], &f, sizeof f) != (ssize_t)sizeof f || f == NULL) return NULL;
        void *a = f();
        if (write(from_thread[1], &a, sizeof a) != (ssize_t)sizeof a) return NULL;
    }
}

/* Which C library is mapped: the libniji build directory's or the host's. */
static const char *which_libc(void) {
    FILE *m = fopen("/proc/self/maps", "r");
    char line[4096];
    const char *r = "unknown";
    if (m == NULL) return r;
    while (fgets(line, sizeof line, m) != NULL) {
        if (strstr(line, "/libc.so.6") == NULL) continue;
        r = strstr(line, "target/niji/") != NULL ? "libniji" : "host";
        break;
    }
    fclose(m);
    return r;
}

int main(int argc, char **argv) {
    if (argc < 3) {
        fprintf(stderr, "usage: probe <threads 0|1> <object.so>...\n");
        return 2;
    }
    int threads = atoi(argv[1]);
    pthread_t t;
    if (threads) {
        if (pipe(to_thread) != 0 || pipe(from_thread) != 0 || pthread_create(&t, NULL, worker, NULL) != 0) {
            fprintf(stderr, "probe: thread set-up failed\n");
            return 2;
        }
    }
    const char *libc = which_libc();
    int n = 0, rc = 0;
    for (int i = 2; i < argc; i++) {
        void *h = dlopen(argv[i], RTLD_NOW | RTLD_LOCAL);
        if (h == NULL) {
            const char *e = dlerror();
            printf("libc=%s loaded=%d fail: %s\n", libc, n, e ? e : "(no dlerror text)");
            rc = 1;
            break;
        }
        touch_fn f = (touch_fn)dlsym(h, "tls_touch");
        if (f == NULL) {
            printf("libc=%s loaded=%d BAD: no tls_touch in %s\n", libc, n, argv[i]);
            rc = 2;
            break;
        }
        void *mine = f();
        if (threads) {
            void *theirs = NULL;
            if (write(to_thread[1], &f, sizeof f) != (ssize_t)sizeof f ||
                read(from_thread[0], &theirs, sizeof theirs) != (ssize_t)sizeof theirs ||
                theirs == NULL || theirs == mine) {
                printf("libc=%s loaded=%d BAD: thread block %p, main block %p\n", libc, n, theirs, mine);
                rc = 2;
                break;
            }
        }
        n++;
    }
    if (rc == 0) printf("libc=%s loaded=%d ok\n", libc, n);
    if (threads) {
        touch_fn stop = NULL;
        if (write(to_thread[1], &stop, sizeof stop) == (ssize_t)sizeof stop) pthread_join(t, NULL);
    }
    fflush(stdout);
    return rc;
}
