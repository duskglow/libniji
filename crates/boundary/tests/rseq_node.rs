//! The heap's node read and the rseq area (D-3.21): `node_id` is read only from an area Liella
//! registered. In this hosted test process the host C library started every thread, so Liella owns
//! no registration: a thread with a Liella descriptor (made on first need, D-22.10) still answers
//! that the area is not Liella's, and the heap's rseq node is absent (the vDSO `getcpu` serves).

#![allow(clippy::unwrap_used, missing_docs)]

use liella_boundary::{heap, pthread};

#[test]
fn a_registration_liella_does_not_own_is_never_read() {
    // Trace: D-3.21 (no spec clause: the heap's internal node source).
    let k = pthread::key_create(0).unwrap();
    pthread::setspecific(k, 1).unwrap(); // gives this thread a Liella descriptor
    assert!(!heap::rseq_owned(), "a thread whose rseq registration Liella did not make reads as Liella's");
    assert_eq!(heap::rseq_node(), None, "the node came from an rseq area Liella does not own");
    let t = std::thread::spawn(|| {
        let k = pthread::key_create(0).unwrap();
        pthread::setspecific(k, 2).unwrap();
        (heap::rseq_owned(), heap::rseq_node())
    });
    assert_eq!(t.join().unwrap(), (false, None), "another thread of the host library");
}
