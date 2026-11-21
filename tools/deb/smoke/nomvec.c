/* -lm without the vector maths library (issue #28): one scalar sin call on a volatile operand, so
   nothing is vectorised or folded. Linked with -lm (and --no-as-needed, so the scalar library is
   recorded even by a linker that defaults to --as-needed), the program needs libm.so.6 and NOT
   libmvec.so.1: the libm.so script names libmvec.so.1 AS_NEEDED. Prints "nomvec: sin(1) 0.8415".
   Run by crates/core/tests/deb_manifest.rs, tools/deb/check.sh, check-system.sh and root-system.sh. */
#include <math.h>
#include <stdio.h>

int main(void)
{
    volatile double x = 1.0;
    printf("nomvec: sin(1) %.4f\n", sin(x));
    return 0;
}
