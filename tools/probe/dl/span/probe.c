/* R-5 (spec/behaviour/dynlink.md), black-box probe: which pcs does _dl_find_object accept for a dlopen'd
 * object, relative to its PT_LOADs? Public interfaces only; the reference is only executed.
 *   cc -O1 -o probe probe.c && ./probe /abs/path/lib.so <offset from the load bias>...
 * prints, per offset, the result and (0) the reported span and PT_GNU_EH_FRAME as offsets. */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <link.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static const char *want;
static unsigned long base;
static int cb(struct dl_phdr_info *i, size_t s, void *d) {
    (void)s; (void)d;
    if (strstr(i->dlpi_name, want)) base = i->dlpi_addr;
    return 0;
}

int main(int argc, char **argv) {
    want = argv[1];
    if (!dlopen(argv[1], RTLD_NOW)) { printf("dlopen: %s\n", dlerror()); return 1; }
    dl_iterate_phdr(cb, 0);
    for (int k = 2; k < argc; k++) {
        unsigned long off = strtoul(argv[k], 0, 0);
        struct dl_find_object f;
        memset(&f, 0, sizeof f);
        int r = _dl_find_object((void *)(base + off), &f);
        printf("off %#lx -> %d", off, r);
        if (r == 0) printf(" map %#lx..%#lx eh %#lx", (unsigned long)f.dlfo_map_start - base, (unsigned long)f.dlfo_map_end - base, (unsigned long)f.dlfo_eh_frame - base);
        printf("\n");
    }
    return 0;
}
