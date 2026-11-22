// pthread_exit and acting on cancellation unwind the thread's stack (spec §22.1 s22_1_c_2,
// §22.3 s22_3_c_4; D-22.19; issue #25): built dynamically with the system g++ and run on libniji's
// products and on the host C library (the reference) by forced_unwind_products.rs. Each line names
// a case and what C++ frames saw: destructors run, `catch (abi::__forced_unwind &)` and
// `catch (...)` handlers entered (they rethrow), and the thread's result as `pthread_join` gives it
// (-1 for PTHREAD_CANCELED). With the argument `swallow` it then runs a handler that ends without
// rethrowing, which ends the process (D-22.19; run on libniji only).
#include <pthread.h>
#include <semaphore.h>
#include <unistd.h>
#include <time.h>
#include <cxxabi.h>
#include <cstdio>
#include <cstring>
#include <future>

static int dtors, caught_fu, caught_all;
static pthread_mutex_t m = PTHREAD_MUTEX_INITIALIZER;
struct Guard { ~Guard() { dtors++; } };
struct Unlock { ~Unlock() { pthread_mutex_unlock(&m); } };
static int pfd[2];
static sem_t sem;
static pthread_cond_t cv = PTHREAD_COND_INITIALIZER;
static pthread_t blocker;
static pthread_key_t key;
static char order[16];
static int norder;
static void note(char c) { if (norder < 15) order[norder++] = c; }

static void *t_block(void *) { for (;;) pause(); }
static void inner_exit() { Guard g; pthread_exit((void *)42); }
static void *t_dtor(void *) { Guard g; inner_exit(); return nullptr; }
static void *t_catch_forced(void *) {
    Guard g;
    try { pthread_exit((void *)7); }
    catch (abi::__forced_unwind &) { caught_fu++; throw; }
    return nullptr;
}
static void *t_catch_all(void *) {
    Guard g;
    try { inner_exit(); }
    catch (...) { caught_all++; throw; }
    return nullptr;
}
static void *t_lock(void *) {
    pthread_mutex_lock(&m);
    Unlock u;
    pthread_exit(nullptr);
}
// Asynchronous cancellation, in a frame with nothing to clean up and no handler, so the C++
// personality routine never looks at it. On a library that acts at once, the request can
// interrupt any instruction here (the loop's jump included); the frames above this one are
// then left at their call instructions, which their call-site tables cover. A frame with a
// handler interrupted between two calls is outside its table, and the personality routine calls
// std::terminate instead of unwinding (an outcome that depends on timing). libniji acts at the
// next safe point, pthread_testcancel (D-22.11).
__attribute__((noinline, noclone)) static void async_spin() {
    int old;
    pthread_setcanceltype(PTHREAD_CANCEL_ASYNCHRONOUS, &old);
    for (;;) pthread_testcancel();
}
// One cancellation point of each kind libniji has: a blocking call in the cancellation window
// (pause, nanosleep), a direct point (read), the futex waits (pthread_cond_wait, sem_wait,
// pthread_join), pthread_testcancel, and asynchronous cancellation.
static void *t_cancel(void *arg) {
    int kind = (int)(long)arg;
    Guard g;
    try {
        char c;
        switch (kind) {
        case 0: pause(); break;
        case 1: (void)!read(pfd[0], &c, 1); break;
        case 2: { struct timespec ts = {30, 0}; nanosleep(&ts, nullptr); break; }
        case 3: { pthread_mutex_lock(&m); Unlock u; pthread_cond_wait(&cv, &m); break; }
        case 4: sem_wait(&sem); break;
        case 5: for (;;) pthread_testcancel();
        case 6: pthread_join(blocker, nullptr); break;
        case 7: async_spin();
        }
    } catch (abi::__forced_unwind &) { caught_fu++; throw; }
    return nullptr;
}
static void keydtor(void *) { note('k'); }
static thread_local struct TL { int on; ~TL() { if (on) note('t'); } } tl;
static void *t_order(void *) {
    pthread_setspecific(key, (void *)1);
    tl.on = 1;
    struct L { ~L() { note('d'); } } l;
    pthread_exit(nullptr);
}
static void *t_cleanup_push(void *) {
    pthread_cleanup_push([](void *) { note('h'); }, nullptr);
    { struct L { ~L() { note('d'); } } l; pthread_exit(nullptr); }
    pthread_cleanup_pop(0);
    return nullptr;
}
static pthread_once_t once = PTHREAD_ONCE_INIT;
static int once_runs;
static void once_init() {
    if (++once_runs == 1) { Guard g; pthread_exit(nullptr); }
}
static void *t_once(void *) { pthread_once(&once, once_init); return nullptr; }
static void *t_swallow(void *) {
    Guard outer;
    try { Guard g; pthread_exit((void *)5); }
    catch (...) { std::printf("swallow: handler entered, dtors=%d\n", dtors); }
    std::printf("swallow: code after the handler ran\n");
    return (void *)99;
}

static void run(const char *name, void *(*f)(void *), void *arg, bool cancel) {
    dtors = caught_fu = caught_all = 0;
    pthread_t t;
    pthread_create(&t, nullptr, f, arg);
    if (cancel) {
        struct timespec ts = {0, 20 * 1000000L};
        nanosleep(&ts, nullptr);
        pthread_cancel(t);
    }
    void *r;
    pthread_join(t, &r);
    std::printf("%s: dtors=%d caught_fu=%d caught_all=%d result=%ld\n", name, dtors, caught_fu, caught_all,
                r == PTHREAD_CANCELED ? -1L : (long)r);
}

int main(int argc, char **argv) {
    setvbuf(stdout, nullptr, _IOLBF, 0);
    (void)!pipe(pfd);
    sem_init(&sem, 0, 0);
    pthread_key_create(&key, keydtor);
    pthread_create(&blocker, nullptr, t_block, nullptr);
    run("exit-dtor", t_dtor, nullptr, false);
    run("exit-catch-forced", t_catch_forced, nullptr, false);
    run("exit-catch-all", t_catch_all, nullptr, false);
    run("exit-lock", t_lock, nullptr, false);
    std::printf("exit-lock: trylock after join=%d\n", pthread_mutex_trylock(&m));
    pthread_mutex_unlock(&m);
    const char *points[] = {"pause", "read", "nanosleep", "cond_wait", "sem_wait", "testcancel", "join", "async"};
    for (int k = 0; k < 8; k++) {
        char n[64];
        std::snprintf(n, sizeof n, "cancel-%s", points[k]);
        run(n, t_cancel, (void *)(long)k, true);
    }
    std::printf("cancel-cond_wait: trylock after join=%d\n", pthread_mutex_trylock(&m));
    pthread_mutex_unlock(&m);
    norder = 0;
    run("order", t_order, nullptr, false);
    std::printf("order: %.*s\n", norder, order);
    norder = 0;
    run("cleanup-push", t_cleanup_push, nullptr, false);
    std::printf("cleanup-push: %.*s\n", norder, order);
    run("once", t_once, nullptr, false);
    pthread_once(&once, once_init);
    std::printf("once: runs=%d\n", once_runs);
    {
        auto fut = std::async(std::launch::async, [] { pthread_exit(nullptr); return 1; });
        try { fut.get(); std::printf("async: value\n"); }
        catch (std::future_error &e) { std::printf("async: broken_promise=%d\n", e.code() == std::future_errc::broken_promise); }
    }
    if (argc > 1 && std::strcmp(argv[1], "swallow") == 0) {
        run("swallow", t_swallow, nullptr, false);
        std::printf("swallow: the process went on\n");
    }
    struct MainGuard { ~MainGuard() { (void)!write(1, "main-exit: dtor ran\n", 20); } } mg;
    pthread_cancel(blocker);
    pthread_exit(nullptr);
}
