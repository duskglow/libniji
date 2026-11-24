/* s27_dlopen_tls_dynamic: 1 GiB of global-dynamic .tbss. Opening it needs no memory for the
 * block; the first access in a thread does, and under a 512 MiB address-space limit it cannot
 * be had. */
__thread char dld_huge_block[1L << 30];
char *dld_huge_at(void) { return dld_huge_block; }
