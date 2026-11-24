/* Spec §21.1 Guarantees ("With CET, the shadow stack is unwound to match (via incssp)") and
 * §21.2, D-21.5: with the user shadow stack enabled, every jump and context switch keeps the
 * shadow stack matching the stack, so no later `ret` raises a control-protection fault (SIGSEGV,
 * exit by signal). main enables it for this thread (arch_prctl(ARCH_SHSTK_ENABLE), inline, so
 * main itself never returns: no frame older than the enable has an entry), then runs: longjmp
 * out of nested frames, siglongjmp out of a signal handler, setcontext to a getcontext point
 * from a deeper frame, setcontext from a signal handler to the kernel's ucontext (D-21.7),
 * swapcontext between main and a makecontext context (eight arguments, on
 * its own shadow stack) and back, a context function returning to uc_link, makecontext on the
 * same stack memory again (the earlier shadow stack is unmapped, D-21.7), vfork returning in
 * the parent after the child's calls, posix_spawn, and threads (their own shadow stacks: create,
 * join, pthread_exit through cleanup handlers, cancellation). Each step returns through the
 * frames it went through, which is what the hardware checks.
 *
 * Only a CPU and kernel with user shadow stacks (Intel CET or AMD's, Linux 6.6+ with
 * CONFIG_X86_USER_SHADOW_STACK) can run it: elsewhere the enable fails and the program reports
 * SKIPPED and exits 0. */
#define _GNU_SOURCE
#include <pthread.h>
#include <setjmp.h>
#include <signal.h>
#include <spawn.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/wait.h>
#include <ucontext.h>
#include <unistd.h>

#define ARCH_SHSTK_ENABLE 0x5001
#define ARCH_SHSTK_SHSTK 1

extern char **environ;

static unsigned long ssp(void)
{
    unsigned long v = 0;
    __asm__ volatile("rdsspq %0" : "+r"(v));
    return v;
}

static int failures;
#define CHECK(c) do { if (!(c)) { printf("FAIL line %d: %s\n", __LINE__, #c); failures++; } } while (0)

/* longjmp out of n nested frames. */
static jmp_buf jb;
static __attribute__((noinline)) int deep(int n)
{
    if (n == 0)
        longjmp(jb, 7);
    return deep(n - 1) + 1;
}

/* siglongjmp out of a handler (the kernel's signal frame is on the shadow stack too). */
static sigjmp_buf sjb;
static void on_usr1(int sig)
{
    (void)sig;
    siglongjmp(sjb, 1);
}

/* Contexts. */
static ucontext_t main_uc, co_uc, back_uc, ret_uc;
static char co_stack[256 * 1024], ret_stack[64 * 1024];
static long sum;
static int pings, returned;

static void co(int a, int b, int c, int d, int e, int f, int g, int h)
{
    sum = a + 2L * b + 3L * c + 4L * d + 5L * e + 6L * f + 7L * g + 8L * h;
    for (int i = 0; i < 3; i++) {
        pings++;
        swapcontext(&co_uc, &main_uc);
    }
}

static __attribute__((noinline)) void returns(void)
{
    returned++;
}

/* The kB of shadow-stack mappings ("ss" in an smaps VmFlags line), -1 if unreadable. */
static long shadow_kb(void)
{
    FILE *f = fopen("/proc/self/smaps", "r");
    if (!f)
        return -1;
    char line[512];
    long size = 0, total = 0;
    while (fgets(line, sizeof line, f)) {
        if (sscanf(line, "Size: %ld kB", &size) == 1)
            continue;
        if (strncmp(line, "VmFlags:", 8) == 0 && strstr(line, " ss "))
            total += size;
    }
    fclose(f);
    return total;
}

static __attribute__((noinline)) void set_from_deeper(int n, ucontext_t *uc)
{
    if (n == 0)
        setcontext(uc);
    set_from_deeper(n - 1, uc);
}

/* setcontext on the kernel's ucontext (a handler's third argument) from frames inside the
 * handler: the interrupted code resumes, and its frames return (D-21.7). */
static volatile int handled;
static __attribute__((noinline)) void set_kernel_uc(int n, void *uc)
{
    if (n == 0)
        setcontext((ucontext_t *)uc);
    set_kernel_uc(n - 1, uc);
}

static void on_usr2(int sig, siginfo_t *si, void *uc)
{
    (void)sig;
    (void)si;
    handled++;
    set_kernel_uc(5, uc);
}

static __attribute__((noinline)) int raise_deep(int n)
{
    if (n == 0)
        return raise(SIGUSR2);
    return raise_deep(n - 1) + 1;
}

static __attribute__((noinline)) pid_t vfork_and_return(void)
{
    pid_t p = vfork();
    if (p == 0) {
        /* The child's calls overwrite vfork's shadow-stack entry. */
        getpid();
        _exit(3);
    }
    return p;
}

/* Threads. */
static int cleaned;
static void cleanup(void *p)
{
    (void)p;
    cleaned++;
}

static __attribute__((noinline)) void exit_deep(int n)
{
    if (n == 0)
        pthread_exit((void *)5);
    exit_deep(n - 1);
}

static void *exits_through_cleanups(void *arg)
{
    (void)arg;
    pthread_cleanup_push(cleanup, NULL);
    exit_deep(5);
    pthread_cleanup_pop(0);
    return NULL;
}

static void *waits(void *arg)
{
    (void)arg;
    pthread_cleanup_push(cleanup, NULL);
    for (;;)
        pause();
    pthread_cleanup_pop(0);
    return NULL;
}

static void *plain(void *arg)
{
    return (void *)(ssp() != 0 ? arg : NULL);
}

int main(void)
{
    long r;
    __asm__ volatile("syscall" : "=a"(r) : "a"(158L), "D"((long)ARCH_SHSTK_ENABLE), "S"((long)ARCH_SHSTK_SHSTK) : "rcx", "r11", "memory");
    if (r != 0 || ssp() == 0) {
        printf("SKIPPED: no user shadow stack here (arch_prctl(ARCH_SHSTK_ENABLE) = %ld)\n", r);
        return 0;
    }
    /* From here on main must not return (exit() below). */

    /* longjmp out of 1 and 300 frames (more than one incssp step of 255). */
    int v = setjmp(jb);
    if (v == 0)
        deep(1);
    CHECK(v == 7);
    static volatile int far;
    v = setjmp(jb);
    if (v == 0)
        deep(300);
    else
        far = 1;
    CHECK(far == 1);

    /* siglongjmp out of a signal handler. */
    signal(SIGUSR1, on_usr1);
    volatile int jumped = 0;
    if (sigsetjmp(sjb, 1) == 0)
        raise(SIGUSR1);
    else
        jumped = 1;
    CHECK(jumped == 1);

    /* setcontext to a getcontext point from 20 frames deeper. */
    volatile int resumed = 0;
    getcontext(&back_uc);
    if (!resumed) {
        resumed = 1;
        set_from_deeper(20, &back_uc);
    }
    CHECK(resumed == 1);

    /* setcontext from a handler to the ucontext it was given, from 5 frames inside it; the
     * signal arrived 10 frames down in main. */
    struct sigaction sa = {0};
    sa.sa_sigaction = on_usr2;
    sa.sa_flags = SA_SIGINFO;
    CHECK(sigaction(SIGUSR2, &sa, NULL) == 0);
    CHECK(raise_deep(10) == 10);
    CHECK(handled == 1);

    /* swapcontext with a makecontext context and back, three times; then it ends in main_uc. */
    getcontext(&co_uc);
    co_uc.uc_stack.ss_sp = co_stack;
    co_uc.uc_stack.ss_size = sizeof co_stack;
    co_uc.uc_link = &main_uc;
    makecontext(&co_uc, (void (*)(void))co, 8, 1, 2, 3, 4, 5, 6, 7, 8);
    for (int i = 0; i < 4; i++)
        swapcontext(&main_uc, &co_uc);
    CHECK(sum == 204);
    CHECK(pings == 3);

    /* A context function that returns, to uc_link (a swapcontext point in main). */
    getcontext(&ret_uc);
    ret_uc.uc_stack.ss_sp = ret_stack;
    ret_uc.uc_stack.ss_size = sizeof ret_stack;
    ret_uc.uc_link = &main_uc;
    makecontext(&ret_uc, returns, 0);
    swapcontext(&main_uc, &ret_uc);
    CHECK(returned == 1);

    /* makecontext on the same stack memory 64 more times, each context run to its return
     * (D-21.7 (5)): the shadow stack made for the earlier context on it is unmapped, so the
     * shadow-stack mappings stay at their size after the first of these rounds. */
    long kb_first = 0;
    for (int i = 0; i < 64; i++) {
        getcontext(&ret_uc);
        ret_uc.uc_stack.ss_sp = ret_stack;
        ret_uc.uc_stack.ss_size = sizeof ret_stack;
        ret_uc.uc_link = &main_uc;
        makecontext(&ret_uc, returns, 0);
        swapcontext(&main_uc, &ret_uc);
        if (i == 0)
            kb_first = shadow_kb();
    }
    long kb_last = shadow_kb();
    CHECK(returned == 65);
    CHECK(kb_first > 0 && kb_last == kb_first);
    if (kb_last != kb_first)
        printf("shadow-stack mappings: %ld kB after the first reuse, %ld kB after 64\n", kb_first, kb_last);

    /* vfork: the parent returns through the frame that called it. */
    int st = 0;
    pid_t p = vfork_and_return();
    CHECK(p > 0 && waitpid(p, &st, 0) == p && WIFEXITED(st) && WEXITSTATUS(st) == 3);

    /* posix_spawn (its child runs on the parent's stack until it execs). */
    char *argv[] = {"true", NULL};
    CHECK(posix_spawnp(&p, "true", NULL, NULL, argv, environ) == 0 && waitpid(p, &st, 0) == p && WIFEXITED(st) && WEXITSTATUS(st) == 0);

    /* Threads: each has its own shadow stack. */
    pthread_t t;
    void *res = NULL;
    CHECK(pthread_create(&t, NULL, plain, (void *)1) == 0 && pthread_join(t, &res) == 0 && res == (void *)1);
    CHECK(pthread_create(&t, NULL, exits_through_cleanups, NULL) == 0 && pthread_join(t, &res) == 0 && res == (void *)5);
    CHECK(pthread_create(&t, NULL, waits, NULL) == 0);
    CHECK(pthread_cancel(t) == 0 && pthread_join(t, &res) == 0 && res == PTHREAD_CANCELED);
    CHECK(cleaned == 2);

    printf("shadow stack on: %s\n", failures ? "FAILED" : "all steps passed");
    fflush(stdout);
    exit(failures ? 1 : 0);
}
