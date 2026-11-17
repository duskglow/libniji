/* The heap's address space at the steps of GitHub issue #21 (HEAP-ADDRESS-SPACE): prints
 * "<step> <VmSize KiB>" per step to stdout (no stdio, so only the steps allocate). With an
 * argument N, RLIMIT_AS is first set to the VmSize at start plus N KiB, and every step must
 * still succeed (exit 10..12 names the step that failed). */
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/resource.h>
#include <unistd.h>

#define N 200000
static void *blocks[N];
static char buf[8192];

static long vmsize(void) {
    int fd = open("/proc/self/status", O_RDONLY);
    ssize_t n = fd < 0 ? -1 : read(fd, buf, sizeof buf - 1);
    if (fd >= 0) close(fd);
    buf[n > 0 ? n : 0] = 0;
    char *p = strstr(buf, "VmSize:");
    return p ? strtol(p + 7, 0, 10) : -1;
}

static void step(const char *name) {
    char line[64];
    int len = snprintf(line, sizeof line, "%s %ld\n", name, vmsize());
    if (write(1, line, (size_t)len) != len) _exit(3);
}

int main(int argc, char **argv) {
    if (argc > 1) {
        rlim_t lim = ((rlim_t)vmsize() + (rlim_t)strtol(argv[1], 0, 10)) * 1024;
        struct rlimit rl = {lim, lim};
        if (setrlimit(RLIMIT_AS, &rl) != 0) return 4;
    }
    step("start");
    void *one = malloc(16);
    if (!one) return 9;
    step("malloc1");
    for (int i = 0; i < N; i++)
        if (!(blocks[i] = malloc(96 + (size_t)(i % 64)))) return 10;
    step("alloc");
    for (int i = 1; i < N; i += 2) free(blocks[i]);
    for (int i = 1; i < N; i += 2)
        if (!(blocks[i] = malloc(120))) return 11;
    step("refill");
    for (int i = 0; i < N; i++) free(blocks[i]);
    void **arr = 0;
    size_t cap = 0;
    for (size_t i = 0; i < N; i++) {
        if (i == cap) {
            cap = cap ? 2 * cap : 1;
            void **a = realloc(arr, cap * sizeof *arr);
            if (!a) return 12;
            arr = a;
        }
        arr[i] = one;
    }
    step("realloc");
    free(arr);
    free(one);
    return 0;
}
