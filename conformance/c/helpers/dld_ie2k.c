/* s27_dlopen_tls_dynamic: 2 KiB of initial-exec TLS (DF_STATIC_TLS), more than the default static
 * TLS room for dlopened objects (glibc.rtld.optional_static_tls 512 + 1200 = 1712 bytes), so its
 * dlopen fails with "cannot allocate memory in static TLS block". */
__attribute__((tls_model("initial-exec"))) __thread char dld_ie2k_block[2048];
char *dld_ie2k_at(void) { return dld_ie2k_block; }
