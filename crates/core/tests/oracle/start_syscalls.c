/* start_syscalls PROG [ARG ...] [NAME=VALUE ...]: run PROG (its arguments the words before the first
 * one holding '='; its environment exactly the NAME=VALUE words, so this program itself runs under
 * the caller's own environment) under ptrace
 * and print one line per system call it enters, "nr arg0 arg1 arg3" in decimal (x86-64 registers
 * rax, rdi, rsi, r10), from the exec to the exit. Exit status: PROG's, or 120-125 on a tracing
 * failure. DYN-START-QUICK: hot_text_products reads the list a dynamic start makes. */
#define _GNU_SOURCE
#include <signal.h>
#include <stdio.h>
#include <string.h>
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
        /* PROG's arguments: the words before the first NAME=VALUE one. */
        char *args[argc];
        int n = 0;
        while (1 + n < argc && (n == 0 || !strchr(argv[1 + n], '='))) {
            args[n] = argv[1 + n];
            n++;
        }
        args[n] = NULL;
        execve(argv[1], args, argv + 1 + n);
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
