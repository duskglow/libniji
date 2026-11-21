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

#[test]
fn passing_scope_removes_its_directory() {
    // Trace: no spec clause (test-harness behaviour: repeated runs must not grow target/).
    let d = Scratch::dir_in(std::path::Path::new(env!("CARGO_TARGET_TMPDIR")), "scratch-pass");
    std::fs::write(d.join("f"), b"x").unwrap();
    let kept = d.to_path_buf();
    drop(d);
    assert!(std::fs::symlink_metadata(&kept).is_err(), "{} survived a passing scope", kept.display());
}

#[test]
fn failing_scope_keeps_its_directory() {
    // Trace: no spec clause (test-harness behaviour: a failure's scratch stays for inspection).
    let seen = std::sync::Mutex::new(None);
    let r = std::panic::catch_unwind(|| {
        let d = Scratch::dir("scratch-fail");
        std::fs::write(d.join("f"), b"x").unwrap();
        *seen.lock().unwrap() = Some(d.to_path_buf());
        panic!("a deliberately failing scope");
    });
    assert!(r.is_err());
    let kept = seen.lock().unwrap().take().unwrap();
    let survived = std::fs::read(kept.join("f"));
    scratch::remove(&kept);
    assert_eq!(survived.ok().as_deref(), Some(&b"x"[..]), "{} was removed by a failing scope", kept.display());
}
