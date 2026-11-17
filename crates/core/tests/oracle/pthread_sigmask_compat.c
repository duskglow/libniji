/* pthread_sigmask's old version against its default version (spec s0_6_c0_6_0, s20_3_c_1;
 * spec/behaviour/pthread-sigmask-compat.md).
 *
 * Binds pthread_sigmask@GLIBC_2.2.5 (the version Debian trixie's btscanner, pbzip2 and wavemon bind)
 * and the default pthread_sigmask@GLIBC_2.32 with `.symver`, and calls both on the same cases: every
 * `how` (the three valid ones, 3 and -1), a null and six non-null sets (one with the signals 32 and 33
 * the kernel numbers below SIGRTMIN, SIGKILL and SIGSTOP, all 64 bits, the real-time range), a null
 * and a non-null old set, from two starting masks (one with 32 and 33 blocked). The starting mask is
 * set, and the resulting one read, by the raw rt_sigprocmask system call, so the C library under
 * test never filters what is observed. Per call it prints the return value, errno (preset to 999),
 * the first 8 bytes of the old set (preset to 0xa5 bytes), how many of the other 120 bytes changed,
 * and the kernel mask afterwards; then whether all of it equals the default version's.
 * The reference and libniji must print the same text (crates/core/tests/crt_products.rs).
 *
 * Build: cc -O1 pthread_sigmask_compat.c */
#define _GNU_SOURCE
#include <errno.h>
#include <pthread.h>
#include <signal.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <sys/syscall.h>
#include <unistd.h>

int pthread_sigmask_old(int, const sigset_t *, sigset_t *);
__asm__(".symver pthread_sigmask_old,pthread_sigmask@GLIBC_2.2.5");
int pthread_sigmask_new(int, const sigset_t *, sigset_t *);
__asm__(".symver pthread_sigmask_new,pthread_sigmask@GLIBC_2.32");

typedef int (*mask_fn)(int, const sigset_t *, sigset_t *);

static uint64_t bit(int sig)
{
    return 1ull << (sig - 1);
}

static void kset(uint64_t m)
{
    syscall(SYS_rt_sigprocmask, SIG_SETMASK, &m, NULL, sizeof m);
}

static uint64_t kget(void)
{
    uint64_t m = 0;
    syscall(SYS_rt_sigprocmask, SIG_BLOCK, NULL, &m, sizeof m);
    return m;
}

struct obs {
    int ret, err, tail;
    uint64_t head, after;
};

static void call(mask_fn f, uint64_t start, int how, const sigset_t *set, int want_old, struct obs *o)
{
    sigset_t old;
    memset(&old, 0xa5, sizeof old);
    kset(start);
    errno = 999;
    o->ret = f(how, set, want_old ? &old : NULL);
    o->err = errno;
    o->after = kget();
    kset(0);
    memcpy(&o->head, &old, sizeof o->head);
    o->tail = 0;
    for (size_t i = sizeof o->head; i < sizeof old; i++)
        o->tail += ((unsigned char *)&old)[i] != 0xa5;
}

int main(void)
{
    static const int hows[] = {SIG_BLOCK, SIG_UNBLOCK, SIG_SETMASK, 3, -1};
    const uint64_t start[2] = {bit(SIGINT), bit(SIGINT) | bit(32) | bit(33)};
    uint64_t masks[6] = {
        bit(SIGUSR1),
        bit(SIGUSR1) | bit(32) | bit(33),
        bit(SIGKILL) | bit(SIGSTOP) | bit(SIGUSR2),
        ~0ull,
        bit(34) | bit(50) | bit(64),
        bit(SIGINT) | bit(SIGTERM),
    };
    sigset_t sets[6];
    int cases = 0, diffs = 0;
    for (int k = 0; k < 6; k++) {
        memset(&sets[k], 0, sizeof sets[k]);
        memcpy(&sets[k], &masks[k], sizeof masks[k]);
    }
    for (int s = 0; s < 2; s++)
        for (int h = 0; h < 5; h++)
            for (int k = -1; k < 6; k++)
                for (int w = 0; w < 2; w++) {
                    struct obs a, b;
                    const sigset_t *set = k < 0 ? NULL : &sets[k];
                    call(pthread_sigmask_old, start[s], hows[h], set, w, &a);
                    call(pthread_sigmask_new, start[s], hows[h], set, w, &b);
                    int same = a.ret == b.ret && a.err == b.err && a.head == b.head && a.tail == b.tail && a.after == b.after;
                    cases++;
                    diffs += !same;
                    printf("pthread_sigmask@GLIBC_2.2.5(start %016llx, how %d, set %016llx%s, %s) = %d errno %d old %016llx +%d after %016llx: %s\n",
                           (unsigned long long)start[s], hows[h], (unsigned long long)(k < 0 ? 0 : masks[k]), k < 0 ? " (null)" : "",
                           w ? "old" : "null", a.ret, a.err, (unsigned long long)a.head, a.tail, (unsigned long long)a.after,
                           same ? "same as default" : "DIFFERS from default");
                    if (!same)
                        printf("  default: %d errno %d old %016llx +%d after %016llx\n", b.ret, b.err, (unsigned long long)b.head,
                               b.tail, (unsigned long long)b.after);
                }
    printf("pthread_sigmask@GLIBC_2.2.5: %d cases, %d differ from the default\n", cases, diffs);
    return 0;
}
