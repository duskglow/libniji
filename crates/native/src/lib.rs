//! `niji-native`: the native Rust interface to libniji (implementation-plan.md, D-X.10).
//!
//! A Rust program that runs on libniji can bypass the POSIX translation layer. This crate gives
//! it two things, and has no logic of its own:
//!
//! * **The clean API**, compiled into the program as plain Rust: [`niji`] (text, numbers,
//!   Unicode, calendar, formatting, ...) and [`math`] (`niji-math`). With feature `static` the
//!   public types of `niji-rt` and `niji-kernel` are re-exported too (`rt`, `kernel`).
//!   These are stateless: they hold no process state, so a copy in the program is the same as
//!   the copy in the C library.
//! * **The stateful services** as safe Rust: [`heap`] (the global allocator), [`secure`]
//!   (zeroing that is never optimised away, and [`SecureVec`](secure::SecureVec), a buffer
//!   for secrets that the heap zeroes when it lets go of it), [`env`](mod@env), [`fd`] and
//!   [`fs`], [`clock`], [`tz`], [`locale`] and [`process`]. Each wraps a function of the
//!   native C ABI (`include/niji/native.h`, `spec/abi/native.tsv`, version node
//!   `NIJI_NATIVE_1`) that the program's C library exports, so the program, its C code and
//!   the C library share one instance of every piece of state: one heap, one environment, one
//!   time zone, one set of exit handlers. Rust data structures never cross between separately
//!   compiled objects; only the C ABI's fixed-width integers, byte slices and `#[repr(C)]`
//!   structures do.
//!
//! # Linkage
//!
//! Exactly one of two features is meant for a program's build:
//!
//! * `static`: the program links like a product: statically, with `niji-crt1.o` as its start
//!   object and libniji's `libc.a`, and no host C library. The native functions resolve to
//!   `libc.a`'s definitions.
//! * `dynamic`: the program links against libniji's `libc.so.6` (`DT_NEEDED libc.so.6`) and
//!   references the native functions as `niji_*@NIJI_NATIVE_1`. Running it needs libniji's
//!   dynamic loader (phase 14); the host loader must never load libniji's `libc.so.6`.
//!
//! The crate also builds with neither (the library alone, as `cargo test --workspace` builds
//! it) and with both (`cargo clippy --all-features`); with both, `static` wins. [`LINKAGE`]
//! says which applies. The declarations are the same in both modes: what differs is how the
//! program is linked, which `cargo xtask native` shows with exact commands for both examples
//! (`examples/native_static.rs`, `examples/native_dynamic.rs`).
//!
//! # The Rust dylib
//!
//! Feature `dylib` (which includes `dynamic`) makes the crate the root of the Rust dylib
//! `libniji_rust.so.<abi>`: the crate and the clean API compiled once, by the toolchain of
//! the Rust ABI version `<abi>` (`spec/abi/rust-abi.toml`), with the stateful services calling
//! `libc.so.6`'s native C ABI. A Rust program built by the same toolchain links it instead of
//! compiling the crate in (`--extern niji_native=.../libniji.so`) and records
//! `DT_NEEDED libniji_rust.so.<abi>`. The dylib defines the global allocator and the panic
//! handler, so such a program defines neither (`examples/native_rust_dylib.rs`); [`RUST_ABI`]
//! is the version the program was compiled against and the exported `NIJI_RUST_ABI` that of
//! the library it runs with. `cargo xtask rust-dylib` builds and checks it; docs/toolchain.md
//! gives every parameter and command, docs/interfaces.md when to use which path.
//!
//! # `std` and `no_std` programs
//!
//! The services work in either. Until the C library provides every symbol Rust's `std` needs,
//! the examples are `#![no_std]`, `#![no_main]` programs that name their entry point with
//! [`entry!`] (the C `main` that the start object calls) and set [`heap::Allocator`] as the
//! global allocator.
//!
//! # Conventions
//!
//! * Errors are [`Errno`] (the clean library's error, the kernel's numbering): every native
//!   function's negative result decoded. Results are `Result<T, Errno>`; no wrapper panics.
//! * Byte slices in, byte vectors out: names, values, paths and texts are `&[u8]`, never
//!   NUL-terminated (an interior NUL in a path or stored text is `EINVAL`).
//! * Text out: as an owned `Vec<u8>` (the wrapper sizes the buffer, calling again if the text
//!   grew in between), or into a caller buffer, reported as [`Fit`]: copied whole, or the
//!   length needed (nothing partial is delivered).
//! * Lookups that may find nothing (`env::get`, `process::arg`, ...) give `Ok(None)` for the
//!   C ABI's `ENOENT`.
//! * Allocation is fallible (S8): a buffer the wrapper cannot allocate is `ENOMEM`.
//!
//! # Unsafe code
//!
//! The C ABI's declarations, the [`entry!`] export, the `GlobalAlloc` implementation, the byte
//! views of a secure block that [`SecureVec`](secure::SecureVec) owns and, with
//! feature `dylib`, the attributes that export `NIJI_RUST_ABI` and place the note are the
//! crate's only unsafe code, all in `src/ffi.rs` (category *native* of
//! `cargo xtask unsafe-audit`). Every function there that takes a pointer is private and
//! called through a wrapper taking Rust slices or references, so the rest of the crate,
//! which forbids `unsafe_code` module by module, cannot pass an arbitrary address.

#![cfg_attr(not(test), no_std)]
#![deny(unsafe_code)]
#![cfg_attr(not(test), deny(clippy::disallowed_methods))] // S8: fallible allocation only

extern crate alloc;

mod ffi;
mod buf;

pub mod heap;
pub mod secure;
pub mod env;
pub mod fd;
pub mod fs;
pub mod clock;
pub mod tz;
pub mod locale;
pub mod process;
pub mod nscache;
pub mod aio;
pub mod getpass;
#[cfg(feature = "dylib")]
pub mod dylib;

pub use niji;
pub use niji_math as math;
#[cfg(feature = "static")]
pub use niji_kernel as kernel;
#[cfg(feature = "static")]
pub use niji_rt as rt;

pub use buf::Fit;
pub use niji::error::Errno;
#[cfg(feature = "dylib")]
pub use dylib::RUST_ABI;
#[cfg(feature = "dylib")]
pub use ffi::NIJI_RUST_ABI;

/// How the program reaches the C library (the crate's features, "Linkage" above).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Linkage {
    /// Feature `static`: `libc.a`, linked into the program.
    Static,
    /// Feature `dynamic`: `libc.so.6`, through the `NIJI_NATIVE_1` symbols.
    Dynamic,
    /// Neither feature: the library alone; a program built so has no defined linkage.
    Unspecified,
}

impl Linkage {
    /// `"static"`, `"dynamic"` or `"unspecified"`.
    pub const fn name(self) -> &'static str {
        match self {
            Linkage::Static => "static",
            Linkage::Dynamic => "dynamic",
            Linkage::Unspecified => "unspecified",
        }
    }
}

/// The linkage this build was made for.
#[cfg(feature = "static")]
pub const LINKAGE: Linkage = Linkage::Static;
/// The linkage this build was made for.
#[cfg(all(feature = "dynamic", not(feature = "static")))]
pub const LINKAGE: Linkage = Linkage::Dynamic;
/// The linkage this build was made for.
#[cfg(not(any(feature = "static", feature = "dynamic")))]
pub const LINKAGE: Linkage = Linkage::Unspecified;

/// The newest version node of the native C ABI this crate uses (`NIJI_NATIVE_VERSION`).
pub const NATIVE_VERSION: i32 = 1;
