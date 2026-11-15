//! The shipped `ldconfig` (UTIL-3, spec §31.2, D-31.2): `liella_boundary::ldconfig` linked with the
//! `ldconfig` variant of `machine/entry.S` into a static PIE whose entry point is
//! `liella_machine_ldconfig_entry`; `cargo xtask so` builds it into `target/liella/ldconfig` and
//! checks it needs no interpreter and no shared object.
#![no_std]
#![forbid(unsafe_code)]

pub use liella_boundary as _boundary;

#[global_allocator]
static ALLOCATOR: liella_boundary::ldconfig::Arena = liella_boundary::ldconfig::Arena;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo<'_>) -> ! {
    liella_boundary::ldconfig::panicked(info)
}
