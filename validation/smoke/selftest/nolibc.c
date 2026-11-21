/* validation/smoke/selftest/nolibc.c: a negative control of smoke.sh selftest. Linked by niji-cc without
   start files and with --as-needed, it has libniji's loader as PT_INTERP but names no C library at all, which
   check-elf.sh must reject (a program on libniji's loader must use libniji's libc). Never run. */
void _start(void) { __builtin_trap(); }
