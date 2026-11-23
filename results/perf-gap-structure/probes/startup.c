/* PERF-GAP-STRUCTURE-AUDIT start-up probe, one source, four programs chosen at build time:
 * -DP_EMPTY (return 0), -DP_FOPEN (one fopen+fclose), -DP_HELLO (one printf), -DP_SETLOC
 * (setlocale(LC_ALL, "") with the caller's LANG, then one printf). */
#include <fcntl.h>
#include <locale.h>
#include <stdio.h>
#include <unistd.h>

int main(void)
{
#if defined(P_SMAPS) /* end-of-main snapshot of every mapping's resident pages, raw calls only */
    static char b[1 << 20];
    int fd = open("/proc/self/smaps", O_RDONLY);
    long n = 0, r;
    while (fd >= 0 && (r = read(fd, b + n, sizeof b - n)) > 0)
        n += r;
    if (n > 0)
        write(1, b, (size_t)n);
#elif defined(P_FOPEN)
    FILE *f = fopen("/etc/hostname", "r");
    if (f)
        fclose(f);
#elif defined(P_HELLO)
    printf("hello %d\n", 42);
#elif defined(P_SETLOC)
    if (!setlocale(LC_ALL, ""))
        return 3;
    printf("hello %d\n", 42);
#endif
    return 0;
}
