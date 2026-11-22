/* Spec §22.4 (clause s22_4_c_5, timeouts; also §22.5 and §22.7 timed forms and
 * pthread_timedjoin_np/clockjoin_np): every timed wait given an absolute deadline already in the
 * past returns at once. The deadlines are a negative tv_sec ({-1, 0}, {LONG_MIN, 0}) and one far
 * in the past ({1, 0}), on CLOCK_REALTIME (the timed forms) and on each clock of the clock forms.
 *   - Contended (the object is held by another thread, the semaphore is 0, the thread to join
 *     still runs, any condition-variable wait): ETIMEDOUT (C11: thrd_timedout) without blocking;
 *     a priority-inheritance mutex given a negative tv_sec reports EINVAL (its kernel lock call
 *     rejects the deadline; the reference reports the same).
 *   - Uncontended (a free lock, a semaphore of 1, a thread that has ended): the call succeeds;
 *     a past deadline is no error when the call need not wait.
 * Every cell was settled by a black-box probe of the reference library (2026-11-22; run with
 * --probe, it prints the whole matrix, invalid tv_nsec beside a negative tv_sec included), and
 * this program passes against it. Failing cells are listed on stderr; the exit status is their
 * count (at most 100). Each cell runs in its own child; a call that does not return is ended by
 * a hang guard (alarm, 30 s; the calls return at once) and fails the cell as "blocks". */
#define _GNU_SOURCE
#include <errno.h>
#include <limits.h>
#include <pthread.h>
#include <sched.h>
#include <semaphore.h>
#include <signal.h>
#include <stdio.h>
#include <string.h>
#include <sys/wait.h>
#include <threads.h>
#include <time.h>
#include <unistd.h>

enum kind { MUTEX, MTX, COND, CND, RWRD, RWWR, RWWR_SELF, SEM, JOIN };
enum attr { A_DEFAULT, A_RECURSIVE, A_ERRORCHECK, A_ADAPTIVE, A_ROBUST, A_PI, A_PP, A_PSHARED, A_CONDMONO, A_WRITERPREF };
enum how { H_TIMED, H_MONO, H_REAL };

struct op {
    const char *name;
    enum kind kind;
    enum attr attr;
    enum how how;
};

static const struct op ops[] = {
    {"pthread_mutex_timedlock(default)", MUTEX, A_DEFAULT, H_TIMED},
    {"pthread_mutex_clocklock(default, MONOTONIC)", MUTEX, A_DEFAULT, H_MONO},
    {"pthread_mutex_clocklock(default, REALTIME)", MUTEX, A_DEFAULT, H_REAL},
    {"pthread_mutex_timedlock(recursive)", MUTEX, A_RECURSIVE, H_TIMED},
    {"pthread_mutex_timedlock(errorcheck)", MUTEX, A_ERRORCHECK, H_TIMED},
    {"pthread_mutex_timedlock(adaptive)", MUTEX, A_ADAPTIVE, H_TIMED},
    {"pthread_mutex_timedlock(robust)", MUTEX, A_ROBUST, H_TIMED},
    {"pthread_mutex_clocklock(robust, MONOTONIC)", MUTEX, A_ROBUST, H_MONO},
    {"pthread_mutex_timedlock(prio-inherit)", MUTEX, A_PI, H_TIMED},
    {"pthread_mutex_clocklock(prio-inherit, MONOTONIC)", MUTEX, A_PI, H_MONO},
    {"pthread_mutex_timedlock(prio-protect)", MUTEX, A_PP, H_TIMED},
    {"pthread_mutex_clocklock(prio-protect, MONOTONIC)", MUTEX, A_PP, H_MONO},
    {"pthread_mutex_timedlock(pshared)", MUTEX, A_PSHARED, H_TIMED},
    {"mtx_timedlock(mtx_timed)", MTX, A_DEFAULT, H_TIMED},
    {"mtx_timedlock(mtx_timed|mtx_recursive)", MTX, A_RECURSIVE, H_TIMED},
    {"pthread_cond_timedwait(default)", COND, A_DEFAULT, H_TIMED},
    {"pthread_cond_clockwait(MONOTONIC)", COND, A_DEFAULT, H_MONO},
    {"pthread_cond_clockwait(REALTIME)", COND, A_DEFAULT, H_REAL},
    {"pthread_cond_timedwait(condattr MONOTONIC)", COND, A_CONDMONO, H_TIMED},
    {"pthread_cond_timedwait(pshared)", COND, A_PSHARED, H_TIMED},
    {"cnd_timedwait", CND, A_DEFAULT, H_TIMED},
    {"pthread_rwlock_timedrdlock(default)", RWRD, A_DEFAULT, H_TIMED},
    {"pthread_rwlock_clockrdlock(MONOTONIC)", RWRD, A_DEFAULT, H_MONO},
    {"pthread_rwlock_clockrdlock(REALTIME)", RWRD, A_DEFAULT, H_REAL},
    {"pthread_rwlock_timedrdlock(prefer-writer)", RWRD, A_WRITERPREF, H_TIMED},
    {"pthread_rwlock_timedrdlock(pshared)", RWRD, A_PSHARED, H_TIMED},
    {"pthread_rwlock_timedwrlock(default)", RWWR, A_DEFAULT, H_TIMED},
    {"pthread_rwlock_clockwrlock(MONOTONIC)", RWWR, A_DEFAULT, H_MONO},
    {"pthread_rwlock_clockwrlock(REALTIME)", RWWR, A_DEFAULT, H_REAL},
    {"pthread_rwlock_timedwrlock(caller holds read)", RWWR_SELF, A_DEFAULT, H_TIMED},
    {"pthread_rwlock_clockwrlock(caller holds read, MONOTONIC)", RWWR_SELF, A_DEFAULT, H_MONO},
    {"sem_timedwait(private)", SEM, A_DEFAULT, H_TIMED},
    {"sem_clockwait(MONOTONIC)", SEM, A_DEFAULT, H_MONO},
    {"sem_clockwait(REALTIME)", SEM, A_DEFAULT, H_REAL},
    {"sem_timedwait(pshared)", SEM, A_PSHARED, H_TIMED},
    {"pthread_timedjoin_np", JOIN, A_DEFAULT, H_TIMED},
    {"pthread_clockjoin_np(MONOTONIC)", JOIN, A_DEFAULT, H_MONO},
    {"pthread_clockjoin_np(REALTIME)", JOIN, A_DEFAULT, H_REAL},
};
enum { NOPS = sizeof ops / sizeof ops[0] };

static const struct timespec deadlines[] = {
    {-1, 0}, {LONG_MIN, 0}, {1, 0}, {-1, 999999999}, {0, 0}, {-1, 1000000000}, {-1, -1},
};
static const char *const deadline_name[] = {
    "{-1, 0}", "{LONG_MIN, 0}", "{1, 0}", "{-1, 999999999}", "{0, 0}", "{-1, 1000000000}", "{-1, -1}",
};
enum { NDL = sizeof deadlines / sizeof deadlines[0], NDL_CHECK = 3 };

/* Child exit codes: an errno value (0 = success), or one of these. */
enum { C11_BASE = 200, SETUP = 250, BLOCKS = 251, NOT_RUN = 252, SKIP = 253 };

static sem_t held, release;
static int hold_result;
static pthread_mutex_t mu;
static mtx_t mx;
static pthread_rwlock_t rw;

static clockid_t clock_of(enum how h)
{
    return h == H_MONO ? CLOCK_MONOTONIC : CLOCK_REALTIME;
}

static int mutex_setup(enum attr at)
{
    pthread_mutexattr_t a;
    pthread_mutexattr_init(&a);
    int r = 0;
    switch (at) {
    case A_RECURSIVE: r = pthread_mutexattr_settype(&a, PTHREAD_MUTEX_RECURSIVE); break;
    case A_ERRORCHECK: r = pthread_mutexattr_settype(&a, PTHREAD_MUTEX_ERRORCHECK); break;
    case A_ADAPTIVE: r = pthread_mutexattr_settype(&a, PTHREAD_MUTEX_ADAPTIVE_NP); break;
    case A_ROBUST: r = pthread_mutexattr_setrobust(&a, PTHREAD_MUTEX_ROBUST); break;
    case A_PI: r = pthread_mutexattr_setprotocol(&a, PTHREAD_PRIO_INHERIT); break;
    case A_PP:
        r = pthread_mutexattr_setprotocol(&a, PTHREAD_PRIO_PROTECT);
        if (r == 0)
            r = pthread_mutexattr_setprioceiling(&a, sched_get_priority_min(SCHED_FIFO));
        break;
    case A_PSHARED: r = pthread_mutexattr_setpshared(&a, PTHREAD_PROCESS_SHARED); break;
    default: break;
    }
    if (r == 0)
        r = pthread_mutex_init(&mu, &a);
    pthread_mutexattr_destroy(&a);
    return r;
}

static int mutex_call(const struct op *o, const struct timespec *t)
{
    return o->how == H_TIMED ? pthread_mutex_timedlock(&mu, t) : pthread_mutex_clocklock(&mu, clock_of(o->how), t);
}

static int sem_call(sem_t *s, const struct op *o, const struct timespec *t)
{
    int r = o->how == H_TIMED ? sem_timedwait(s, t) : sem_clockwait(s, clock_of(o->how), t);
    return r == 0 ? 0 : errno;
}

static int rw_call(const struct op *o, int write, const struct timespec *t)
{
    if (write)
        return o->how == H_TIMED ? pthread_rwlock_timedwrlock(&rw, t) : pthread_rwlock_clockwrlock(&rw, clock_of(o->how), t);
    return o->how == H_TIMED ? pthread_rwlock_timedrdlock(&rw, t) : pthread_rwlock_clockrdlock(&rw, clock_of(o->how), t);
}

static int join_call(pthread_t th, const struct op *o, const struct timespec *t)
{
    return o->how == H_TIMED ? pthread_timedjoin_np(th, NULL, t) : pthread_clockjoin_np(th, NULL, clock_of(o->how), t);
}

/* The other thread: take the object (or just run, for a join), report, hold until released. */
static void *holder(void *p)
{
    const struct op *o = p;
    switch (o->kind) {
    case MUTEX: hold_result = pthread_mutex_lock(&mu); break;
    case MTX: hold_result = mtx_lock(&mx) == thrd_success ? 0 : SETUP; break;
    case RWRD: hold_result = pthread_rwlock_wrlock(&rw); break;
    case RWWR: hold_result = pthread_rwlock_rdlock(&rw); break;
    default: hold_result = 0; break;
    }
    sem_post(&held);
    if (hold_result != 0)
        return NULL;
    while (sem_wait(&release) != 0)
        ;
    switch (o->kind) {
    case MUTEX: pthread_mutex_unlock(&mu); break;
    case MTX: mtx_unlock(&mx); break;
    case RWRD: case RWWR: pthread_rwlock_unlock(&rw); break;
    default: break;
    }
    return NULL;
}

static void *quick(void *p)
{
    return p;
}

static int start_holder(pthread_t *th, const struct op *o)
{
    if (pthread_create(th, NULL, holder, (void *)o) != 0)
        return SETUP;
    while (sem_wait(&held) != 0)
        ;
    if (hold_result != 0) {
        pthread_join(*th, NULL);
        return SETUP;
    }
    return 0;
}

static void stop_holder(pthread_t th)
{
    sem_post(&release);
    pthread_join(th, NULL);
}

static int c11(int r)
{
    return C11_BASE + r;
}

/* One cell; runs in its own child. */
static int call(const struct op *o, const struct timespec *t, int contended)
{
    pthread_t th;
    int r;
    sem_init(&held, 0, 0);
    sem_init(&release, 0, 0);
    switch (o->kind) {
    case MUTEX:
        if (mutex_setup(o->attr) != 0)
            return SETUP;
        if (contended && start_holder(&th, o) != 0)
            return SETUP;
        r = mutex_call(o, t);
        if (r == 0)
            pthread_mutex_unlock(&mu);
        if (contended)
            stop_holder(th);
        return r;
    case MTX:
        if (mtx_init(&mx, o->attr == A_RECURSIVE ? mtx_timed | mtx_recursive : mtx_timed) != thrd_success)
            return SETUP;
        if (contended && start_holder(&th, o) != 0)
            return SETUP;
        r = mtx_timedlock(&mx, t);
        if (r == thrd_success)
            mtx_unlock(&mx);
        if (contended)
            stop_holder(th);
        return c11(r);
    case COND: {
        pthread_condattr_t a;
        pthread_cond_t cv;
        if (!contended)
            return SKIP;
        pthread_condattr_init(&a);
        if (o->attr == A_CONDMONO)
            pthread_condattr_setclock(&a, CLOCK_MONOTONIC);
        if (o->attr == A_PSHARED)
            pthread_condattr_setpshared(&a, PTHREAD_PROCESS_SHARED);
        if (pthread_cond_init(&cv, &a) != 0 || mutex_setup(A_DEFAULT) != 0)
            return SETUP;
        pthread_mutex_lock(&mu);
        r = o->how == H_TIMED ? pthread_cond_timedwait(&cv, &mu, t) : pthread_cond_clockwait(&cv, &mu, clock_of(o->how), t);
        pthread_mutex_unlock(&mu);
        return r;
    }
    case CND: {
        cnd_t cv;
        if (!contended)
            return SKIP;
        if (cnd_init(&cv) != thrd_success || mtx_init(&mx, mtx_plain) != thrd_success)
            return SETUP;
        mtx_lock(&mx);
        r = cnd_timedwait(&cv, &mx, t);
        mtx_unlock(&mx);
        return c11(r);
    }
    case RWRD:
    case RWWR:
    case RWWR_SELF: {
        pthread_rwlockattr_t a;
        pthread_rwlockattr_init(&a);
        if (o->attr == A_PSHARED)
            pthread_rwlockattr_setpshared(&a, PTHREAD_PROCESS_SHARED);
        if (o->attr == A_WRITERPREF)
            pthread_rwlockattr_setkind_np(&a, PTHREAD_RWLOCK_PREFER_WRITER_NONRECURSIVE_NP);
        if (pthread_rwlock_init(&rw, &a) != 0)
            return SETUP;
        if (o->kind == RWWR_SELF) {
            if (!contended)
                return SKIP;
            if (pthread_rwlock_rdlock(&rw) != 0)
                return SETUP;
        } else if (contended && start_holder(&th, o) != 0) {
            return SETUP;
        }
        r = rw_call(o, o->kind != RWRD, t);
        if (r == 0)
            pthread_rwlock_unlock(&rw);
        if (o->kind == RWWR_SELF)
            pthread_rwlock_unlock(&rw);
        else if (contended)
            stop_holder(th);
        return r;
    }
    case SEM: {
        sem_t s;
        if (sem_init(&s, o->attr == A_PSHARED, contended ? 0 : 1) != 0)
            return SETUP;
        return sem_call(&s, o, t);
    }
    case JOIN:
        if (contended) {
            if (start_holder(&th, o) != 0)
                return SETUP;
            r = join_call(th, o, t);
            sem_post(&release);
            if (r != 0)
                pthread_join(th, NULL);
            return r;
        }
        if (pthread_create(&th, NULL, quick, NULL) != 0)
            return SETUP;
        /* The thread ends at once; until it has, the call may report it running. */
        while ((r = join_call(th, o, t)) == ETIMEDOUT)
            sched_yield();
        if (r != 0)
            pthread_join(th, NULL);
        return r;
    }
    return SETUP;
}

static const char *code_name(int c, char *buf, size_t n)
{
    static const struct { int c; const char *s; } names[] = {
        {0, "0"}, {ETIMEDOUT, "ETIMEDOUT"}, {EINVAL, "EINVAL"}, {EDEADLK, "EDEADLK"}, {EBUSY, "EBUSY"},
        {EAGAIN, "EAGAIN"}, {EPERM, "EPERM"}, {ENOSYS, "ENOSYS"}, {EOWNERDEAD, "EOWNERDEAD"}, {EINTR, "EINTR"},
        {C11_BASE + thrd_success, "thrd_success"}, {C11_BASE + thrd_busy, "thrd_busy"},
        {C11_BASE + thrd_error, "thrd_error"}, {C11_BASE + thrd_nomem, "thrd_nomem"},
        {C11_BASE + thrd_timedout, "thrd_timedout"}, {SETUP, "setup failed"}, {BLOCKS, "blocks"},
        {NOT_RUN, "not run"}, {SKIP, "n/a"},
    };
    for (size_t i = 0; i < sizeof names / sizeof names[0]; i++)
        if (names[i].c == c)
            return names[i].s;
    snprintf(buf, n, "code %d", c);
    return buf;
}

/* The settled result of a checked cell. */
static int expected(const struct op *o, int contended, const struct timespec *t)
{
    int c11k = o->kind == MTX || o->kind == CND;
    /* A priority-inheritance mutex hands the deadline to the kernel's lock call, which rejects
     * a negative tv_sec: EINVAL, as the reference reports it. */
    if (o->kind == MUTEX && o->attr == A_PI && contended && t->tv_sec < 0)
        return EINVAL;
    if (contended || o->kind == COND || o->kind == CND)
        return c11k ? C11_BASE + thrd_timedout : ETIMEDOUT;
    return c11k ? C11_BASE + thrd_success : 0;
}

struct cell {
    int op, dl, contended, result;
};

int main(int argc, char **argv)
{
    int probe = argc > 1 && strcmp(argv[1], "--probe") == 0;
    unsigned guard = probe ? 3 : 30;
    enum { MAXCELLS = NOPS * NDL * 2, PAR = 16 };
    static struct cell cells[MAXCELLS];
    static pid_t pids[MAXCELLS];
    int n = 0;
    for (int o = 0; o < NOPS; o++)
        for (int d = 0; d < (probe ? NDL : NDL_CHECK); d++)
            for (int k = 1; k >= 0; k--) {
                /* Checked: every deadline contended; the uncontended success with {-1, 0}. */
                if (!probe && !k && d != 0)
                    continue;
                if (!k && (ops[o].kind == COND || ops[o].kind == CND || ops[o].kind == RWWR_SELF))
                    continue;
                /* Probe only: an unprivileged caller cannot take a priority-protected mutex
                 * from the reference (EINVAL even when free), so nothing is compared there. */
                if (!probe && ops[o].attr == A_PP)
                    continue;
                cells[n++] = (struct cell){o, d, k, NOT_RUN};
            }
    int next = 0, running = 0;
    while (next < n || running > 0) {
        while (next < n && running < PAR) {
            fflush(NULL);
            pid_t p = fork();
            if (p == 0) {
                alarm(guard);
                const struct cell *c = &cells[next];
                _exit(call(&ops[c->op], &deadlines[c->dl], c->contended));
            }
            if (p < 0) {
                perror("fork");
                return 100;
            }
            pids[next++] = p;
            running++;
        }
        int st;
        pid_t p = wait(&st);
        if (p < 0)
            break;
        running--;
        for (int i = 0; i < next; i++)
            if (pids[i] == p) {
                cells[i].result = WIFEXITED(st) ? WEXITSTATUS(st) : WIFSIGNALED(st) && WTERMSIG(st) == SIGALRM ? BLOCKS : SETUP;
                break;
            }
    }
    int failed = 0;
    char b1[32], b2[32];
    for (int i = 0; i < n; i++) {
        const struct cell *c = &cells[i];
        const struct op *o = &ops[c->op];
        if (probe) {
            printf("%-58s %-10s %-18s %s\n", o->name, c->contended ? "contended" : "free", deadline_name[c->dl],
                   code_name(c->result, b1, sizeof b1));
            continue;
        }
        int want = expected(o, c->contended, &deadlines[c->dl]);
        if (c->result != want) {
            failed++;
            fprintf(stderr, "FAIL %s, %s, deadline %s: %s, expected %s\n", o->name, c->contended ? "contended" : "free",
                    deadline_name[c->dl], code_name(c->result, b1, sizeof b1), code_name(want, b2, sizeof b2));
        }
    }
    if (!probe)
        printf("%d cells, %d failed\n", n, failed);
    return failed > 100 ? 100 : failed;
}
