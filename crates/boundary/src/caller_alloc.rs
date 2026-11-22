//! Category *extern*: the interposable C services the translation layer calls by symbol
//! (finding F23, decision D-2.1): caller-owned memory through `malloc`/`calloc`/`realloc`/`free`,
//! and the calling thread's `errno` cell through `__errno_location`.
//!
//! In products these symbols resolve to libniji's own exports (phases 3–4), or to a program's own
//! malloc family when it replaces it (D-X.14, "Replacing malloc"); in hosted test builds they
//! resolve to the running C library. Either way C callers and the library agree on one allocator
//! and one `errno`. The allocator is reached through the C layer's forwarders
//! (`niji_caller_malloc`, ...: one call each to the symbol), never directly: within the
//! library's own optimised code a call to `malloc` could be bound to, inlined from or merged with
//! the library's definition, and a program's replacement would then be bypassed.

use core::ffi::c_void;

use niji_kernel::errno::{Errno, KResult};

use crate::conv;

// Soundness: `malloc`, `calloc` and `__errno_location` are sound for every argument (they
// return fresh memory, NULL, or the thread's errno cell). `realloc` and `free` take a pointer
// the caller vouches for: they are crate-private and called only with pointers that a C
// contract says came from this allocator (documented at each call).
//
// UNSAFE(extern): Rust cannot call C library functions without declaring them.
#[allow(unsafe_code)]
unsafe extern "C" {
    #[link_name = "niji_caller_malloc"]
    safe fn c_malloc(n: usize) -> *mut c_void;
    #[link_name = "niji_caller_calloc"]
    safe fn c_calloc(n: usize, size: usize) -> *mut c_void;
    /// Obligation: `p` is NULL or came from this allocator and is not used afterwards.
    #[link_name = "niji_caller_realloc"]
    safe fn c_realloc(p: *mut c_void, n: usize) -> *mut c_void;
    /// Obligation: `p` is NULL or came from this allocator and is not used afterwards.
    #[link_name = "niji_caller_free"]
    safe fn c_free(p: *mut c_void);
    pub(crate) safe fn __errno_location() -> *mut i32;
    safe fn niji_link_family_replaced() -> core::ffi::c_int;
}

// ------------------------------------------------------------------------------ internal routing

/// Where the library's own allocations (Rust's global allocator in the products, `heap::Niji`)
/// go (ruling #7583, INTERNAL-ALLOC-ROUTING): [`PRIVATE`] (libniji's heap), [`ROUTED`] (every
/// block at most [`niji_rt::heap::ALIGN`]-aligned through the symbols, as caller memory is) or
/// [`ROUTED_CHECKED`] (the same, but the heap already held blocks of the library's own when the
/// route was decided, so a release asks the heap whose block it is first). Decided once, by
/// [`decide_route`], before any constructor; never changed afterwards.
static ROUTE: core::sync::atomic::AtomicU8 = core::sync::atomic::AtomicU8::new(PRIVATE);
pub(crate) const PRIVATE: u8 = 0;
pub(crate) const ROUTED: u8 = 1;
pub(crate) const ROUTED_CHECKED: u8 = 2;

/// [`ROUTE`]'s value: one load, [`PRIVATE`] unless the family was replaced.
#[inline(always)]
pub(crate) fn route() -> u8 {
    ROUTE.load(core::sync::atomic::Ordering::Relaxed)
}

/// Decide [`ROUTE`] (libniji's `ld.so` calls `early_init` after relocation, before any
/// constructor): when `malloc`, `calloc`, `realloc` or `free` resolves outside libc.so.6 (a
/// program, an `LD_PRELOAD` library or a tool replaced the family, "Replacing malloc"), the
/// library's own allocations go through the replacement from here on, as the reference's do.
/// The loader's own data (`ld.so` keeps no heap) and the blocks of an aligned allocation above
/// the replacement's guarantee stay private (docs/differences-from-glibc.md). Cold: a start
/// without a replacement pays the call and one comparison loop.
#[cold]
#[inline(never)]
#[cfg_attr(not(feature = "runtime"), allow(dead_code))]
pub(crate) fn decide_route() {
    if niji_link_family_replaced() == 0 {
        return;
    }
    let mode = if crate::heap::HEAP.untouched() { ROUTED } else { ROUTED_CHECKED };
    ROUTE.store(mode, core::sync::atomic::Ordering::Release);
}


/// Count `d` caller-owned blocks more (or fewer) live on the calling thread when `yes`: the
/// test builds' leak measure (`selftest::caller_blocks`, FREE-POISON-TESTS-6 ruling #296), valid
/// whichever allocator serves the symbols. Products compile it away.
#[inline(always)]
fn counted(yes: bool, d: i64) {
    #[cfg(feature = "selftest")]
    if yes {
        crate::selftest::count_caller(d);
    }
    #[cfg(not(feature = "selftest"))]
    let _ = (yes, d);
}

/// `malloc(n)` through the symbol.
#[allow(dead_code)] // the §3 exports (phase 3)
#[inline(always)]
pub(crate) fn malloc(n: usize) -> *mut c_void {
    let p = c_malloc(n);
    counted(!p.is_null(), 1);
    p
}

/// `calloc(n, size)` through the symbol.
#[inline(always)]
pub(crate) fn calloc(n: usize, size: usize) -> *mut c_void {
    let p = c_calloc(n, size);
    counted(!p.is_null(), 1);
    p
}

/// `realloc(p, n)` through the symbol. Obligation: `p` is NULL or came from this allocator and
/// is not used afterwards.
#[allow(dead_code)] // the §3 exports (phase 3)
#[inline(always)]
pub(crate) fn realloc(p: *mut c_void, n: usize) -> *mut c_void {
    let q = c_realloc(p, n);
    // A new block from NULL; `realloc(p, 0)` returning NULL released `p`.
    counted(p.is_null() && !q.is_null(), 1);
    counted(!p.is_null() && n == 0 && q.is_null(), -1);
    q
}

/// `free(p)` through the symbol. Obligation: `p` is NULL or came from this allocator and is not
/// used afterwards.
#[inline(always)]
pub(crate) fn free(p: *mut c_void) {
    counted(!p.is_null(), -1);
    c_free(p);
}

/// A new caller-owned copy of `text` plus a terminating zero unit (`strdup`, `wcsdup`): its
/// address, or `ENOMEM`.
///
/// The copy is `malloc`ed (§4.10: "a new `malloc`ed copy"), not `calloc`ed: every unit is written
/// here, so zeroing it first cost ~42 instructions per `strdup` (STRDUP-FAST).
pub(crate) fn dup_z<T: Copy + Default>(text: &[T]) -> KResult<usize> {
    let k = text.len();
    // `text` is a slice, so `k * size_of::<T>() <= isize::MAX`: `(k + 1) * size_of::<T>()` cannot wrap
    // (it is at most `isize::MAX + size_of::<T>()`). A size above `isize::MAX` is refused by `malloc`
    // itself (NULL, `ENOMEM`), so no separate check is made here (STRDUP-FAST-2: ~2 instructions).
    let bytes = k.wrapping_add(1).wrapping_mul(size_of::<T>());
    let p = malloc(bytes).cast::<T>();
    if p.is_null() {
        return Err(Errno::ENOMEM);
    }
    conv::fresh_copy_z(p, text);
    Ok(p.expose_provenance())
}

/// A new caller-owned copy of `bytes` (no terminator added); its address, or `ENOMEM`.
pub(crate) fn dup_bytes(bytes: &[u8]) -> KResult<usize> {
    let p = calloc(bytes.len().max(1), 1);
    let dst = conv::fresh::<u8>(p, bytes.len().max(1)).ok_or(Errno::ENOMEM)?;
    for (d, s) in dst.iter_mut().zip(bytes) {
        *d = *s;
    }
    Ok(p.expose_provenance())
}

/// `reallocarray(p, k, n)`: `realloc(p, k * n)` through the (replaceable) `realloc` symbol, as
/// glibc's manual describes it ("Replacing malloc", D-X.14); `ENOMEM` when the product
/// overflows. A size of 0 is `realloc`'s (the block freed, NULL without an error). Obligation (on
/// the C caller): `addr` is 0 or came from this allocator.
/// Without the exported symbols (hosted test builds, where `realloc` is the running C library's
/// while `free` and `malloc_usable_size` under test are libniji's), the heap's own path.
pub(crate) fn reallocarray(addr: usize, k: usize, n: usize) -> KResult<usize> {
    #[cfg(not(feature = "export"))]
    {
        let r = crate::heap::checked("realloc", crate::heap::c_reallocarray(addr, k, n));
        if let Ok(q) = r {
            counted(addr == 0 && q != 0, 1);
            counted(addr != 0 && q == 0, -1);
        }
        r
    }
    #[cfg(feature = "export")]
    {
        reallocarray_by_symbol(addr, k, n)
    }
}

#[cfg(feature = "export")]
fn reallocarray_by_symbol(addr: usize, k: usize, n: usize) -> KResult<usize> {
    let total = k.checked_mul(n).ok_or(Errno::ENOMEM)?;
    let q = c_realloc(core::ptr::with_exposed_provenance_mut(addr), total);
    counted(addr == 0 && !q.is_null(), 1);
    counted(addr != 0 && total == 0 && q.is_null(), -1);
    if q.is_null() && total != 0 {
        return Err(Errno::ENOMEM);
    }
    Ok(q.expose_provenance())
}

/// `value` in a new block of the caller's allocator (`malloc(size_of::<T>())` through the
/// symbol): an object the library makes on the program's behalf (a stream, ...), which a program
/// that replaces `malloc` sees made and, through [`release`], freed (ruling #7583). The object, or
/// `ENOMEM` (also when a replacement returns NULL without setting `errno`). It is never dropped:
/// its block goes back with [`release`].
#[inline(always)]
pub(crate) fn new_value<T>(value: T) -> KResult<&'static mut T> {
    conv::fresh_value(malloc(size_of::<T>()), value).ok_or(Errno::ENOMEM)
}

/// Release a caller-owned block. Obligation (on the caller of this function): `addr` is 0 or
/// came from this allocator and is not used afterwards.
pub(crate) fn release(addr: usize) {
    free(core::ptr::with_exposed_provenance_mut(addr));
}

// ------------------------------------------------------------------------------ libresolv.so.2
//
// The libresolv product (LIBRESOLV-OWN-CODE) reaches the C library's state only through
// libc.so.6's exports: `fwrite`, `stdout`, `strlen` and `strnlen` (the standard interface), and
// two private entries (`NIJI_PRIVATE`) for what has no standard one: the thread's result blocks
// and `_res.pfcode`. The calls that need `unsafe` are made where the category allows them:
// `strz::byte_len` (the length scanners), `posix::s16_nameservices` (`fwrite`, `stdout`); the
// product's global allocator is `heap::LibresolvAlloc`.

// Soundness: `__niji_resolv_block` and `__niji_resolv_pfcode` are sound for every argument (a
// block address or 0; a number). `fwrite` reads `n` bytes at `p` and takes a stream the C caller
// vouches for; `strlen`/`strnlen` read a string the caller vouches for; `stdout` is the C
// library's object, written only by the C library (a pointer-sized word).
#[cfg(all(feature = "libresolv", not(feature = "runtime")))]
// UNSAFE(extern): Rust cannot call C library functions or read its objects without declaring them.
#[allow(unsafe_code)]
unsafe extern "C" {
    safe fn __niji_resolv_block(kind: usize, need: usize) -> usize;
    safe fn __niji_resolv_pfcode() -> u64;
    /// Obligation: `p` holds `size * n` readable bytes; `fp` is a valid stream.
    pub(crate) fn fwrite(p: *const c_void, size: usize, n: usize, fp: *mut c_void) -> usize;
    /// Written only by the C library.
    pub(crate) static stdout: usize;
    /// Obligation: `s` is readable up to and including a NUL.
    pub(crate) fn strlen(s: *const u8) -> usize;
    /// Obligation: `s` is readable up to a NUL or `max` bytes.
    pub(crate) fn strnlen(s: *const u8, max: usize) -> usize;
}

/// The calling thread's result block of `kind` (`need` bytes): its address, 0 without memory.
#[cfg(all(feature = "libresolv", not(feature = "runtime")))]
pub(crate) fn libc_block(kind: usize, need: usize) -> usize {
    __niji_resolv_block(kind, need)
}

/// The calling thread's `_res.pfcode`.
#[cfg(all(feature = "libresolv", not(feature = "runtime")))]
pub(crate) fn libc_pfcode() -> u64 {
    __niji_resolv_pfcode()
}

/// `malloc(n)` for the libresolv product's global allocator (`heap::LibresolvAlloc`): through the symbol.
#[cfg(feature = "libresolv")]
pub(crate) fn caller_malloc(n: usize) -> *mut c_void {
    c_malloc(n)
}

/// `calloc(n, size)` for `heap::LibresolvAlloc`.
#[cfg(feature = "libresolv")]
pub(crate) fn caller_calloc(n: usize, size: usize) -> *mut c_void {
    c_calloc(n, size)
}

/// `realloc(p, n)` for `heap::LibresolvAlloc`. Obligation: `p` is NULL or came from this allocator and is
/// not used afterwards.
#[cfg(feature = "libresolv")]
pub(crate) fn caller_realloc(p: *mut c_void, n: usize) -> *mut c_void {
    c_realloc(p, n)
}

/// `free(p)` for `heap::LibresolvAlloc`. Obligation: `p` is NULL or came from this allocator and is not
/// used afterwards.
#[cfg(feature = "libresolv")]
pub(crate) fn caller_free(p: *mut c_void) {
    c_free(p);
}
