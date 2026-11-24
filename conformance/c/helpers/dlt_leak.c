/* s27_dlopen_tls_surplus: 1 KiB of initial-exec TLS (DF_STATIC_TLS: a place in the static TLS
 * surplus) and a call to a function no object defines, so every dlopen with RTLD_NOW fails after
 * the TLS block has been given a place: a failed open must give that place back (80 attempts need
 * more than the static surplus). */
__attribute__((tls_model("initial-exec"))) __thread char dlt_leak_block[1024] = {1};
int dlt_leak_missing(void);
int dlt_leak_use(void) { return dlt_leak_missing() + dlt_leak_block[0]; }
