/* Spec §20.7 (s20_7_i_0): under _GNU_SOURCE MINSIGSTKSZ and SIGSTKSZ are not constants but
 * sysconf (_SC_MINSIGSTKSZ) and sysconf (_SC_SIGSTKSZ); <signal.h> alone makes the expansion
 * usable (the uses come before any other header). The values are sysconf's, at least the
 * historical constants (2048, 8192), and an alternate stack of SIGSTKSZ bytes is accepted.
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
    if (min != sysconf(_SC_MINSIGSTKSZ) || size != sysconf(_SC_SIGSTKSZ)) {
        fprintf(stderr, "s20_sigstksz_dynamic: not sysconf's values: %ld %ld\n", min, size);
        return 1;
    }
    if (min < 2048 || size < 8192 || size < min) {
        fprintf(stderr, "s20_sigstksz_dynamic: below the constants: %ld %ld\n", min, size);
        return 1;
    }
    stack_t ss = { 0 };
    ss.ss_sp = malloc((size_t) size);
    ss.ss_size = (size_t) size;
    if (ss.ss_sp == NULL || sigaltstack(&ss, NULL) != 0) {
        fprintf(stderr, "s20_sigstksz_dynamic: sigaltstack of SIGSTKSZ bytes failed\n");
        return 1;
    }
    return 0;
}
