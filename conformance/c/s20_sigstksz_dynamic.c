/* Spec §20.7 (s20_7_i_0 and its erratum, ERRATA.md E14; D-20.10): under _GNU_SOURCE MINSIGSTKSZ and
 * SIGSTKSZ are not constants; both are sysconf (_SC_SIGSTKSZ), as with the reference (probed: both 14528 on the
 * Skylake-SP host, where sysconf (_SC_MINSIGSTKSZ) is 3632; both 8192 on the Zen 1 host, 1776). <signal.h> alone makes the expansion usable (the uses come
 * before any other header). Both are at least the historical constants (2048, 8192), and the kernel accepts an
 * alternate stack of MINSIGSTKSZ bytes.
 * Built against libniji's own headers too (run.sh --niji-headers). */
#define _GNU_SOURCE 1
#include <signal.h>
static long min_size(void) { return MINSIGSTKSZ; }
static long stack_size(void) { return SIGSTKSZ; }
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
int main(void)
{
    long min = min_size(), size = stack_size();
    printf("MINSIGSTKSZ %ld SIGSTKSZ %ld\n", min, size);
    if (min != sysconf(_SC_SIGSTKSZ) || size != sysconf(_SC_SIGSTKSZ) || min < sysconf(_SC_MINSIGSTKSZ)) {
        fprintf(stderr, "s20_sigstksz_dynamic: not sysconf's values: %ld %ld\n", min, size);
        return 1;
    }
    if (min < 2048 || size < 8192 || size < min) {
        fprintf(stderr, "s20_sigstksz_dynamic: below the constants: %ld %ld\n", min, size);
        return 1;
    }
    stack_t ss = { 0 };
    ss.ss_sp = malloc((size_t) min);
    ss.ss_size = (size_t) min;
    if (ss.ss_sp == NULL || sigaltstack(&ss, NULL) != 0) {
        fprintf(stderr, "s20_sigstksz_dynamic: sigaltstack of MINSIGSTKSZ bytes failed\n");
        return 1;
    }
    return 0;
}
