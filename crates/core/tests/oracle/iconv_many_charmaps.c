/* Many charmaps in one process, then memory exhaustion (iconv_charmaps_products.rs).
 *
 * usage: iconv_many_charmaps <first> <more>
 * The charmaps directory holds MANY-0 .. MANY-<first+more-1>; MANY-k maps byte 0x41 to U+0100+k.
 * 1. Opens MANY-0 .. MANY-<first-1> from UTF-8, all kept open, each converting "A" correctly.
 * 2. Limits the address space (RLIMIT_AS) to the process's size plus 256 KiB and opens the
 *    next ones until one fails: that failure must be ENOMEM, and one must fail.
 * 3. Lifts the limit: every descriptor opened so far still converts correctly, and the name
 *    that failed now opens and converts (nothing half-loaded was kept).
 * Prints "ok first=N more_opened=K errno=ENOMEM" and exits 0, else a reason on stderr and exits 1.
 */
#include <errno.h>
#include <fcntl.h>
#include <iconv.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/resource.h>
#include <unistd.h>

#define MAX 4096
static iconv_t cds[MAX];

static int converts(iconv_t cd, int k) {
    char in[1] = {'A'}, out[8];
    char *ip = in, *op = out;
    size_t il = 1, ol = sizeof out;
    if (iconv(cd, &ip, &il, &op, &ol) == (size_t)-1 || op - out != 2) return 0;
    unsigned cp = 0x100u + (unsigned)k;
    return (unsigned char)out[0] == (0xC0u | (cp >> 6)) && (unsigned char)out[1] == (0x80u | (cp & 0x3Fu));
}

static iconv_t open_k(int k) {
    char name[32];
    snprintf(name, sizeof name, "MANY-%d", k);
    return iconv_open("UTF-8", name);
}

/* The process's virtual size in bytes, from /proc/self/statm (first field, pages). */
static unsigned long vm_bytes(void) {
    char buf[128];
    int fd = open("/proc/self/statm", O_RDONLY);
    if (fd < 0) return 0;
    ssize_t n = read(fd, buf, sizeof buf - 1);
    close(fd);
    if (n <= 0) return 0;
    buf[n] = 0;
    return strtoul(buf, 0, 10) * (unsigned long)sysconf(_SC_PAGESIZE);
}

int main(int argc, char **argv) {
    if (argc != 3) return 2;
    int first = atoi(argv[1]), more = atoi(argv[2]);
    if (first <= 0 || more <= 0 || first + more > MAX) return 2;
    for (int k = 0; k < first; k++) {
        cds[k] = open_k(k);
        if (cds[k] == (iconv_t)-1) { fprintf(stderr, "open MANY-%d failed: errno %d\n", k, errno); return 1; }
        if (!converts(cds[k], k)) { fprintf(stderr, "MANY-%d converts wrongly\n", k); return 1; }
    }
    struct rlimit old;
    if (getrlimit(RLIMIT_AS, &old) != 0) { fprintf(stderr, "getrlimit failed\n"); return 1; }
    unsigned long vm = vm_bytes();
    if (vm == 0) { fprintf(stderr, "no /proc/self/statm\n"); return 1; }
    struct rlimit lim = old;
    lim.rlim_cur = vm + (256ul << 10);
    if (setrlimit(RLIMIT_AS, &lim) != 0) { fprintf(stderr, "setrlimit failed\n"); return 1; }
    int k = first, err = 0;
    for (; k < first + more; k++) {
        cds[k] = open_k(k);
        if (cds[k] == (iconv_t)-1) { err = errno; break; }
    }
    if (setrlimit(RLIMIT_AS, &old) != 0) { fprintf(stderr, "restoring the limit failed\n"); return 1; }
    if (k == first + more) { fprintf(stderr, "no open failed under the limit (%d more opened)\n", more); return 1; }
    if (err != ENOMEM) { fprintf(stderr, "open MANY-%d under the limit failed with errno %d, not ENOMEM\n", k, err); return 1; }
    for (int j = 0; j < k; j++)
        if (!converts(cds[j], j)) { fprintf(stderr, "after the failure MANY-%d converts wrongly\n", j); return 1; }
    iconv_t again = open_k(k);
    if (again == (iconv_t)-1) { fprintf(stderr, "MANY-%d does not open after the limit is lifted: errno %d\n", k, errno); return 1; }
    if (!converts(again, k)) { fprintf(stderr, "MANY-%d converts wrongly after the limit is lifted\n", k); return 1; }
    printf("ok first=%d more_opened=%d errno=ENOMEM\n", first, k - first);
    return 0;
}
