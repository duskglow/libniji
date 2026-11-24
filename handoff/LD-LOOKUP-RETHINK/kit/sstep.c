/* LD-LOOKUP-RETHINK: exact user-instruction trace of one run of a libniji program, by ptrace single-step.
 * usage: sstep OUT PROG [ARG...]   (PROG gets this process's environment; run under `setarch -R`)
 * Writes OUT.seq: (rip, rsp) uint64 pairs, one per user instruction executed, from exec to exit;
 * OUT.maps: /proc/<pid>/maps at the exit stop. Prints "steps N". Used only on libniji's own loader. */
#define _GNU_SOURCE
#include <signal.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ptrace.h>
#include <sys/user.h>
#include <sys/wait.h>
#include <unistd.h>

extern char **environ;

static void copy_maps(pid_t pid, const char *out) {
    char src[64], dst[4096], line[4096];
    snprintf(src, sizeof src, "/proc/%d/maps", (int)pid);
    snprintf(dst, sizeof dst, "%s.maps", out);
    FILE *in = fopen(src, "r"), *o = fopen(dst, "w");
    if (!in || !o) { perror("maps"); exit(2); }
    while (fgets(line, sizeof line, in)) fputs(line, o);
    fclose(in);
    fclose(o);
}

int main(int argc, char **argv) {
    if (argc < 3) { fprintf(stderr, "usage: sstep OUT [-L LIBPATH] PROG [ARG...]\n"); return 2; }
    /* -L: the child's whole environment is LD_LIBRARY_PATH=LIBPATH (as the kit's stc), this process keeps its own */
    static char ldlp[4096];
    char *child_env[2] = {0, 0};
    char **use_env = environ;
    if (argc > 4 && strcmp(argv[2], "-L") == 0) {
        snprintf(ldlp, sizeof ldlp, "LD_LIBRARY_PATH=%s", argv[3]);
        child_env[0] = ldlp;
        use_env = child_env;
        argv[3] = argv[1];
        argv += 2;
    }
    char path[4096];
    snprintf(path, sizeof path, "%s.seq", argv[1]);
    FILE *seq = fopen(path, "wb");
    if (!seq) { perror(path); return 2; }
    pid_t pid = fork();
    if (pid < 0) { perror("fork"); return 2; }
    if (pid == 0) {
        if (ptrace(PTRACE_TRACEME, 0, 0, 0) != 0) _exit(126);
        execve(argv[2], argv + 2, use_env);
        _exit(127);
    }
    int st;
    if (waitpid(pid, &st, 0) != pid || !WIFSTOPPED(st)) { fprintf(stderr, "no exec stop\n"); return 2; }
    if (ptrace(PTRACE_SETOPTIONS, pid, 0, PTRACE_O_TRACEEXIT | PTRACE_O_EXITKILL) != 0) { perror("setoptions"); return 2; }
    static uint64_t buf[2 * 65536];
    size_t k = 0;
    unsigned long long n = 0;
    int sig = 0, maps_done = 0;
    for (;;) {
        struct user_regs_struct r;
        if (ptrace(PTRACE_GETREGS, pid, 0, &r) != 0) { perror("getregs"); return 2; }
        buf[k++] = r.rip;
        buf[k++] = r.rsp;
        n++;
        if (k == sizeof buf / sizeof buf[0]) { fwrite(buf, sizeof buf[0], k, seq); k = 0; }
        if (ptrace(PTRACE_SINGLESTEP, pid, 0, (void *)(long)sig) != 0) { perror("singlestep"); return 2; }
        sig = 0;
        if (waitpid(pid, &st, 0) != pid) { perror("waitpid"); return 2; }
        if (WIFEXITED(st) || WIFSIGNALED(st)) break;
        if (WIFSTOPPED(st) && (st >> 8) == (SIGTRAP | (PTRACE_EVENT_EXIT << 8))) {
            /* exit stop: the last instruction (exit_group) is counted; record the maps, let it go */
            copy_maps(pid, argv[1]);
            maps_done = 1;
            ptrace(PTRACE_CONT, pid, 0, 0);
            waitpid(pid, &st, 0);
            break;
        }
        if (WIFSTOPPED(st) && WSTOPSIG(st) != SIGTRAP) sig = WSTOPSIG(st);
    }
    fwrite(buf, sizeof buf[0], k, seq);
    fclose(seq);
    printf("steps %llu maps %d status %#x\n", n, maps_done, st);
    return maps_done ? 0 : 1;
}
