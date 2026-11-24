/* Spec §2.3 (s2_3_c_2, s2_3_c_3): `error` and `error_at_line` exit only when the status is not 0,
 * so a call with status 0 returns to its caller. Built against libniji's own <error.h> at -O2: a
 * declaration that told the compiler these functions never return would let it drop the code after
 * a status-0 call (the stores below and the final exit status), which this program then misses.
 * Each call also takes a status known only at run time (0 here), which must return as well. */
/* NIJI-HEADERS: yes */
/* OPT: -O2 */
/* EXPECT-STDERR: s02_error_returns: zero 1 */
/* EXPECT-STDERR: s02_error_returns:f.c:7: zero 2 */
/* EXPECT-STDERR: s02_error_returns: variable 3 */
/* EXPECT-STDERR: s02_error_returns:g.c:9: variable 4 */
/* EXPECT-STDERR: after 15 */
#define _GNU_SOURCE
#include <error.h>
#include <stdio.h>
#include <stdlib.h>

/* Not foldable: the compiler cannot know it is 0. */
volatile int run_status;
static volatile int after;

int main(void)
{
    error(0, 0, "zero %d", 1);
    after |= 1;
    error_at_line(0, 0, "f.c", 7, "zero %d", 2);
    after |= 2;
    error(run_status, 0, "variable %d", 3);
    after |= 4;
    error_at_line(run_status, 0, "g.c", 9, "variable %d", 4);
    after |= 8;
    fprintf(stderr, "after %d\n", after);
    return after == 15 ? 0 : 1;
}
