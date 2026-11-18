//! §22.7 named semaphores when the table of open names cannot grow (no spec clause: the error is
//! the fallible-allocation rule, S8 and ruling #5594; HL-7 removed the table's fixed count). The
//! test build's allocation-failure hook (ALLOC-FAIL-INJECT) fails the one allocation each
//! `sem_open` is allowed: the opens the table already has room for succeed, the first that needs
//! more room fails with `ENOMEM` (not `EMFILE`: no descriptor limit was reached), every name opened
//! before it still gives back its own semaphore, and the refused name opens once memory is back.

#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::arithmetic_side_effects, missing_docs)]

use niji_boundary::selftest::failinject::{self, FailAlloc};
use niji_boundary::sync;
use niji_kernel::errno::Errno;
use niji_kernel::uapi::{O_CREAT, O_EXCL};

#[global_allocator]
static ALLOC: FailAlloc = FailAlloc;

fn name(i: usize) -> Vec<u8> {
    format!("/niji-sem-nomem-{}-{i}", std::process::id()).into_bytes()
}

fn excl() -> i32 {
    i32::try_from(O_CREAT | O_EXCL).unwrap()
}

/// §22.7 Contract (s22_7_c_5): named semaphores, open many at once in one process; the table of
/// open names grows, and only memory running out refuses an open (`ENOMEM`, S8).
#[test]
fn table_growth_failure_is_enomem_and_keeps_open_names() {
    let mut open: Vec<(Vec<u8>, usize)> = Vec::new();
    let mut refused = None;
    for i in 0..4096 {
        let n = name(i);
        failinject::fail_nth(1);
        let r = sync::sem_open(Ok(&n), excl(), 0o600, 0);
        failinject::off();
        match r {
            Ok(a) => open.push((n, a)),
            Err(e) => {
                refused = Some((n, e));
                break;
            }
        }
    }
    // The refused file exists (O_CREAT ran before the table refused); unlink everything on the way out.
    let names: Vec<Vec<u8>> = open.iter().map(|(n, _)| n.clone()).chain(refused.iter().map(|(n, _)| n.clone())).collect();
    let cleanup = || {
        for n in &names {
            let _ = sync::sem_unlink(Ok(n));
        }
    };
    let good = refused.as_ref().is_some_and(|&(_, e)| e == Errno::ENOMEM) && open.len() >= 16;
    if !good {
        cleanup();
    }
    assert!(good, "open {} refused with {refused:?}: want ENOMEM, after the static area's 16", open.len());
    let Some((n, _)) = refused else { return };
    // Memory back: the refused name opens, as a semaphore of its own.
    let a = sync::sem_open(Ok(&n), 0, 0, 0);
    let ok_reopen = a.is_ok_and(|a| open.iter().all(|&(_, b)| b != a));
    // Every earlier name still gives back its own address (the table kept them).
    let kept = open.iter().all(|(n, addr)| sync::sem_open(Ok(n), 0, 0, 0).is_ok_and(|b| b == *addr));
    let closed = open.iter().all(|&(_, addr)| sync::sem_close(addr).is_ok() && sync::sem_close(addr).is_ok())
        && a.is_ok_and(|a| sync::sem_close(a).is_ok());
    // Every name is closed now: a close frees its entry, so opening and closing again and again
    // needs no memory at all (every allocation fails while armed).
    let churn_name = name(9999);
    let churn = (0..1000).all(|_| {
        failinject::fail_nth(1);
        let r = sync::sem_open(Ok(&churn_name), excl() & !i32::try_from(O_EXCL).unwrap(), 0o600, 0);
        failinject::off();
        r.is_ok_and(|a| sync::sem_close(a).is_ok())
    });
    let _ = sync::sem_unlink(Ok(&churn_name));
    cleanup();
    assert!(ok_reopen, "the refused name did not open once memory was back: {a:?}");
    assert!(kept, "a name opened before the refusal lost its semaphore");
    assert!(closed, "closing the names failed");
    assert!(churn, "churn: an open after closes needed new memory (a closed entry was not reused)");
}
