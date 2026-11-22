/* Helper of x_self_bind (LINK-HELPERS: sbind_lib): a library loaded ahead of libc.so.6 that reaches the
 * exported data `stdout` and `optind` through its own relocations. With the program holding COPY
 * relocations of both, this library and libc.so.6 must see the program's copies. The helpers build as C17
 * (no feature macros by default), and POSIX declares optind in <unistd.h> only for a POSIX program. */
#define _POSIX_C_SOURCE 200809L
#include <stdio.h>
#include <unistd.h>

FILE *sbind_lib_stdout(void) { return stdout; }
int *sbind_lib_optind(void) { return &optind; }
