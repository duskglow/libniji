/* STARTUP-STRUCTURE-REVIEW: count one program from exec to exit, RUNS times, with perf_event_open counters
 * (enable_on_exec) on a forked child: user instructions, kernel instructions, page faults.
 * Usage: stcount RUNS PROG [ARG...]. PROG gets this process's environment unchanged, so run it as
 * `env -i NAME=VALUE ... stcount 101 PROG` to fix the environment exactly.
 * Prints: user_median U user_min A user_max B kernel_median K faults_median F faults_min C faults_max D */
#define _GNU_SOURCE
#include <linux/perf_event.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/syscall.h>
#include <sys/wait.h>
#include <unistd.h>

extern char **environ;

static int open_counter(pid_t pid, unsigned type, unsigned long long config, int no_user, int no_kernel) {
    struct perf_event_attr a;
    memset(&a, 0, sizeof a);
    a.size = sizeof a;
    a.type = type;
    a.config = config;
    a.disabled = 1;
    a.enable_on_exec = 1;
    a.exclude_user = no_user;
    a.exclude_kernel = no_kernel;
    a.exclude_hv = 1;
    return (int)syscall(SYS_perf_event_open, &a, pid, -1, -1, 0);
}

static int by_value(const void *x, const void *y) {
    long long a = *(const long long *)x, b = *(const long long *)y;
    return (a > b) - (a < b);
}

static long long read_counter(int fd) {
    long long v = -1;
    if (read(fd, &v, sizeof v) != (ssize_t)sizeof v) return -1;
    close(fd);
    return v;
}

int main(int argc, char **argv) {
    if (argc < 3) {
        fprintf(stderr, "usage: stcount RUNS PROG [ARG...]\n");
        return 2;
    }
    int runs = atoi(argv[1]);
    /* STARTUP-REPROFILE: optional `-L DIR`: the child gets LD_LIBRARY_PATH=DIR as its whole environment
       (this harness keeps the host library); without it the child gets this process environment. */
    static char ldlp[4096];
    char *cenv[2] = { ldlp, 0 };
    char **use_env = environ;
    if (argc > 4 && strcmp(argv[2], "-L") == 0) {
        snprintf(ldlp, sizeof ldlp, "LD_LIBRARY_PATH=%s", argv[3]);
        use_env = cenv;
        argv += 2;
        argc -= 2;
    }
    if (runs < 1 || runs > 100001) return 2;
    long long *u = calloc(runs, sizeof *u), *k = calloc(runs, sizeof *k), *f = calloc(runs, sizeof *f);
    if (!u || !k || !f) return 3;
    for (int i = 0; i < runs; i++) {
        int go[2];
        if (pipe(go) != 0) return 3;
        pid_t pid = fork();
        if (pid < 0) return 3;
        if (pid == 0) {
            char c;
            close(go[1]);
            if (read(go[0], &c, 1) != 1) _exit(126);
            close(go[0]);
            execve(argv[2], argv + 2, use_env);
            _exit(127);
        }
        close(go[0]);
        int fu = open_counter(pid, PERF_TYPE_HARDWARE, PERF_COUNT_HW_INSTRUCTIONS, 0, 1);
        int fk = open_counter(pid, PERF_TYPE_HARDWARE, PERF_COUNT_HW_INSTRUCTIONS, 1, 0);
        int ff = open_counter(pid, PERF_TYPE_SOFTWARE, PERF_COUNT_SW_PAGE_FAULTS, 0, 0);
        if (fu < 0 || fk < 0 || ff < 0) {
            perror("perf_event_open");
            return 4;
        }
        if (write(go[1], "x", 1) != 1) return 3;
        close(go[1]);
        int st;
        if (waitpid(pid, &st, 0) != pid) return 3;
        if (!WIFEXITED(st) || WEXITSTATUS(st) != 0) {
            fprintf(stderr, "%s: status %#x\n", argv[2], st);
            return 5;
        }
        u[i] = read_counter(fu);
        k[i] = read_counter(fk);
        f[i] = read_counter(ff);
        if (u[i] < 0 || k[i] < 0 || f[i] < 0) return 6;
    }
    qsort(u, runs, sizeof *u, by_value);
    qsort(k, runs, sizeof *k, by_value);
    qsort(f, runs, sizeof *f, by_value);
    printf("user_median %lld user_min %lld user_max %lld kernel_median %lld faults_median %lld faults_min %lld "
           "faults_max %lld\n",
           u[runs / 2], u[0], u[runs - 1], k[runs / 2], f[runs / 2], f[0], f[runs - 1]);
    return 0;
}
