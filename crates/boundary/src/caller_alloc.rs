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

/// Release a caller-owned block. Obligation (on the caller of this function): `addr` is 0 or
/// came from this allocator and is not used afterwards.
pub(crate) fn release(addr: usize) {
    free(core::ptr::with_exposed_provenance_mut(addr));
}

// ------------------------------------------------------------------------------ libresolv.so.2
//
// The libresolv product (LIBRESOLV-OWN-CODE) reaches the C library's state only through
// libc.so.6's exports: `fwrite` and `stdout` (the standard interface), and two private entries
// (`NIJI_PRIVATE`) for what has no standard one: the thread's result blocks and `_res.pfcode`.

// Soundness: `__niji_resolv_block` and `__niji_resolv_pfcode` are sound for every argument (a
// block address or 0; a number). `fwrite` reads `n` bytes at `p` (the caller passes a live
// slice) and takes a stream the C caller vouches for. `stdout` is the C library's object, written
// only by the C library (a pointer-sized word).
#[cfg(all(feature = "libresolv", not(feature = "runtime")))]
// UNSAFE(extern): Rust cannot call C library functions or read its objects without declaring them.
#[allow(unsafe_code)]
unsafe extern "C" {
    safe fn __niji_resolv_block(kind: usize, need: usize) -> usize;
    safe fn __niji_resolv_pfcode() -> u64;
    fn fwrite(p: *const c_void, size: usize, n: usize, fp: *mut c_void) -> usize;
    static stdout: usize;
    fn strlen(s: *const u8) -> usize;
    fn strnlen(s: *const u8, max: usize) -> usize;
}

/// Bytes before the first NUL at `p`, at most `limit` (`usize::MAX`: unbounded), measured by the
/// C library's `strlen`/`strnlen`.
///
/// # Safety
/// `p` is non-NULL and points to readable bytes up to and including a NUL, or `limit` bytes.
#[cfg(all(feature = "libresolv", not(feature = "runtime")))]
// UNSAFE(extern): calling the C library's length scanners on a caller-vouched pointer.
#[allow(unsafe_code)]
pub(crate) unsafe fn libc_len(p: *const u8, limit: usize) -> usize {
    // UNSAFE(extern): the C library's `strlen`/`strnlen`.
    // SAFETY: the function's contract (readable up to a NUL or `limit` bytes) is theirs.
    unsafe { if limit == usize::MAX { strlen(p) } else { strnlen(p, limit) } }
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

/// Write `t` to the stream at `fp` (a stream the C caller passed).
#[cfg(all(feature = "libresolv", not(feature = "runtime")))]
pub(crate) fn libc_fwrite(t: &[u8], fp: usize) {
    if t.is_empty() {
        return;
    }
    // UNSAFE(extern): the C library's `fwrite`.
    #[allow(unsafe_code)]
    // SAFETY: `t` is a live slice of `t.len()` bytes; `fp` is the stream the C caller named
    // (its contract: a valid `FILE *`).
    let _ = unsafe { fwrite(t.as_ptr().cast(), 1, t.len(), core::ptr::with_exposed_provenance_mut(fp)) };
}

/// The address of the standard output stream (`stdout`'s value).
#[cfg(all(feature = "libresolv", not(feature = "runtime")))]
pub(crate) fn libc_stdout() -> usize {
    // UNSAFE(extern): reading the C library's `stdout` object.
    #[allow(unsafe_code)]
    // SAFETY: `stdout` is a pointer-sized object the C library defines and keeps valid.
    unsafe { core::ptr::read_volatile(core::ptr::addr_of!(stdout)) }
}

/// Rust's global allocator in the libresolv product: the caller allocator (`malloc` through the
/// symbol), so libresolv.so.2 holds no heap of its own. Blocks are at most 16-aligned (`malloc`'s
/// alignment); a larger alignment fails (null), which the fallible containers report as `ENOMEM`.
#[cfg(feature = "libresolv")]
pub struct Caller;

// Soundness: blocks come from `malloc`/`calloc`/`realloc`, which never hand out overlapping live
// blocks and honour sizes up to the 16-byte alignment checked here; null on exhaustion.
#[cfg(feature = "libresolv")]
// UNSAFE(allocator): implementing `GlobalAlloc` for the libresolv product.
#[allow(unsafe_code)]
unsafe impl core::alloc::GlobalAlloc for Caller {
    // UNSAFE(allocator): the trait's required method.
    #[allow(unsafe_code)]
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        if layout.align() > 16 { core::ptr::null_mut() } else { c_malloc(layout.size().max(1)).cast() }
    }

    // UNSAFE(allocator): the trait's required method.
    #[allow(unsafe_code)]
    unsafe fn dealloc(&self, ptr: *mut u8, _layout: core::alloc::Layout) {
        c_free(ptr.cast());
    }

    // UNSAFE(allocator): the trait's provided method, overridden (one `calloc`).
    #[allow(unsafe_code)]
    unsafe fn alloc_zeroed(&self, layout: core::alloc::Layout) -> *mut u8 {
        if layout.align() > 16 { core::ptr::null_mut() } else { c_calloc(layout.size().max(1), 1).cast() }
    }

    // UNSAFE(allocator): the trait's provided method, overridden (one `realloc`).
    #[allow(unsafe_code)]
    unsafe fn realloc(&self, ptr: *mut u8, layout: core::alloc::Layout, new_size: usize) -> *mut u8 {
        if layout.align() > 16 { core::ptr::null_mut() } else { c_realloc(ptr.cast(), new_size.max(1)).cast() }
    }
}
