//! The plain route of a foreign thread's heap calls (HEAP-FOREIGN-PAIR, D-X.139): in a process
//! another C library started (this test process), `heap::DEBUG` holds `FOREIGN`, and while no
//! debugging is on the debugging paths call the heap directly with the thread's cache word found
//! once. That route must still zero `calloc`'s block, and must not be taken once debugging is on
//! (`M_PERTURB` fills a new block with the complement of its byte).
//! One test, in order: `mallopt` changes the process's heap state for every later call.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::indexing_slicing)]

use niji_boundary::posix::s03_memory as m;

/// `M_PERTURB` in `<malloc.h>`.
const M_PERTURB: core::ffi::c_int = -6;

fn bytes(p: *mut core::ffi::c_void, n: usize) -> Vec<u8> {
    assert!(!p.is_null(), "allocation of {n} bytes failed");
    // UNSAFE(test): `p` is a live block of at least `n` bytes the heap just returned.
    #[allow(unsafe_code)]
    // SAFETY: see above.
    unsafe {
        core::slice::from_raw_parts(p.cast::<u8>(), n).to_vec()
    }
}

fn dirty(p: *mut core::ffi::c_void, n: usize) {
    // UNSAFE(test): `p` is a live block of at least `n` bytes.
    #[allow(unsafe_code)]
    // SAFETY: see above.
    unsafe {
        core::ptr::write_bytes(p.cast::<u8>(), 0xAB, n);
    }
}

#[test]
fn foreign_plain_route_zeroes_calloc_and_yields_to_perturb() {
    // Trace: s3_1_c_2 (calloc zero-fills), s3_2_c_1 (mallopt M_PERTURB), on the foreign plain route (D-X.139).
    // `calloc` on a block the thread's cache just took back dirty: zeroed (the same size class,
    // so the cache hands the freed block out again).
    for n in [24usize, 64, 200] {
        for _ in 0..8 {
            let p = m::malloc(n);
            dirty(p, n);
            m::free(p);
            let q = m::calloc(1, n);
            assert!(bytes(q, n).iter().all(|&b| b == 0), "calloc(1, {n}) returned a block that is not zero");
            m::free(q);
        }
    }
    // Debugging on: the route is not taken, so a new block is filled with !0x5A = 0xA5.
    assert_eq!(m::mallopt(M_PERTURB, 0x5A), 1, "mallopt(M_PERTURB) refused");
    let p = m::malloc(64);
    assert!(bytes(p, 64).iter().all(|&b| b == 0xA5), "malloc(64) with M_PERTURB 0x5A was not filled with 0xA5: {:02x?}", bytes(p, 64));
    m::free(p);
    assert_eq!(m::mallopt(M_PERTURB, 0), 1, "mallopt(M_PERTURB, 0) refused");
}
