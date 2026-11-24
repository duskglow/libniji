/* s27_dlopen_tls_dynamic: dld_gd's 16 KiB block reached through TLS descriptors (built with
 * -mtls-dialect=gnu2: the descriptor of a block made on demand). */
__thread char dld_block[16 * 1024] = {7, 9};
char *dld_at(void) { return dld_block; }
