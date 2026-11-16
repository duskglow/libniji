//! The shipped `ldconfig` (UTIL-3, spec §31.2, D-31.2): `niji_boundary::ldconfig` linked with the
//! `ldconfig` variant of `machine/entry.S` into a static PIE whose entry point is
//! `niji_machine_ldconfig_entry`; `cargo xtask so` builds it into `target/niji/ldconfig` and
//! checks it needs no interpreter and no shared object.
#![no_std]
#![forbid(unsafe_code)]

pub use niji_boundary as _boundary;

#[global_allocator]
static ALLOCATOR: niji_boundary::ldconfig::Arena = niji_boundary::ldconfig::Arena;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo<'_>) -> ! {
    niji_boundary::ldconfig::panicked(info)
}
