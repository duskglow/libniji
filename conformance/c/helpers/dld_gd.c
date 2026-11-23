/* s27_dlopen_tls_dynamic: 16 KiB of global-dynamic TLS (no DF_STATIC_TLS), an image of two bytes
 * then zeros. The program opens 64 copies of this object at once: 1 MiB, far more than the static
 * TLS surplus, so each block must be made per thread on demand. */
__thread char dld_block[16 * 1024] = {7, 9};
char *dld_at(void) { return dld_block; }
