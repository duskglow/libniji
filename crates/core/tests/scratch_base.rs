//! `Scratch::path` and `Scratch::path_in` never assume their base directory exists: a CI role removes
//! `target/tmp` before the run, and a test that writes to a fresh scratch path must still work.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing, clippy::arithmetic_side_effects, missing_docs)]

#[path = "support/scratch.rs"]
mod scratch;
use scratch::Scratch;

#[test]
fn path_in_makes_a_missing_base() {
    // Trace: no spec clause (test-harness behaviour; no flakes, owner rule 2026-10-21).
    let root = Scratch::path("scratch-base-root");
    let base = root.join("a").join("b");
    assert!(!base.exists(), "the base starts absent");
    let f = Scratch::path_in(&base, "file");
    std::fs::write(&*f, b"x").unwrap();
    assert_eq!(std::fs::read(&*f).unwrap(), b"x");
}
