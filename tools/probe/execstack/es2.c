/* Black-box probe (COMPAT19-TUNABLE): the stacks an executable-stack request reaches. Prints the
 * permissions of the main stack, of a thread stack the library made before the dlopen, of a
 * caller-supplied stack (pthread_attr_setstack) in use before it, and of a thread stack made
 * after it. With no argument nothing is opened (for a program whose startup dependency asks).
 * No library source consulted. */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <pthread.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <sys/mman.h>
#include <unistd.h>

static void perms_of(uintptr_t a, char out[5]) {
    FILE *f = fopen("/proc/self/maps", "r");
    char line[512];
    strcpy(out, "????");
    while (f && fgets(line, sizeof line, f)) {
        unsigned long lo, hi;
        char p[5];
        if (sscanf(line, "%lx-%lx %4s", &lo, &hi, p) == 3 && a >= lo && a < hi) {
            memcpy(out, p, 5);
            break;
        }
    }
    if (f) fclose(f);
}

static volatile int go;
struct slot { volatile uintptr_t at; };
static void *thr(void *arg) {
    volatile char here;
    ((struct slot *)arg)->at = (uintptr_t)&here;
    while (!go) usleep(1000);
    return 0;
}

static pthread_t start(struct slot *s, void *stack, size_t len) {
    pthread_t th;
    pthread_attr_t a;
    pthread_attr_init(&a);
    if (stack) pthread_attr_setstack(&a, stack, len);
    if (pthread_create(&th, &a, thr, s) != 0) { puts("pthread_create failed"); _exit(3); }
    while (!s->at) usleep(1000);
    return th;
}

int main(int argc, char **argv) {
    volatile char here;
    char m[5], a[5], u[5], c[5];
    struct slot sa = {0}, su = {0}, sc = {0};
    size_t len = 1 << 20;
    void *own = mmap(0, len, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    pthread_t ta = start(&sa, 0, 0), tu = start(&su, own, len), tc;
    if (argc > 1) {
        void *h = dlopen(argv[1], RTLD_NOW);
        printf("dlopen %s: %s\n", argv[1], h ? "ok" : dlerror());
    }
    tc = start(&sc, 0, 0);
    perms_of((uintptr_t)&here, m);
    perms_of(sa.at, a);
    perms_of(su.at, u);
    perms_of(sc.at, c);
    printf("main %s before %s own-stack %s after %s\n", m, a, u, c);
    go = 1;
    pthread_join(ta, 0);
    pthread_join(tu, 0);
    pthread_join(tc, 0);
    return 0;
}
