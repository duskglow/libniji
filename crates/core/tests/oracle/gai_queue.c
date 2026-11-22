/* getaddrinfo_a's queue and notifications under volume, without DNS (spec §16.2; getaddrinfo_a(3):
 * every request of every call completes with its own getaddrinfo result, and every call's
 * requested notification is delivered once). Numeric requests only, so nothing leaves the
 * process and no root is needed. Built statically against libniji's products; the expected
 * lines follow from the man page (gai_queue_products.rs).
 *
 * Rounds: a few requests run to completion (GAI_WAIT), so the next requests start part-way round
 * whatever ring the library keeps, then one GAI_NOWAIT call of more requests than were ever
 * pending at once (a queue that grows does so with requests both before and after the wrap).
 * Then submitter threads make one-request calls at the same time, each notified by a queued
 * real-time signal or a SIGEV_THREAD call carrying the request's index.
 *
 * Every request asks for its own address (10.x.y.z from its index) and port; a request answered
 * with another's address, never answered or notified twice is reported. A request still in
 * progress after 30 s without any completion counts as lost (a hang guard: a healthy run takes
 * well under a second). Usage: gai_queue */
#define _GNU_SOURCE
#include <arpa/inet.h>
#include <netdb.h>
#include <netinet/in.h>
#include <pthread.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

enum { THREADS = 4, CALLS = 2000 };

static struct addrinfo hints;
static unsigned char *seen;
static int bad_values;

struct req {
    struct gaicb cb;
    char name[20];
    char port[8];
};

static void prepare(struct req *r, int i)
{
    snprintf(r->name, sizeof r->name, "10.%d.%d.%d", (i >> 16) & 255, (i >> 8) & 255, i & 255);
    snprintf(r->port, sizeof r->port, "%d", 1024 + i % 50000);
    memset(&r->cb, 0, sizeof r->cb);
    r->cb.ar_name = r->name;
    r->cb.ar_service = r->port;
    r->cb.ar_request = &hints;
}

/* Wait until no request of the n is in progress; the number still in progress when 30 s pass
 * without one completing (0 normally). */
static int wait_done(struct req *rs, int n)
{
    int left = n, idle = 0;
    while (left > 0 && idle < 30) {
        int now = 0;
        const struct gaicb *one[1] = {NULL};
        for (int i = 0; i < n; i++)
            if (gai_error(&rs[i].cb) == EAI_INPROGRESS) {
                if (!one[0])
                    one[0] = &rs[i].cb;
                now++;
            }
        if (now == 0)
            return 0;
        idle = now < left ? 0 : idle + 1;
        left = now;
        struct timespec t = {1, 0};
        gai_suspend(one, 1, &t);
    }
    return left;
}

/* Requests answered with exactly their own address and port, each result freed. */
static int exact(struct req *rs, int n, int first)
{
    int good = 0;
    for (int i = 0; i < n; i++) {
        if (gai_error(&rs[i].cb) != 0)
            continue;
        const struct addrinfo *a = rs[i].cb.ar_result;
        const struct sockaddr_in *s = a ? (const struct sockaddr_in *)a->ai_addr : NULL;
        int k = first + i;
        if (s && a->ai_family == AF_INET && !a->ai_next && s->sin_addr.s_addr == htonl(0x0a000000u | (unsigned)k) &&
            ntohs(s->sin_port) == 1024 + k % 50000)
            good++;
        freeaddrinfo(rs[i].cb.ar_result);
    }
    return good;
}

static void on_signal(int s, siginfo_t *si, void *u)
{
    (void)s, (void)u;
    int v = si->si_value.sival_int;
    if (si->si_code != SI_ASYNCNL || v < 0 || v >= THREADS * CALLS)
        __atomic_fetch_add(&bad_values, 1, __ATOMIC_RELAXED);
    else
        __atomic_fetch_add(&seen[v], 1, __ATOMIC_RELAXED);
}

static void on_thread(union sigval v)
{
    if (v.sival_int < 0 || v.sival_int >= THREADS * CALLS)
        __atomic_fetch_add(&bad_values, 1, __ATOMIC_RELAXED);
    else
        __atomic_fetch_add(&seen[v.sival_int], 1, __ATOMIC_RELAXED);
}

static struct req *conc;
static int refused;

static void *submitter(void *arg)
{
    int t = (int)(long)arg;
    for (int i = 0; i < CALLS; i++) {
        int k = t * CALLS + i;
        struct gaicb *one[] = {&conc[k].cb};
        struct sigevent ev;
        memset(&ev, 0, sizeof ev);
        ev.sigev_value.sival_int = k;
        if (k % 2) {
            ev.sigev_notify = SIGEV_THREAD;
            ev.sigev_notify_function = on_thread;
        } else {
            ev.sigev_notify = SIGEV_SIGNAL;
            ev.sigev_signo = SIGRTMIN;
        }
        if (getaddrinfo_a(GAI_NOWAIT, one, 1, &ev) != 0)
            __atomic_fetch_add(&refused, 1, __ATOMIC_RELAXED);
    }
    return NULL;
}

int main(void)
{
    hints.ai_family = AF_INET;
    hints.ai_socktype = SOCK_STREAM;
    hints.ai_flags = AI_NUMERICHOST | AI_NUMERICSERV;

    /* Rounds: `lead` requests to completion, then one call of `mass`. */
    static const int lead[] = {1, 37, 63, 500, 3001, 777};
    static const int mass[] = {100, 3000, 3000, 8000, 20000, 40000};
    int first = 0;
    for (unsigned r = 0; r < sizeof lead / sizeof *lead; r++) {
        int n = lead[r] + mass[r];
        struct req *rs = calloc(n, sizeof *rs);
        struct gaicb **list = calloc(n, sizeof *list);
        for (int i = 0; i < n; i++) {
            prepare(&rs[i], first + i);
            list[i] = &rs[i].cb;
        }
        int a = getaddrinfo_a(GAI_WAIT, list, lead[r], NULL);
        int b = getaddrinfo_a(GAI_NOWAIT, list + lead[r], mass[r], NULL);
        int lost = wait_done(rs, n);
        int good = exact(rs, n, first);
        printf("round %u: %d then %d requests: calls %d %d, exact %d of %d, lost %d\n", r, lead[r], mass[r], a, b, good, n, lost);
        free(list);
        free(rs);
        first += n;
    }

    /* Concurrent one-request calls with notifications; only the main thread takes the signal. */
    struct sigaction sa;
    memset(&sa, 0, sizeof sa);
    sa.sa_sigaction = on_signal;
    sa.sa_flags = SA_SIGINFO | SA_RESTART;
    sigaction(SIGRTMIN, &sa, NULL);
    int n = THREADS * CALLS;
    conc = calloc(n, sizeof *conc);
    seen = calloc(n, 1);
    for (int i = 0; i < n; i++)
        prepare(&conc[i], i);
    sigset_t b, old;
    sigemptyset(&b);
    sigaddset(&b, SIGRTMIN);
    pthread_sigmask(SIG_BLOCK, &b, &old);
    pthread_t th[THREADS];
    for (int t = 0; t < THREADS; t++)
        pthread_create(&th[t], NULL, submitter, (void *)(long)t);
    pthread_sigmask(SIG_SETMASK, &old, NULL);
    for (int t = 0; t < THREADS; t++)
        pthread_join(th[t], NULL);
    int lost = wait_done(conc, n);
    /* Notifications follow completion: wait for them the same way (30 s without a new one). */
    int got = 0, idle = 0;
    while (idle < 300) {
        int now = 0;
        for (int i = 0; i < n; i++)
            now += __atomic_load_n(&seen[i], __ATOMIC_RELAXED) != 0;
        if (now == n)
            break;
        idle = now > got ? 0 : idle + 1;
        got = now;
        usleep(100000);
    }
    usleep(200000);
    int missing = 0, twice = 0;
    for (int i = 0; i < n; i++) {
        int s = __atomic_load_n(&seen[i], __ATOMIC_RELAXED);
        missing += s == 0;
        twice += s > 1;
    }
    int good = exact(conc, n, 0);
    printf("concurrent: %d threads x %d calls: refused %d, exact %d of %d, lost %d\n", THREADS, CALLS, refused, good, n, lost);
    printf("notifications: missing %d, twice %d, bad %d\n", missing, twice, bad_values);
    free(conc);
    free(seen);
    return 0;
}
