// validation/smoke/dlclose/plug_tl.cc: a plug-in with a thread_local object (smoke program cxx_dlclose).
// cxx_host touches it on a second thread and on the main thread, then closes the plug-in and exits; the
// whole output is compared with the reference build's. No libstdc++ types (see plug.cc).
#include <cstdio>

namespace {
struct PerThread {
    const char *who;
    explicit PerThread(const char *w) : who(w) { std::printf("plug_tl: construct for %s\n", who); }
    ~PerThread() { std::printf("plug_tl: destroy for %s\n", who); }
};
}  // namespace

extern "C" void plug_tl_touch(const char *who) {
    thread_local PerThread object(who);
    std::printf("plug_tl: touched by %s\n", object.who);
}

struct Global {
    ~Global() { std::printf("plug_tl: destroy global\n"); }
};
static Global global;
