//! D-X.60 (HEAP-FOREIGN-TB-SPEED): in a process the runtime did not start (this test process:
//! another C library made every thread block), a thread's heap cache word lives in the thread
//! registry, and the heap finds it there by thread pointer alone: a warm `malloc`/`free` makes no
//! system call (a `gettid` per operation roughly doubled the time of a foreign thread's heap
//! calls). A forked child under strict secure computing mode, where any system call kills it,
//! allocates and frees through the cache its thread warmed before the fork. The only test of this
//! target, so that no other thread can hold a heap lock at the fork.
// opaque-ok: the test is about libniji's own `malloc`/`free` in a process another library started.
#![no_builtins]
#![allow(clippy::unwrap_used, missing_docs)]

use niji_boundary::posix::s03_memory as mem;
use niji_boundary::selftest::{self, ChildEnd};

/// Small sizes of four classes, all served by the thread's cache once warm.
const SIZES: [usize; 4] = [24, 64, 136, 200];

/// `rounds` allocations and releases of each size; whether every allocation succeeded.
fn pairs(rounds: usize) -> bool {
    let mut ok = true;
    for _ in 0..rounds {
        for n in SIZES {
            let p = mem::malloc(n);
            ok &= !p.is_null();
            mem::free(p);
        }
    }
    ok
}

fn strict_pairs() -> i32 {
    if pairs(1000) { 0 } else { 1 }
}

#[test]
fn foreign_thread_warm_malloc_free_make_no_system_call() {
    // Trace: no spec clause (D-X.60, HEAP-FOREIGN-TB-SPEED: a foreign thread's heap cache word,
    // found without a system call).
    // One live block of each class, so that no slab of the warm cache empties in the child.
    let keep: Vec<_> = SIZES.iter().map(|&n| mem::malloc(n)).collect();
    assert!(pairs(1000), "warm-up: an allocation failed");
    let end = selftest::in_child_no_syscalls(strict_pairs).unwrap();
    for p in keep {
        mem::free(p);
    }
    if end == ChildEnd::Exited(selftest::NO_STRICT_MODE) {
        eprintln!("skipped: the kernel refused strict secure computing mode (prctl PR_SET_SECCOMP)");
        return;
    }
    assert_eq!(end, ChildEnd::Exited(0), "warm malloc/free on a foreign thread: Signaled(9) = a system call (strict mode), Exited(1) = an allocation failed");
}
