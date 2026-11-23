/* The heap's address space for a directory walk over 200000 entries (GitHub issue #21's real
 * workload, findutils' find over one directory of 200000 files; HEAP-FIND-DEPTH-NEED): 200000
 * blocks of 264 bytes (find's entry size range, 257..272, HEAP-SPILL-COLD), all held at once, so
 * the heap maps several segments of one shard in turn. Prints "start <VmSize KiB>" and
 * "held <VmSize KiB>" to stdout (no stdio, so only the walk allocates). With an argument N,
 * RLIMIT_AS is first set to the VmSize at start plus N KiB and every allocation must still
 * succeed (exit 10: one failed); "-" sets no limit. */
#define _GNU_SOURCE
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

static void step(const char *name, long kib) {
    char line[64];
    int len = snprintf(line, sizeof line, "%s %ld\n", name, kib);
    if (write(1, line, (size_t)len) != len) _exit(3);
}

int main(int argc, char **argv) {
    long start = vmsize();
    if (argc > 1 && strcmp(argv[1], "-") != 0) {
        rlim_t lim = ((rlim_t)start + (rlim_t)strtol(argv[1], 0, 10)) * 1024;
        struct rlimit rl = {lim, lim};
        if (setrlimit(RLIMIT_AS, &rl) != 0) return 4;
    }
    step("start", start);
    for (int i = 0; i < N; i++) {
        if (!(blocks[i] = malloc(264))) return 10;
        memset(blocks[i], 1, 8);
    }
    step("held", vmsize());
    for (int i = 0; i < N; i++) free(blocks[i]);
    return 0;
}
