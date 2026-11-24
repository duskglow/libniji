/* TLS-SURPLUS-PROBE helper: one shared object holding one TLS block of SIZE bytes, accessed with
 * the TLS model MODEL ("initial-exec" marks the object DF_STATIC_TLS: its block must be in the
 * static TLS area of every thread; "global-dynamic" lets the loader put it anywhere).
 * Built by run.sh: cc -shared -fPIC -O2 -DSIZE=<n> -DMODEL='"<model>"' tls_obj.c -o <file>.so
 * No spec clause: a black-box measurement probe (audit row HL-33), not a test. */
#ifndef SIZE
#error SIZE
#endif
#ifndef MODEL
#error MODEL
#endif

static __thread char blk[SIZE] __attribute__((tls_model(MODEL)));

/* Write the block's first and last byte in the calling thread and return the block's address:
 * the probe checks that each thread sees a block of its own. */
void *tls_touch(void) {
    blk[0] = 1;
    blk[SIZE - 1] = 2;
    return blk;
}
