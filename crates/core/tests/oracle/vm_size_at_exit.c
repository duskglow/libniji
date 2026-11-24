/* The process's address space at exit (HEAP-FIND-DEPTH-NEED-3, D-3.25): a shared object preloaded
 * into a program (the loader's --preload) whose destructor copies the "VmSize:" line of
 * /proc/self/status to file descriptor 3, which the caller opens (`find` closes its standard error
 * at exit, before destructors run). heap_meta_products takes it as the address space a walk
 * maps without a limit: the heap keeps its segments to the end, so at exit that is everything the
 * walk left mapped, without the over-mapping it used and returned on the way.
 * Freestanding (raw system calls, no headers, built with -nostdlib): it depends on no C library,
 * so it neither links against nor calls into the one under test. */

static long sys3(long n, long a, long b, long c) {
    long r;
    __asm__ volatile("syscall" : "=a"(r) : "a"(n), "D"(a), "S"(b), "d"(c) : "rcx", "r11", "memory");
    return r;
}

__attribute__((destructor)) static void vm_size_at_exit(void) {
    static char buf[8192];
    static const char path[] = "/proc/self/status";
    static const char key[] = "\nVmSize:";
    long fd = sys3(2 /* open */, (long)path, 0 /* O_RDONLY */, 0);
    if (fd < 0)
        return;
    long n = 0;
    for (;;) {
        long r = sys3(0 /* read */, fd, (long)(buf + n), (long)(sizeof buf - 1) - n);
        if (r <= 0)
            break;
        n += r;
    }
    sys3(3 /* close */, fd, 0, 0);
    for (long i = 0; i + (long)(sizeof key - 1) <= n; i++) {
        long k = 0;
        while (k < (long)(sizeof key - 1) && buf[i + k] == key[k])
            k++;
        if (k < (long)(sizeof key - 1))
            continue;
        long end = i + 1;
        while (end < n && buf[end] != '\n')
            end++;
        if (end < n)
            end++;
        sys3(1 /* write */, 3, (long)(buf + i + 1), end - i - 1);
        return;
    }
}
