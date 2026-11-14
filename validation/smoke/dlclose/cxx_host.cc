// validation/smoke/dlclose/cxx_host.cc: smoke program cxx_dlclose. Loads the C++ plug-in libplug.so three
// times, calls into it (function-local static, atexit handlers, an exception caught inside it), unloads it with
// dlclose each time, and throws an exception in the program after each unload; then loads libplug_tl.so,
// touches its thread_local object on a second thread and on the main thread, closes it, and exits. Every step
// prints one line; smoke.sh compares the whole output with the reference build's (kind `same`).
//
//   cxx_host DIR     (DIR holds libplug.so and libplug_tl.so)
#include <dlfcn.h>

#include <cstdio>
#include <cstdlib>
#include <stdexcept>
#include <string>
#include <thread>

static void *open_or_die(const std::string &path) {
    void *h = dlopen(path.c_str(), RTLD_NOW | RTLD_LOCAL);
    if (!h) {
        std::printf("host: dlopen failed: %s\n", dlerror());
        std::exit(1);
    }
    return h;
}

template <typename F> static F sym_or_die(void *h, const char *name) {
    void *p = dlsym(h, name);
    if (!p) {
        std::printf("host: dlsym %s failed: %s\n", name, dlerror());
        std::exit(1);
    }
    return reinterpret_cast<F>(p);
}

static void host_atexit() { std::printf("host: atexit handler\n"); }

int main(int argc, char **argv) {
    std::setvbuf(stdout, nullptr, _IONBF, 0);
    if (argc != 2) {
        std::printf("usage: cxx_host DIR\n");
        return 2;
    }
    std::string dir = argv[1];
    std::atexit(host_atexit);
    for (int round = 1; round <= 3; round++) {
        std::printf("host: round %d: open\n", round);
        void *h = open_or_die(dir + "/libplug.so");
        auto bump = sym_or_die<int (*)()>(h, "plug_bump");
        auto reg = sym_or_die<void (*)()>(h, "plug_register");
        auto thr = sym_or_die<int (*)(int)>(h, "plug_throw");
        int a = bump();
        int b = bump();
        std::printf("host: round %d: counter %d %d\n", round, a, b);
        reg();
        std::printf("host: round %d: plug_throw returned %d\n", round, thr(round));
        std::printf("host: round %d: close\n", round);
        if (dlclose(h) != 0) {
            std::printf("host: dlclose failed: %s\n", dlerror());
            return 1;
        }
        std::printf("host: round %d: closed\n", round);
        try {
            throw std::runtime_error("host error " + std::to_string(round));
        } catch (const std::exception &e) {
            std::printf("host: caught \"%s\"\n", e.what());
        }
    }
    void *t = open_or_die(dir + "/libplug_tl.so");
    auto touch = sym_or_die<void (*)(const char *)>(t, "plug_tl_touch");
    std::thread worker([touch] { touch("worker thread"); });
    worker.join();
    std::printf("host: worker joined\n");
    touch("main thread");
    std::printf("host: close thread_local plug-in\n");
    if (dlclose(t) != 0) {
        std::printf("host: dlclose failed: %s\n", dlerror());
        return 1;
    }
    std::printf("host: exit\n");
    return 0;
}
