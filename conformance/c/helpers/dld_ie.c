/* s27_dlopen_tls_dynamic: a small initial-exec block (DF_STATIC_TLS): it needs a place in the
 * static TLS surplus, which the 64 on-demand objects opened before it must not have used. */
__attribute__((tls_model("initial-exec"))) __thread int dld_ie = 5;
int *dld_ie_at(void) { return &dld_ie; }
