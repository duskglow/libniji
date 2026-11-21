/* validation/smoke/selftest/tls_plugin.c: a shared object with a thread-local variable (smoke.sh selftest).
   Built with -fPIC it reaches the variable through the loader's __tls_get_addr, so its DT_NEEDED names the
   loader by the loader's own soname; check-elf.sh must accept that (the program interpreter itself). */
static __thread int tls_counter;

int tls_plugin_bump(void) { return ++tls_counter; }
