//! The plain route of a foreign thread's heap calls (HEAP-FOREIGN-PAIR, D-X.139): in a process
//! another C library started (this test process), `heap::DEBUG` holds `FOREIGN`, and while no
//! debugging is on the debugging paths call the heap directly with the thread's cache word found
//! once. That route must still zero `calloc`'s block, and must not be taken once debugging is on
//! (`M_PERTURB` fills a new block with the complement of its byte).
//! One test, in order: `mallopt` changes the process's heap state for every later call. The
//! blocks are written and read through the opaque wrappers, so the compiler cannot assume what
//! `calloc` returns.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::indexing_slicing)]

use niji_boundary::posix::s03_memory as m;
use niji_boundary::selftest::opaque;

/// `M_PERTURB` in `<malloc.h>`.
const M_PERTURB: core::ffi::c_int = -6;

/// The first `n` bytes of the block at `p`.
fn bytes(p: *mut core::ffi::c_void, n: usize) -> Vec<u8> {
    assert!(!p.is_null(), "allocation of {n} bytes failed");
    let mut v = vec![0x11u8; n];
    opaque::memcpy(v.as_mut_ptr().cast(), p, n);
    v
}

#[test]
fn foreign_plain_route_zeroes_calloc_and_yields_to_perturb() {
    // Trace: s3_1_c_2 (calloc zero-fills), s3_2_c_1 (mallopt M_PERTURB), on the foreign plain route (D-X.139).
    // `calloc` on a block the thread's cache just took back dirty: zeroed (the same size class,
    // so the cache hands the freed block out again).
    for n in [24usize, 64, 200] {
        for _ in 0..8 {
            let p = opaque::malloc(n);
            assert!(!p.is_null(), "malloc({n}) failed");
            opaque::memset(p, 0xAB, n);
            opaque::free(p);
            let q = opaque::calloc(1, n);
            // The free reached the heap: this thread's next block of the class is the one freed
            // (the lowest free block of the slab's hint word, which the free just cleared).
            assert_eq!(q, p, "the block freed by free({n}-byte block) was not reused by the next calloc(1, {n})");
            assert!(bytes(q, n).iter().all(|&b| b == 0), "calloc(1, {n}) returned a block that is not zero");
            opaque::free(q);
        }
    }
    // Debugging on: the route is not taken, so a new block is filled with !0x5A = 0xA5.
    assert_eq!(m::mallopt(M_PERTURB, 0x5A), 1, "mallopt(M_PERTURB) refused");
    let p = opaque::malloc(64);
    let got = bytes(p, 64);
    assert!(got.iter().all(|&b| b == 0xA5), "malloc(64) with M_PERTURB 0x5A was not filled with 0xA5: {got:02x?}");
    opaque::free(p);
    assert_eq!(m::mallopt(M_PERTURB, 0), 1, "mallopt(M_PERTURB, 0) refused");
}
