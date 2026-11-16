//! Product crate for `libc.so.6` and `libc.a` (implementation-plan.md §9.4).
//!
//! It links `niji-boundary` with feature `runtime` (which implies `export`), so every
//! translation function is emitted under its C name, together with the C syscall layer
//! (`csys/`) that `niji-boundary` compiles, and the process runtime (`__libc_start_main`,
//! `exit`, `environ`, ...). This crate adds only what a `no_std` product must name once: the
//! global allocator (libniji's heap, so Rust code inside the library allocates from it) and the
//! panic handler (a panic is a library bug: report the location and abort).
//!
//! `cargo xtask so` builds it twice, as a static archive and as a shared object with the version
//! script generated from `spec/abi/versions.tsv` (spec Appendix C.3).

#![no_std]
#![forbid(unsafe_code)]

pub use niji_boundary as _boundary;

#[global_allocator]
static ALLOCATOR: niji_boundary::heap::libniji = niji_boundary::heap::libniji;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo<'_>) -> ! {
    niji_boundary::process::panicked(info)
}
