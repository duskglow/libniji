//! Product crate for libresolv.so.2's own code (spec §16.4, LIBRESOLV-OWN-CODE).
//!
//! It links `niji-boundary` with feature `libresolv` only, so the libresolv.so.2 names marked
//! `export! { libc_libresolv ... }` are emitted under their C names and nothing of libc is. The
//! code reaches the C library's state only through libc.so.6's exports: `errno`
//! (`__errno_location`), memory (`malloc` family, also Rust's global allocator here), `fwrite` and
//! `stdout`, and the private `__niji_resolv_block` / `__niji_resolv_pfcode` (`NIJI_PRIVATE`).
//! A process that never loads libresolv.so.2 carries none of it.
//!
//! `cargo xtask so` links the static library into one relocatable object (roots: the exports),
//! checks that it defines no state of its own beyond the exported objects, and links
//! libresolv.so.2 from it. libc.a keeps its own copy of these names (D-16.6: a static `-lresolv`).

#![no_std]
#![forbid(unsafe_code)]

pub use niji_boundary as _boundary;

#[global_allocator]
static ALLOCATOR: niji_boundary::heap::Caller = niji_boundary::heap::Caller;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    niji_boundary::process::libresolv_panicked()
}
