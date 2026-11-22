/* User-mode instructions of a program from exec to exit (DYN-START-PARITY), built with the host's C
 * compiler and library: it is the measuring harness, never the measured program.
 *
 *   exec_instr RUNS [-e NAME=VALUE]... PROGRAM [ARGS...]
 *
 * Runs PROGRAM RUNS times, each in a forked child that waits on a pipe until a counter is open on it
 * (perf_event_open(2): hardware instructions, exclude_kernel, exclude_hv, enable_on_exec), so the
 * count starts at the exec and nothing of this harness is in it. The program's environment is the
 * `-e` options only.
 * Prints `instr MIN MEDIAN` on stdout. Exit 0; 2 when no counter can be opened here (the message
 * names why); 3 when a run fails (the program exits other than 0 or the counter cannot be read). */
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <linux/perf_event.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ioctl.h>
#include <sys/syscall.h>
#include <sys/wait.h>
#include <unistd.h>

static int cmp(const void *a, const void *b) {
    uint64_t x = *(const uint64_t *)a, y = *(const uint64_t *)b;
    return x < y ? -1 : x > y;
}

int main(int argc, char **argv) {
    if (argc < 3)
        return 64;
    long runs = strtol(argv[1], NULL, 10);
    if (runs < 1 || runs > 10000)
        return 64;
    /* `-e NAME=VALUE` options: the program's whole environment (this harness runs with its own, which
     * may not suit it: a library path for the measured program's library is not this harness's). */
    char *env[64];
    int nenv = 0, first = 2;
    while (first + 1 < argc && strcmp(argv[first], "-e") == 0 && nenv < 63) {
        env[nenv++] = argv[first + 1];
        first += 2;
    }
    env[nenv] = NULL;
    if (first >= argc)
        return 64;
    uint64_t *seen = calloc((size_t)runs, sizeof *seen);
    if (seen == NULL)
        return 3;
    for (long i = 0; i < runs; i++) {
        int go[2];
        if (pipe2(go, O_CLOEXEC) != 0)
            return 3;
        pid_t p = fork();
        if (p < 0)
            return 3;
        if (p == 0) {
            char c;
            close(go[1]);
            if (read(go[0], &c, 1) != 1)
                _exit(126);
            execve(argv[first], argv + first, env);
            _exit(127);
        }
        close(go[0]);
        struct perf_event_attr a;
        memset(&a, 0, sizeof a);
        a.type = PERF_TYPE_HARDWARE;
        a.size = sizeof a;
        a.config = PERF_COUNT_HW_INSTRUCTIONS;
        a.disabled = 1;
        a.enable_on_exec = 1;
        a.exclude_kernel = 1;
        a.exclude_hv = 1;
        int fd = (int)syscall(SYS_perf_event_open, &a, p, -1, -1, PERF_FLAG_FD_CLOEXEC);
        int err = errno;
        if (write(go[1], "g", 1) != 1)
            return 3;
        close(go[1]);
        int st;
        if (waitpid(p, &st, 0) != p)
            return 3;
        if (fd < 0) {
            fprintf(stderr, "no instruction counter: perf_event_open: %s\n", strerror(err));
            return 2;
        }
        if (!WIFEXITED(st) || WEXITSTATUS(st) != 0) {
            fprintf(stderr, "%s: run %ld ended with status %#x\n", argv[first], i, st);
            return 3;
        }
        if (read(fd, &seen[i], sizeof seen[i]) != (ssize_t)sizeof seen[i])
            return 3;
        close(fd);
    }
    qsort(seen, (size_t)runs, sizeof *seen, cmp);
    printf("instr %llu %llu\n", (unsigned long long)seen[0], (unsigned long long)seen[runs / 2]);
    return 0;
}
