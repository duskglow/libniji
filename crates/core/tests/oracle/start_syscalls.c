/* start_syscalls PROG [NAME=VALUE ...]: run PROG (no arguments; its environment exactly the
 * NAME=VALUE words, so this program itself runs under the caller's own environment) under ptrace
 * and print one line per system call it enters, "nr arg0 arg1 arg3" in decimal (x86-64 registers
 * rax, rdi, rsi, r10), from the exec to the exit. Exit status: PROG's, or 120-125 on a tracing
 * failure. DYN-START-QUICK: hot_text_products reads the list a dynamic start makes. */
#define _GNU_SOURCE
#include <signal.h>
#include <stdio.h>
#include <sys/ptrace.h>
#include <sys/user.h>
#include <sys/wait.h>
#include <unistd.h>

int main(int argc, char **argv)
{
    if (argc < 2)
        return 125;
    pid_t pid = fork();
    if (pid < 0)
        return 124;
    if (pid == 0) {
        if (ptrace(PTRACE_TRACEME, 0, 0, 0) != 0)
            _exit(123);
        char *args[] = { argv[1], NULL };
        execve(argv[1], args, argv + 2);
        _exit(122);
    }
    int st;
    /* The stop at the exec. */
    if (waitpid(pid, &st, 0) != pid || !WIFSTOPPED(st))
        return 121;
    ptrace(PTRACE_SETOPTIONS, pid, 0, (void *)(long)(PTRACE_O_TRACESYSGOOD | PTRACE_O_EXITKILL));
    int entering = 1;
    for (;;) {
        if (ptrace(PTRACE_SYSCALL, pid, 0, 0) != 0)
            return 120;
        if (waitpid(pid, &st, 0) != pid)
            return 120;
        if (WIFEXITED(st))
            return WEXITSTATUS(st);
        if (WIFSIGNALED(st))
            return 120;
        if (WSTOPSIG(st) != (SIGTRAP | 0x80))
            continue;
        if (entering) {
            struct user_regs_struct r;
            if (ptrace(PTRACE_GETREGS, pid, 0, &r) != 0)
                return 120;
            printf("%llu %llu %llu %llu\n", r.orig_rax, r.rdi, r.rsi, r.r10);
            fflush(stdout);
        }
        entering = !entering;
    }
}
