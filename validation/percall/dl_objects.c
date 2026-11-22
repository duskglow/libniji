/* DL-FIND-OBJECT-FAST: _dl_find_object, dladdr and one dl_iterate_phdr walk with 4, 32, 104 and 600 objects
 * loaded, for the first, middle and last object in dl_iterate_phdr order (the vDSO left out). The objects past
 * the start-time ones are libq<k>.so from the directory $PC_DL_DIR, made by validation/percall/dl_objects.sh
 * (`-nostdlib` libraries, one function each); the setup (dlopen of them) is the same in the n = 0 and n = N
 * runs, so it cancels. The C++ unwinders call _dl_find_object once per frame. */
#include "percall.h"
#include <dlfcn.h>
#include <link.h>
#include <string.h>

struct pick { unsigned long at[3]; int n; };

static int collect(struct dl_phdr_info *i, size_t sz, void *d) {
    unsigned long *a = d;
    (void)sz;
    if (strstr(i->dlpi_name, "linux-vdso")) return 0;
    for (int k = 0; k < i->dlpi_phnum; k++)
        if (i->dlpi_phdr[k].p_type == PT_LOAD && (i->dlpi_phdr[k].p_flags & PF_X)) {
            a[1 + a[0]++] = i->dlpi_addr + i->dlpi_phdr[k].p_vaddr + 16;
            break;
        }
    return a[0] >= 1023;
}

static int count_cb(struct dl_phdr_info *i, size_t sz, void *d) { (void)i; (void)sz; ++*(long *)d; return 0; }

/* Load libraries until `want` objects are loaded (the vDSO counted), then the first, middle and last. */
static struct pick load(int want) {
    static unsigned long a[1024];
    long have = 0;
    dl_iterate_phdr(count_cb, &have);
    const char *d = getenv("PC_DL_DIR");
    if (!d && have < want) { fprintf(stderr, "dl_objects: set PC_DL_DIR (validation/percall/dl_objects.sh)\n"); exit(3); }
    for (int k = 1; have < want; k++, have++) {
        char p[4096];
        snprintf(p, sizeof p, "%s/libq%d.so", d, k);
        if (!dlopen(p, RTLD_NOW)) { fprintf(stderr, "dl_objects: %s\n", dlerror()); exit(3); }
    }
    a[0] = 0;
    dl_iterate_phdr(collect, a);
    struct pick r = { { a[1], a[1 + a[0] / 2], a[a[0]] }, (int)a[0] };
    return r;
}

static void find_n(long n, int want, int which) {
    struct pick p = load(want);
    void *pc = (void *)p.at[which];
    struct dl_find_object f;
    for (long i = 0; i < n; i++) pc_sink += _dl_find_object(pc, &f);
    PC_CHECKING { pc_check("%d %d", _dl_find_object(pc, &f), p.n); }
}

static void dladdr_n(long n, int want, int which) {
    struct pick p = load(want);
    void *a = (void *)p.at[which];
    Dl_info info;
    for (long i = 0; i < n; i++) pc_sink += dladdr(a, &info);
    PC_CHECKING { pc_check("%d %d", dladdr(a, &info), p.n); }
}

static int nothing(struct dl_phdr_info *i, size_t sz, void *d) { (void)i; (void)sz; (void)d; return 0; }

static void walk_n(long n, int want) {
    struct pick p = load(want);
    for (long i = 0; i < n; i++) pc_sink += dl_iterate_phdr(nothing, 0);
    PC_CHECKING { long c = 0; dl_iterate_phdr(count_cb, &c); pc_check("%ld", c); (void)p; }
}

/* DL-READER-COSTS: one dlopen + dlclose pair per call with `want` objects loaded. `new`: the object is not loaded
 * (libq<want+1+i%8>.so in turn, so every open maps, relocates and publishes one object and the close unmaps it;
 * the corpus needs want + 8 libraries). `held`: libq1.so by path, already open (libc.so.6 by name at 4 objects,
 * where no libq is loaded): a reference count up and down. */
static void open_new_n(long n, int want) {
    struct pick p = load(want);
    const char *d = getenv("PC_DL_DIR");
    char path[8][4096];
    for (int k = 0; k < 8; k++) snprintf(path[k], sizeof path[k], "%s/libq%d.so", d, want + 1 + k);
    for (long i = 0; i < n; i++) {
        void *h = dlopen(path[i & 7], RTLD_NOW);
        pc_sink += h != 0;
        if (h) pc_sink += dlclose(h);
    }
    PC_CHECKING { long c = 0; void *h = dlopen(path[0], RTLD_NOW); dl_iterate_phdr(count_cb, &c);
                  pc_check("%d %ld %d", h != 0, c, h ? dlclose(h) : -1); (void)p; }
}

static void open_held_n(long n, int want) {
    struct pick p = load(want);
    const char *d = getenv("PC_DL_DIR");
    char path[4096];
    if (p.n > 4) snprintf(path, sizeof path, "%s/libq1.so", d ? d : ".");
    else snprintf(path, sizeof path, "libc.so.6");
    for (long i = 0; i < n; i++) {
        void *h = dlopen(path, RTLD_NOW);
        pc_sink += h != 0;
        if (h) pc_sink += dlclose(h);
    }
    PC_CHECKING { void *h = dlopen(path, RTLD_NOW); pc_check("%d %d %d", h != 0, h ? dlclose(h) : -1, p.n); }
}

#define ROWS(N) \
    CASE(dlopen_new_##N, 0, 1) { (void)dir; open_new_n(n, N); } \
    CASE(dlopen_held_##N, 0, 1) { (void)dir; open_held_n(n, N); } \
    CASE(dl_find_object_first_##N, 0, 1) { (void)dir; find_n(n, N, 0); } \
    CASE(dl_find_object_middle_##N, 0, 1) { (void)dir; find_n(n, N, 1); } \
    CASE(dl_find_object_last_##N, 0, 1) { (void)dir; find_n(n, N, 2); } \
    CASE(dladdr_first_##N, 0, 1) { (void)dir; dladdr_n(n, N, 0); } \
    CASE(dladdr_middle_##N, 0, 1) { (void)dir; dladdr_n(n, N, 1); } \
    CASE(dladdr_last_##N, 0, 1) { (void)dir; dladdr_n(n, N, 2); } \
    CASE(dl_iterate_phdr_walk_##N, 0, 1) { (void)dir; walk_n(n, N); }

ROWS(4)
ROWS(32)
ROWS(104)
ROWS(600)
