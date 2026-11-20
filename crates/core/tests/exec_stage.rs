//! The helpers that write a file a test then execs (`helpers/exec_stage.rs`) never leave it busy.
//! No spec clause (test-harness behaviour; finding #6787): sibling threads fork in a loop while this
//! thread writes a program and execs it at once; a descriptor open for writing in this process
//! would be inherited by a sibling's child and the exec would fail with ETXTBSY.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[path = "helpers/exec_stage.rs"]
mod exec_stage;

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};

const ROUNDS: usize = 200;
const FORKERS: usize = 4;

fn dir(tag: &str) -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("exec-stage-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Run `body` for [`ROUNDS`] rounds while [`FORKERS`] threads spawn short children in a loop.
fn under_forks(body: impl Fn(usize) + Sync) {
    let stop = AtomicBool::new(false);
    std::thread::scope(|s| {
        for _ in 0..FORKERS {
            s.spawn(|| {
                while !stop.load(Ordering::Relaxed) {
                    let _ = Command::new("true").stdin(Stdio::null()).status();
                }
            });
        }
        // A failing round panics: the forkers must still be told to stop, or the scope never ends.
        struct Stop<'a>(&'a AtomicBool);
        impl Drop for Stop<'_> {
            fn drop(&mut self) {
                self.0.store(true, Ordering::Relaxed);
            }
        }
        let _stop = Stop(&stop);
        for i in 0..ROUNDS {
            body(i);
        }
    });
}

fn run(p: &std::path::Path) {
    let st = Command::new(p).stdin(Stdio::null()).status();
    assert!(st.as_ref().is_ok_and(|s| s.success()), "exec of {} failed: {st:?}", p.display());
}

#[test]
fn copy_for_exec_never_leaves_the_program_busy() {
    // Trace: no spec clause (test-harness behaviour; finding #6787).
    let d = dir("copy");
    let src = d.join("src");
    exec_stage::copy_for_exec(std::path::Path::new("/bin/true"), &src);
    under_forks(|i| {
        let q = d.join(format!("p{i}"));
        exec_stage::copy_for_exec(&src, &q);
        run(&q);
    });
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn write_for_exec_never_leaves_the_script_busy() {
    // Trace: no spec clause (test-harness behaviour; finding #6787).
    let d = dir("write");
    under_forks(|i| {
        let q = d.join(format!("s{i}"));
        exec_stage::write_for_exec(&q, b"#!/bin/sh\nexit 0\n", 0o755);
        run(&q);
    });
    std::fs::remove_dir_all(&d).ok();
}
