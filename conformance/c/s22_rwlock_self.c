/* Spec §22.5 (clause s22_5_c_2): what a read-write lock call returns when the calling thread
 * already holds the lock, by every lock configuration (static initialiser, default attributes,
 * each kind setkind_np accepts, private and process-shared) and every non-blocking call form.
 *   - The try forms report a lock the caller holds for writing as busy (EBUSY), as for any lock
 *     they cannot take (POSIX; callers such as Berkeley DB's latches read EBUSY as "held" and
 *     treat EDEADLK as fatal: issue #15).
 *   - The blocking, timed and clock forms report it as EDEADLK.
 *   - A timed or clock form validates its deadline first: tv_nsec outside [0, 1e9) or an unknown
 *     clock is EINVAL whatever the lock state, even on a free lock.
 *   - Read-held by the caller: a further read lock is taken (recursive read), a write lock is
 *     busy (EBUSY) or times out (ETIMEDOUT); the blocking wrlock, which would wait forever, is
 *     not called.
 * Every cell was settled by a black-box probe of the reference library (2026-11-17), and this
 * program passes against it. Failing cells are listed on stderr; the exit status is their count
 * (at most 100). A call that never returns is a failure too: a hang guard (alarm) names the cell
 * and exits 101. */
#define _GNU_SOURCE
#include <errno.h>
#include <pthread.h>
#include <stdio.h>
#include <string.h>
#include <sys/mman.h>
#include <signal.h>
#include <time.h>
#include <unistd.h>

/* The cell being called, for the hang guard. */
static char current[160];

static void hang_guard(int sig)
{
    (void)sig;
    static const char head[] = "hang guard: did not return: ";
    write(2, head, sizeof head - 1);
    write(2, current, strlen(current));
    write(2, "\n", 1);
    _exit(101);
}

enum { NONE, RD, WR };
static const char *const held_name[] = {"none", "read", "write"};

enum op {
    RDLOCK, WRLOCK, TRYRD, TRYWR, TIMEDRD, TIMEDWR, CLOCKRD, CLOCKWR,
    TIMEDRD_NSNEG, TIMEDWR_NSNEG, TIMEDRD_NSBIG, TIMEDWR_NSBIG,
    CLOCKRD_NSNEG, CLOCKWR_NSNEG, CLOCKRD_BADCLK, CLOCKWR_BADCLK, NOPS
};
static const char *const op_name[] = {
    "rdlock", "wrlock", "tryrdlock", "trywrlock", "timedrdlock(past)", "timedwrlock(past)",
    "clockrdlock(MONOTONIC, past)", "clockwrlock(MONOTONIC, past)",
    "timedrdlock(nsec -1)", "timedwrlock(nsec -1)", "timedrdlock(nsec 1e9)", "timedwrlock(nsec 1e9)",
    "clockrdlock(nsec -1)", "clockwrlock(nsec -1)", "clockrdlock(clock 99)", "clockwrlock(clock 99)",
};

static int call(pthread_rwlock_t *l, enum op op)
{
    static const struct timespec past = {0, 0}, nsneg = {0, -1}, nsbig = {0, 1000000000};
    switch (op) {
    case RDLOCK: return pthread_rwlock_rdlock(l);
    case WRLOCK: return pthread_rwlock_wrlock(l);
    case TRYRD: return pthread_rwlock_tryrdlock(l);
    case TRYWR: return pthread_rwlock_trywrlock(l);
    case TIMEDRD: return pthread_rwlock_timedrdlock(l, &past);
    case TIMEDWR: return pthread_rwlock_timedwrlock(l, &past);
    case CLOCKRD: return pthread_rwlock_clockrdlock(l, CLOCK_MONOTONIC, &past);
    case CLOCKWR: return pthread_rwlock_clockwrlock(l, CLOCK_MONOTONIC, &past);
    case TIMEDRD_NSNEG: return pthread_rwlock_timedrdlock(l, &nsneg);
    case TIMEDWR_NSNEG: return pthread_rwlock_timedwrlock(l, &nsneg);
    case TIMEDRD_NSBIG: return pthread_rwlock_timedrdlock(l, &nsbig);
    case TIMEDWR_NSBIG: return pthread_rwlock_timedwrlock(l, &nsbig);
    case CLOCKRD_NSNEG: return pthread_rwlock_clockrdlock(l, CLOCK_MONOTONIC, &nsneg);
    case CLOCKWR_NSNEG: return pthread_rwlock_clockwrlock(l, CLOCK_MONOTONIC, &nsneg);
    case CLOCKRD_BADCLK: return pthread_rwlock_clockrdlock(l, 99, &past);
    case CLOCKWR_BADCLK: return pthread_rwlock_clockwrlock(l, 99, &past);
    case NOPS: break;
    }
    return -1;
}

/* The expected result; -1 for a cell that would block forever (not called). */
static int expected(int held, enum op op)
{
    int is_write = op == WRLOCK || op == TRYWR || op == TIMEDWR || op == CLOCKWR;
    if (op >= TIMEDRD_NSNEG)
        return EINVAL;
    switch (held) {
    case NONE: return 0;
    case RD:
        if (!is_write) return 0;
        if (op == WRLOCK) return -1;
        return op == TRYWR ? EBUSY : ETIMEDOUT;
    default:
        return op == TRYRD || op == TRYWR ? EBUSY : EDEADLK;
    }
}

int main(void)
{
    /* Configurations: static initialiser, NULL attributes, then kinds 0..2 private and shared. */
    enum { NCONF = 8 };
    /* Hang guard only: every call here returns at once (the past deadlines included). */
    signal(SIGALRM, hang_guard);
    alarm(60);
    pthread_rwlock_t *locks = mmap(NULL, sizeof(pthread_rwlock_t) * NCONF * 3 * NOPS, PROT_READ | PROT_WRITE,
                                   MAP_SHARED | MAP_ANONYMOUS, -1, 0);
    if (locks == MAP_FAILED) {
        perror("mmap");
        return 100;
    }
    int bad = 0;
    pthread_rwlock_t *l = locks;
    for (int conf = 0; conf < NCONF; conf++)
        for (int held = NONE; held <= WR; held++)
            for (int op = 0; op < NOPS; op++, l++) {
                int kind = conf < 2 ? -1 : (conf - 2) / 2, shared = conf >= 2 && conf % 2;
                if (conf == 0) {
                    pthread_rwlock_t init = PTHREAD_RWLOCK_INITIALIZER;
                    *l = init;
                } else if (conf == 1) {
                    if (pthread_rwlock_init(l, NULL) != 0) return 100;
                } else {
                    pthread_rwlockattr_t a;
                    if (pthread_rwlockattr_init(&a) != 0 || pthread_rwlockattr_setkind_np(&a, kind) != 0
                        || pthread_rwlockattr_setpshared(&a, shared ? PTHREAD_PROCESS_SHARED : PTHREAD_PROCESS_PRIVATE) != 0
                        || pthread_rwlock_init(l, &a) != 0)
                        return 100;
                }
                int want = expected(held, op);
                if (want < 0)
                    continue;
                if ((held == RD && pthread_rwlock_rdlock(l) != 0) || (held == WR && pthread_rwlock_wrlock(l) != 0))
                    return 100;
                snprintf(current, sizeof current, "config %d (kind %d, %s), caller holds %s: %s", conf, kind,
                         shared ? "shared" : "private", held_name[held], op_name[op]);
                int got = call(l, op);
                if (got != want) {
                    fprintf(stderr, "config %d (kind %d, %s), caller holds %s: %s = %d (%s), expected %d (%s)\n",
                            conf, kind, shared ? "shared" : "private", held_name[held], op_name[op], got,
                            strerror(got), want, strerror(want));
                    bad++;
                }
            }
    /* Issue #15's reproducer, as reported. */
    pthread_rwlock_t r;
    pthread_rwlock_init(&r, NULL);
    pthread_rwlock_wrlock(&r);
    int e = pthread_rwlock_tryrdlock(&r);
    char text[64];
    snprintf(text, sizeof text, "%d %s", e, strerror(e));
    if (strcmp(text, "16 Device or resource busy") != 0) {
        fprintf(stderr, "reproducer: %s, expected 16 Device or resource busy\n", text);
        bad++;
    }
    return bad > 100 ? 100 : bad;
}
