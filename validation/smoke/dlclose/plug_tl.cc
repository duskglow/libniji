// validation/smoke/dlclose/plug_tl.cc: a plug-in with a thread_local object (smoke program cxx_dlclose).
// cxx_host touches it on a second thread and on the main thread and closes the plug-in while the main
// thread's object is still alive: its destructor must still run at exit (the object stays loaded until then),
// and the second thread's destructor runs when that thread ends.
#include <cstdio>
#include <string>

namespace {
struct PerThread {
    std::string who;
    explicit PerThread(const char *w) : who(w) { std::printf("plug_tl: construct for %s\n", who.c_str()); }
    ~PerThread() { std::printf("plug_tl: destroy for %s\n", who.c_str()); }
};
}  // namespace

extern "C" void plug_tl_touch(const char *who) {
    thread_local PerThread object(who);
    std::printf("plug_tl: touched by %s\n", object.who.c_str());
}

struct Global {
    ~Global() { std::printf("plug_tl: destroy global\n"); }
};
static Global global;
