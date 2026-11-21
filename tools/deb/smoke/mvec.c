/* -lm and the vector maths library (issue #28): built with -O3 -ffast-math against headers that
   declare the simd variants of sinf, the second loop becomes calls of _ZGVbN4v_sinf, which only
   libmvec.so.1 defines. Linked with plain -lm, the libm.so script must bring libmvec.so.1 in
   (AS_NEEDED), and the program prints 0.8415 (sinf(1.0) to four places). Run by
   crates/core/tests/deb_manifest.rs, tools/deb/check.sh, check-system.sh and root-system.sh. */
#include <math.h>
#include <stdio.h>

float a[1024], b[1024];

int main(void)
{
    for (int i = 0; i < 1024; i++)
        a[i] = i * 0.001f;
    for (int i = 0; i < 1024; i++)
        b[i] = sinf(a[i]);
    printf("%.4f\n", b[1000]);
    return 0;
}
