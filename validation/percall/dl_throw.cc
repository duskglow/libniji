// LAB-ONLY (DL-FIND-OBJECT-FAST): the cost of one C++ throw through 10 frames, caught one frame above them,
// with a given number of objects loaded. The unwinder looks up each frame's object with _dl_find_object in both
// phases. Built twice from this file: the program, and (-DDL_THROW_LIB) libdlthrow.so, which validation/percall/
// dl_throw.sh loads LAST, after the corpus of validation/percall/dl_objects.sh, so its frames are in the last object
// loaded. Usage: dl_throw <throws> <objects wanted> <prog|lib> <corpus dir> <libdlthrow.so>; the frames are in the
// program (the first object) or in the library (the last). Prints the object count and the throws caught.
#include <dlfcn.h>
#include <link.h>
#include <cstdio>
#include <cstdlib>
#include <cstring>

static volatile int sink;

__attribute__((noinline)) static int down(int k) {
    if (k == 0) throw k;
    int r = down(k - 1);
    sink = sink + r;  // not a tail call: every level keeps its frame
    return r;
}

extern "C" long dl_throw_run(long n) {
    long caught = 0;
    for (long i = 0; i < n; i++) {
        try {
            down(9);
        } catch (int) {
            caught++;
        }
    }
    return caught;
}

#ifndef DL_THROW_LIB
static int count_cb(struct dl_phdr_info *, size_t, void *d) {
    ++*static_cast<long *>(d);
    return 0;
}

int main(int argc, char **argv) {
    if (argc != 6) {
        std::fprintf(stderr, "usage: dl_throw <throws> <objects> <prog|lib> <corpus dir> <libdlthrow.so>\n");
        return 2;
    }
    long n = std::atol(argv[1]);
    long want = std::atol(argv[2]);
    bool in_lib = std::strcmp(argv[3], "lib") == 0;
    long have = 0;
    dl_iterate_phdr(count_cb, &have);
    // The corpus first, then libdlthrow.so as the last object (counted in `want`).
    for (long k = 1; have + 1 < want; k++, have++) {
        char p[4096];
        std::snprintf(p, sizeof p, "%s/libq%ld.so", argv[4], k);
        if (!dlopen(p, RTLD_NOW)) {
            std::fprintf(stderr, "dl_throw: %s\n", dlerror());
            return 3;
        }
    }
    void *h = dlopen(argv[5], RTLD_NOW);
    if (!h) {
        std::fprintf(stderr, "dl_throw: %s\n", dlerror());
        return 3;
    }
    long (*run)(long) = dl_throw_run;
    if (in_lib) run = reinterpret_cast<long (*)(long)>(dlsym(h, "dl_throw_run"));
    long total = 0;
    dl_iterate_phdr(count_cb, &total);
    long caught = run(n);
    std::printf("objects %ld caught %ld\n", total, caught);
    return caught == n ? 0 : 1;
}
#endif
