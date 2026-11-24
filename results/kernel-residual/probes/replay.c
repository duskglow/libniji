/* KERNEL-RESIDUAL-PROFILE: replay a recorded sequence of mapping calls (mmap, munmap, mprotect, brk) in a fresh
   process, so that the kernel's cost of a loader's mapping pattern can be counted exactly, apart from its page
   faults, and variants of the pattern compared. Built static with the system compiler; only system calls matter.

   replay OPS 1   parse OPS, open its files, run every operation, exit (the mappings are torn down at exit)
   replay OPS 0   parse and open only (the baseline subtracted)

   OPS lines (written by replay.py):
     open <idx> <path>
     mmap <id> <base> <off> <len> <prot> <flags> <fd idx or -1> <pgoff>    base -1: no address
     munmap <base> <off> <len>
     mprotect <base> <off> <len> <prot>
     brk <delta>                                                          from the first break
   An address is the result of an earlier mmap <base> plus <off>. */
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <unistd.h>

#define MAXOPS 4096
struct op { char kind; long id, base, off, len, prot, flags, fd, pgoff; };
static struct op ops[MAXOPS];
static unsigned long res[MAXOPS];
static int fds[256];

int main(int argc, char **argv) {
    if (argc < 3) return 2;
    FILE *f = fopen(argv[1], "r");
    if (!f) return 2;
    char line[4096], path[4000];
    int n = 0;
    while (fgets(line, sizeof line, f) && n < MAXOPS) {
        struct op *o = &ops[n];
        long idx;
        memset(o, 0, sizeof *o);
        if (sscanf(line, "open %ld %3999s", &idx, path) == 2) {
            if (idx >= 0 && idx < 256) fds[idx] = open(path, O_RDONLY | O_CLOEXEC);
            continue;
        }
        if (sscanf(line, "mmap %ld %ld %ld %ld %ld %ld %ld %ld", &o->id, &o->base, &o->off, &o->len, &o->prot,
                   &o->flags, &o->fd, &o->pgoff) == 8) o->kind = 'm';
        else if (sscanf(line, "munmap %ld %ld %ld", &o->base, &o->off, &o->len) == 3) o->kind = 'u';
        else if (sscanf(line, "mprotect %ld %ld %ld %ld", &o->base, &o->off, &o->len, &o->prot) == 4) o->kind = 'p';
        else if (sscanf(line, "brk %ld", &o->len) == 1) o->kind = 'b';
        else continue;
        n++;
    }
    fclose(f);
    if (argv[2][0] != '1') return 0;
    unsigned long brk0 = 0;
    int bad = 0;
    for (int i = 0; i < n; i++) {
        struct op *o = &ops[i];
        unsigned long at = o->base >= 0 ? res[o->base] + (unsigned long)o->off : 0;
        if (o->kind == 'm') {
            int fd = o->fd >= 0 ? fds[o->fd] : -1;
            void *p = mmap((void *)at, (size_t)o->len, (int)o->prot, (int)o->flags, fd, o->pgoff);
            if (p == MAP_FAILED) { bad++; continue; }
            if (o->id >= 0 && o->id < MAXOPS) res[o->id] = (unsigned long)p;
        } else if (o->kind == 'u') {
            if (munmap((void *)at, (size_t)o->len)) bad++;
        } else if (o->kind == 'p') {
            if (mprotect((void *)at, (size_t)o->len, (int)o->prot)) bad++;
        } else if (o->kind == 'b') {
            if (!brk0) brk0 = (unsigned long)sbrk(0);
            if (brk((void *)(brk0 + (unsigned long)o->len))) bad++;
        }
    }
    return bad ? 3 : 0;
}
