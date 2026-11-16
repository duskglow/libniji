/* Class adapter (OD-C1): the libm product's load-time constructor (D-10.50, #2920). libm.a and
 * libm.so.6 carry their own copy of the dispatch byte that chooses the FMA builds of the double
 * exp/log fast paths; the C library's startup never writes that copy, so this `.init_array` entry
 * has `niji_libm_start` (crates/boundary/src/dispatch.rs) decide it once, before the program's
 * code calls a math function, and no math call asks whether it has been decided. The loader (and
 * a static program's startup, `run_array`) calls it with `(argc, argv, envp)`. Linked only into
 * the libm product (feature `libm`, build.rs); the name is local in both libraries. */
#include "niji_csys.h"

#if defined(NIJI_LIBM)
void niji_libm_start(int argc, char **argv, char **envp);

__attribute__((used, section(".init_array")))
static void (*const niji_libm_init_entry)(int, char **, char **) = niji_libm_start;
#endif
