// validation/smoke/dlclose/plug.cc: the C++ plug-in cxx_host loads and unloads (smoke program cxx_dlclose).
// Every event prints one line; cxx_host's output on libniji must equal its output on the reference build, so
// the order of constructors, destructors, atexit handlers and function-local statics at dlclose is compared
// line by line. A fresh load starts from fresh data (the counter) when the previous dlclose unloaded it.
// No libstdc++ types: the libniji build links libstdc++ statically into each object, and its STB_GNU_UNIQUE
// symbols would make this object differ from the reference build's (the recipe checks there are none).
#include <cstdio>
#include <cstdlib>

namespace {
struct Noisy {
    const char *name;
    explicit Noisy(const char *n) : name(n) { std::printf("plug: construct %s\n", name); }
    ~Noisy() { std::printf("plug: destroy %s\n", name); }
};
Noisy first("global-1");
Noisy second("global-2");
int counter;
int loads;

struct PlugError {
    int n;
};

void handler_a() { std::printf("plug: atexit handler a\n"); }
void handler_b() { std::printf("plug: atexit handler b\n"); }
}  // namespace

__attribute__((constructor)) static void plug_ctor() { std::printf("plug: constructor attribute (loads %d)\n", ++loads); }
__attribute__((destructor)) static void plug_dtor() { std::printf("plug: destructor attribute\n"); }

extern "C" int plug_bump() {
    static Noisy local("function-local");  // registered with __cxa_atexit at the first call
    return ++counter;
}

extern "C" void plug_register() {
    // atexit in a shared object: the handlers belong to this object and run when it is unloaded.
    std::atexit(handler_a);
    std::atexit(handler_b);
}

extern "C" int plug_throw(int n) {
    // An exception thrown and caught inside the plug-in: the unwinder finds this object's frames.
    try {
        if (n > 0) throw PlugError{n};
        return 0;
    } catch (const PlugError &e) {
        std::printf("plug: caught plug error %d\n", e.n);
        return e.n;
    }
}
