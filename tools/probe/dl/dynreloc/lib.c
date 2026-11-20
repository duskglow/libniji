/* Probe libraries: libdrA (DT_NEEDED of the probe), libdrB (dlopened). */
#include <stdio.h>
static int value;
__attribute__((constructor)) static void init_ctor(void) { value = 40; }
#ifdef TLSDESC
__thread int dr_tls = 2;
int NAME(void) { return value + dr_tls; }
#else
int NAME(void) { return value + 2; }
#endif
