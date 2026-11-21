/* The personality routine's stand-in for a panic=abort Rust program linked against libniji's
 * libc.so.6 (crates/native, D-X.10; compiled and linked by `cargo xtask native`).
 *
 * Rust's precompiled `core` and `alloc` are built for unwinding: their call-frame information
 * names the personality routine `rust_eh_personality`. The unwinder itself comes from libgcc_eh
 * (its `dl_iterate_phdr` and `_dl_find_object` from libniji); libniji's libc.so.6 exports no
 * personality routine (libc.a defines one for its own Rust code, so a static link needs none).
 * A panic=abort program never unwinds, so the routine is never called.
 *
 * It is weak, so Rust's `std` (which defines the personality) takes its place, and it aborts if
 * it is ever reached. */
extern void abort(void) __attribute__((__noreturn__));

void rust_eh_personality(void) __attribute__((__weak__));

void rust_eh_personality(void)
{
    abort();
}
