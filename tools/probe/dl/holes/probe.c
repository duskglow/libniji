/* R-6 (spec/behaviour/dynlink.md), black-box probe and the test program of crates/ld/tests/address_holes.rs:
 * which object do _dl_find_object and dladdr name for an address in a hole the kernel left between a
 * program's segment runs, when another object or an anonymous page lies there? Public interfaces only;
 * the reference is only executed.
 *   cc -O1 -fPIC -shared -o libL.so l.c   (l.c: void l_func(void) {} int l_register(void (*f)(void)) { return atexit(f); })
 *   cc -O1 -o probe probe.c -Wl,-z,max-page-size=0x200000 && ./probe /abs/path/libL.so hole
 * MODE "hole": L must land in the program's highest hole; "any": wherever it lands.
 * The program's own PT_LOAD span has holes (linked with a large max-page-size); the free address
 * space above its highest hole is filled with PROT_NONE so the next mapping the kernel places
 * top-down lands in that hole; then L is dlopen'd. Checks: _dl_find_object and dladdr name L for L's
 * code and the program for main; an anonymous page at the bottom of the hole is printed (the
 * reference: -1 and 0); an atexit handler of the program registered by L runs at L's dlclose.
 * Prints "ok ..." / "FAIL ..." lines, exit 0 when every check passed. No malloc after the fill. */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <link.h>
#include <stdarg.h>
#include <stdio.h>
#include <string.h>
#include <sys/mman.h>
#include <fcntl.h>
#include <unistd.h>

#ifndef MAP_FIXED_NOREPLACE
#define MAP_FIXED_NOREPLACE 0x100000
#endif

static int fails;
static void say(const char *fmt, ...) {
    char b[512];
    va_list ap;
    va_start(ap, fmt);
    int n = vsnprintf(b, sizeof b, fmt, ap);
    va_end(ap);
    if (n > 0) (void)!write(1, b, (size_t)n < sizeof b ? (size_t)n : sizeof b - 1);
}
static void check(int ok, const char *what) {
    say("%s %s\n", ok ? "ok" : "FAIL", what);
    fails += !ok;
}

static int ran;
void main_handler(void) { ran++; }

static unsigned long seg_lo[16], seg_hi[16];
static int nseg;
static int first_object(struct dl_phdr_info *i, size_t s, void *d) {
    (void)s; (void)d;
    for (int k = 0; k < i->dlpi_phnum && nseg < 16; k++) {
        if (i->dlpi_phdr[k].p_type != PT_LOAD) continue;
        seg_lo[nseg] = i->dlpi_addr + i->dlpi_phdr[k].p_vaddr;
        seg_hi[nseg] = seg_lo[nseg] + i->dlpi_phdr[k].p_memsz;
        nseg++;
    }
    return 1; /* the program only */
}

static unsigned long page;
static unsigned long up(unsigned long a) { return (a + page - 1) & ~(page - 1); }

/* Fill every free gap of the address space in [from, to) with PROT_NONE. */
static void fill(unsigned long from, unsigned long to) {
    static char b[1 << 16];
    int fd = open("/proc/self/maps", O_RDONLY);
    ssize_t n, t = 0;
    while (fd >= 0 && t < (ssize_t)sizeof b - 1 && (n = read(fd, b + t, sizeof b - 1 - t)) > 0) t += n;
    if (fd >= 0) close(fd);
    b[t] = 0;
    static unsigned long lo[1024], hi[1024];
    int m = 0;
    for (char *p = b; *p && m < 1024; ) {
        if (sscanf(p, "%lx-%lx", &lo[m], &hi[m]) == 2) m++;
        char *e = strchr(p, '\n');
        if (!e) break;
        p = e + 1;
    }
    unsigned long at = from;
    for (int k = 0; k <= m && at < to; k++) {
        unsigned long next = k < m ? lo[k] : to;
        if (next > to) next = to;
        if (next > at) {
            void *r = mmap((void *)at, next - at, PROT_NONE, MAP_PRIVATE | MAP_ANONYMOUS | MAP_NORESERVE | MAP_FIXED_NOREPLACE, -1, 0);
            if (r != (void *)at) say("note: fill %#lx-%#lx failed\n", at, next);
        }
        if (k < m && hi[k] > at) at = hi[k];
    }
}

/* The start of the [stack] mapping. */
static unsigned long stack_start(void) {
    char line[512];
    FILE *f = fopen("/proc/self/maps", "r");
    unsigned long lo = 0, hi = 0, s = 0;
    while (f && fgets(line, sizeof line, f))
        if (strstr(line, "[stack]") && sscanf(line, "%lx-%lx", &lo, &hi) == 2) s = lo;
    if (f) fclose(f);
    return s;
}

static void find(const char *what, void *p, void *want_map) {
    struct dl_find_object r;
    memset(&r, 0, sizeof r);
    int rc = _dl_find_object(p, &r);
    char t[200];
    unsigned long a = (unsigned long)p;
    snprintf(t, sizeof t, "_dl_find_object(%s) rc %d [%#lx, %#lx) map %p want %p", what, rc, (unsigned long)r.dlfo_map_start,
             (unsigned long)r.dlfo_map_end, (void *)r.dlfo_link_map, want_map);
    check(rc == 0 && (void *)r.dlfo_link_map == want_map && (unsigned long)r.dlfo_map_start <= a && a < (unsigned long)r.dlfo_map_end, t);
}

static void addr(const char *what, void *p, unsigned long want_base) {
    Dl_info d;
    memset(&d, 0, sizeof d);
    int rc = dladdr(p, &d);
    char t[160];
    snprintf(t, sizeof t, "dladdr(%s) rc %d base %#lx want %#lx", what, rc, (unsigned long)d.dli_fbase, want_base);
    check(rc != 0 && (unsigned long)d.dli_fbase == want_base, t);
}

int main(int argc, char **argv) {
    if (argc < 3) return 90;
    page = (unsigned long)sysconf(_SC_PAGESIZE);
    dl_iterate_phdr(first_object, 0);
    unsigned long lo = ~0ul, hi = 0, hole_lo = 0, hole_hi = 0, low_lo = 0, low_hi = 0;
    for (int k = 0; k < nseg; k++) {
        if (seg_lo[k] < lo) lo = seg_lo[k];
        if (seg_hi[k] > hi) hi = seg_hi[k];
        if (k + 1 < nseg && seg_lo[k + 1] > up(seg_hi[k])) {
            if (!low_hi) { low_lo = up(seg_hi[k]); low_hi = seg_lo[k + 1] & ~(page - 1); }
            hole_lo = up(seg_hi[k]);
            hole_hi = seg_lo[k + 1] & ~(page - 1);
        }
    }
    lo &= ~(page - 1);
    say("program [%#lx, %#lx), highest hole [%#lx, %#lx), lowest hole [%#lx, %#lx)\n", lo, hi, hole_lo, hole_hi, low_lo, low_hi);
    int want_hole = strcmp(argv[2], "hole") == 0;
    unsigned long anon = 0;
    if (hole_hi > hole_lo) {
        /* An anonymous page at the bottom of the highest hole. */
        void *a = mmap((void *)hole_lo, page, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE, -1, 0);
        anon = a == (void *)hole_lo ? hole_lo : 0;
        unsigned long top = stack_start();
        if (want_hole && top) fill(hole_hi, top - (256ul << 20));
    }
    void *h = dlopen(argv[1], RTLD_NOW);
    if (!h) { say("dlopen: %s\n", dlerror()); return 91; }
    struct link_map *lm = 0;
    dlinfo(h, RTLD_DI_LINKMAP, &lm);
    unsigned long l_lo = lm ? lm->l_addr : 0;
    int in_hole = l_lo >= hole_lo && l_lo < hole_hi;
    say("L at %#lx (%s the program's highest hole)\n", l_lo, in_hole ? "in" : "not in");
    if (want_hole) check(in_hole, "L placed in the program's hole");
    void *lf = dlsym(h, "l_func");
    int (*reg)(void (*)(void)) = (int (*)(void (*)(void)))dlsym(h, "l_register");
    check(lf && reg, "dlsym");
    if (!lf || !reg) return 92;
    void *self = dlopen(0, RTLD_NOW);
    find("main", (void *)main, self);
    addr("main", (void *)main, lo);
    find("l_func", lf, h);
    addr("l_func", lf, l_lo);
    if (anon) {
        struct dl_find_object r;
        Dl_info d;
        memset(&r, 0, sizeof r);
        memset(&d, 0, sizeof d);
        int f = _dl_find_object((void *)anon, &r);
        int a = dladdr((void *)anon, &d);
        say("anon page %#lx in the highest hole: _dl_find_object rc %d start %#lx; dladdr rc %d base %#lx\n", anon, f,
            (unsigned long)r.dlfo_map_start, a, (unsigned long)d.dli_fbase);
    }
    check(reg(main_handler) == 0, "L registers a handler of the program");
    check(dlclose(h) == 0, "dlclose(L)");
    check(ran == 1, "the handler L registered ran at L's dlclose");
    find("main after dlclose", (void *)main, self);
    addr("main after dlclose", (void *)main, lo);
    say("%s\n", fails ? "FAILED" : "PASSED");
    return fails ? 1 : 0;
}
