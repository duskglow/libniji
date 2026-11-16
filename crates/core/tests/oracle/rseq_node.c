/* The heap's node read on several NUMA nodes (D-3.21): a thread libniji created, pinned to a CPU of a
 * node other than node 0, takes its first slab from that node, reading the node from the rseq
 * area libniji registered for it. The vDSO is unmapped before the thread starts, so a node read
 * through the vDSO getcpu would fault: a clean run shows the node came from rseq. Prints
 * "node <n> page <p>" (n: the CPU's node from the getcpu system call; p: the node the block's
 * page sits on) on stderr, or "skip <why>". Only system calls and the library's own malloc,
 * pthread and string-free calls are used after the unmap (no clock call). */
#include <pthread.h>
#include <stdlib.h>
#include <sys/auxv.h>
#include <sys/mman.h>
#include <sys/syscall.h>
#include <unistd.h>

extern unsigned int __rseq_size;

static long target_cpu = -1, target_node = -1, page_node = -1;

static void say(const char *a, long x, const char *b, long y) {
    char buf[96];
    int n = 0;
    for (const char *p = a; *p; p++) buf[n++] = *p;
    if (x >= 0) {
        char d[24];
        int k = 0;
        do d[k++] = (char)('0' + x % 10); while ((x /= 10) > 0);
        while (k) buf[n++] = d[--k];
    }
    for (const char *p = b; *p; p++) buf[n++] = *p;
    if (y >= 0) {
        char d[24];
        int k = 0;
        do d[k++] = (char)('0' + y % 10); while ((y /= 10) > 0);
        while (k) buf[n++] = d[--k];
    }
    buf[n++] = '\n';
    write(2, buf, (size_t)n);
}

static int pin(long cpu) {
    unsigned long mask[16] = {0};
    mask[cpu / 64] = 1UL << (cpu % 64);
    return (int)syscall(SYS_sched_setaffinity, 0, sizeof mask, mask);
}

static long cpu_node(void) {
    unsigned cpu = 0, node = 0;
    if (syscall(SYS_getcpu, &cpu, &node, 0) != 0) return -1;
    return (long)node;
}

static void *worker(void *arg) {
    (void)arg;
    if (pin(target_cpu) != 0 || cpu_node() != target_node) return (void *)1;
    char *p = malloc(64);
    if (!p) return (void *)2;
    p[0] = 1;
    int node = -1;
    /* MPOL_F_NODE | MPOL_F_ADDR: the node of the page holding p. */
    if (syscall(SYS_get_mempolicy, &node, 0, 0, p, 3) != 0) return (void *)3;
    page_node = node;
    return 0;
}

int main(void) {
    unsigned long allowed[16] = {0};
    if (syscall(SYS_get_mempolicy, 0, allowed, 1024, 0, 4 /* MPOL_F_MEMS_ALLOWED */) != 0) {
        say("skip get_mempolicy refused", -1, "", -1);
        return 0;
    }
    int nodes = 0;
    for (int i = 0; i < 16; i++) nodes += __builtin_popcountl(allowed[i]);
    if (nodes < 2) {
        say("skip one allowed node", -1, "", -1);
        return 0;
    }
    if (getauxval(27 /* AT_RSEQ_FEATURE_SIZE */) < 24 || __rseq_size < 24) {
        say("skip the kernel does not fill rseq node_id (feature size ", (long)getauxval(27), ")", -1);
        return 0;
    }
    for (long c = 0; c < 1024 && target_cpu < 0; c++) {
        if (pin(c) == 0) {
            long n = cpu_node();
            if (n > 0) {
                target_cpu = c;
                target_node = n;
            }
        }
    }
    if (target_cpu < 0) {
        say("skip no CPU of a node other than 0 is allowed", -1, "", -1);
        return 0;
    }
    pin(target_cpu);
    /* Unmap the vDSO (its [vdso] mapping starts at AT_SYSINFO_EHDR; two pages cover the image on
     * x86-64 kernels of this era, and unmapping beyond it would reach the next mapping, so take
     * exactly what /proc/self/maps names). */
    unsigned long base = getauxval(AT_SYSINFO_EHDR);
    if (base) {
        static char maps[65536];
        int fd = (int)syscall(SYS_open, "/proc/self/maps", 0);
        long got = 0, r;
        while (fd >= 0 && got < (long)sizeof maps - 1 && (r = syscall(SYS_read, fd, maps + got, sizeof maps - 1 - got)) > 0) got += r;
        maps[got] = 0;
        unsigned long end = 0;
        for (char *l = maps; *l;) {
            unsigned long a = 0, b = 0;
            char *q = l;
            for (; *q && *q != '-'; q++) a = a * 16 + (unsigned long)(*q <= '9' ? *q - '0' : *q - 'a' + 10);
            for (q++; *q && *q != ' '; q++) b = b * 16 + (unsigned long)(*q <= '9' ? *q - '0' : *q - 'a' + 10);
            if (a == base) end = b;
            while (*l && *l != '\n') l++;
            if (*l) l++;
        }
        if (fd >= 0) syscall(SYS_close, fd);
        if (end == 0 || syscall(SYS_munmap, base, end - base) != 0) {
            say("skip the vDSO could not be unmapped", -1, "", -1);
            return 0;
        }
    }
    pthread_t t;
    void *rc = 0;
    if (pthread_create(&t, 0, worker, 0) != 0 || pthread_join(t, &rc) != 0 || rc != 0) {
        say("worker failed ", (long)rc, "", -1);
        return 1;
    }
    say("node ", target_node, " page ", page_node);
    return 0;
}
