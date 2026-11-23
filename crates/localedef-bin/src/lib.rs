//! The shipped `localedef` (UTIL-12, spec §31.8): `niji_boundary::localedef` linked with the
//! `localedef` variant of `machine/entry.S` into a static PIE whose entry point is
//! `niji_machine_localedef_entry`; `cargo xtask so` builds it into `target/niji/localedef` and
//! checks it needs no interpreter and no shared object.
#![no_std]
#![forbid(unsafe_code)]

pub use niji_boundary as _boundary;

#[global_allocator]
static ALLOCATOR: niji_boundary::ldconfig::Arena = niji_boundary::ldconfig::Arena;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo<'_>) -> ! {
    niji_boundary::localedef::panicked(info)
}
