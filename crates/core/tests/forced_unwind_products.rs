//! `pthread_exit` and acting on cancellation unwind the thread's stack with the program's
//! unwinder (spec §22.1 s22_1_c_2, §22.3 s22_3_c_4; D-22.19; issue #25 from a distribution's
//! libstdc++ tests `30_threads/async/forced_unwind.cc` and `packaged_task/forced_unwind.cc`).
//!
//! `tests/oracle/forced_unwind.cc` and `tests/oracle/forced_unwind.c` are built dynamically with
//! the system compilers, as programs built for the host C library, and run twice: on the host C
//! library (the reference, glibc 2.39 on the test machines) and on libniji's products (libniji's
//! `ld.so` as the interpreter, `LD_LIBRARY_PATH` at `target/niji`, ruling #5909). Both runs must
//! print the same text, which is also fixed here.
//!
//! Needs the products (`cargo xtask so`), `g++` and `gcc`; without a compiler the test says so and
//! passes.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

#[path = "support/timeout.rs"]
mod timeout;
use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn products() -> PathBuf {
    let lib = root().join("target/niji");
    for p in ["libc.so.6", "ld-linux-x86-64.so.2"] {
        assert!(lib.join(p).exists(), "{p} is missing: run `cargo xtask so`");
    }
    lib.canonicalize().unwrap()
}

fn have(compiler: &str) -> bool {
    if Command::new(compiler).arg("--version").output().is_err() {
        eprintln!("SKIPPED: no {compiler} on this machine (the forced unwind of exiting threads is not tested)");
        return false;
    }
    true
}

/// Builds `src` with `compiler` and `flags` twice in `dir`: `<name>.ref` for the host C library,
/// `<name>.niji` with libniji's loader as its interpreter.
fn build(dir: &Path, name: &str, compiler: &str, src: &str, flags: &[&str]) -> (PathBuf, PathBuf) {
    let source = root().join("crates/core/tests/oracle").join(src);
    let interp = format!("-Wl,--dynamic-linker={}", products().join("ld-linux-x86-64.so.2").display());
    let mut out = Vec::new();
    for (suffix, extra) in [("ref", None), ("niji", Some(interp.as_str()))] {
        let exe = dir.join(format!("{name}.{suffix}"));
        let built = Command::new(compiler).args(["-O1", "-pthread"]).args(flags).arg(&source).args(extra).arg("-o").arg(&exe).output().unwrap();
        assert!(built.status.success(), "{src} {flags:?}: {}", String::from_utf8_lossy(&built.stderr));
        out.push(exe);
    }
    let niji = out.pop().unwrap();
    (out.pop().unwrap(), niji)
}

/// Runs `exe` (on libniji's products when `niji`) and returns its standard output, after checking
/// that it exited 0. `LD_LIBRARY_PATH` is set by `env` inside the watchdog, so the watchdog itself
/// runs on the host C library.
fn run(exe: &Path, niji: bool, args: &[&str]) -> String {
    let mut c = Command::new("timeout");
    c.arg(timeout::secs(60)).arg("env");
    if niji {
        c.arg(format!("LD_LIBRARY_PATH={}", products().display()));
    }
    let out = c.arg(exe).args(args).output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(out.status.success(), "{} ({}): status {:?}\nstdout:\n{text}\nstderr:\n{}", exe.display(), if niji { "libniji" } else { "reference" }, out.status, String::from_utf8_lossy(&out.stderr));
    text
}

/// The `DT_NEEDED` names of `exe`.
fn needed(exe: &Path) -> Vec<String> {
    let out = Command::new("readelf").arg("-dW").arg(exe).output().unwrap();
    assert!(out.status.success(), "readelf {}", exe.display());
    String::from_utf8_lossy(&out.stdout).lines().filter(|l| l.contains("(NEEDED)")).filter_map(|l| l.split('[').nth(1)).map(|s| s.trim_end_matches(']').to_string()).collect()
}

/// Whether `exe` takes the function `sym` from a shared library.
fn imports(exe: &Path, sym: &str) -> bool {
    let out = Command::new("nm").args(["-D", "--undefined-only"]).arg(exe).output().unwrap();
    assert!(out.status.success(), "nm {}", exe.display());
    String::from_utf8_lossy(&out.stdout).lines().filter_map(|l| l.split_whitespace().last()).any(|s| s.split('@').next() == Some(sym))
}

fn scratch(tag: &str) -> PathBuf {
    let dir = root().join(format!("target/forced_unwind-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

const CXX_EXPECTED: &str = "exit-dtor: dtors=2 caught_fu=0 caught_all=0 result=42
exit-catch-forced: dtors=1 caught_fu=1 caught_all=0 result=7
exit-catch-all: dtors=2 caught_fu=0 caught_all=1 result=42
exit-lock: dtors=0 caught_fu=0 caught_all=0 result=0
exit-lock: trylock after join=0
cancel-pause: dtors=1 caught_fu=1 caught_all=0 result=-1
cancel-read: dtors=1 caught_fu=1 caught_all=0 result=-1
cancel-nanosleep: dtors=1 caught_fu=1 caught_all=0 result=-1
cancel-cond_wait: dtors=1 caught_fu=1 caught_all=0 result=-1
cancel-sem_wait: dtors=1 caught_fu=1 caught_all=0 result=-1
cancel-testcancel: dtors=1 caught_fu=1 caught_all=0 result=-1
cancel-join: dtors=1 caught_fu=1 caught_all=0 result=-1
cancel-async: dtors=1 caught_fu=1 caught_all=0 result=-1
cancel-cond_wait: trylock after join=0
order: dtors=0 caught_fu=0 caught_all=0 result=0
order: dtk
cleanup-push: dtors=0 caught_fu=0 caught_all=0 result=0
cleanup-push: dh
once: dtors=1 caught_fu=0 caught_all=0 result=0
once: runs=2
async: broken_promise=1
";

/// s22_1_c_2, s22_3_c_4: in C++, `pthread_exit` and acting on cancellation (at a point of each
/// kind) run the destructors of every frame they leave, enter `catch (abi::__forced_unwind &)` and
/// `catch (...)` handlers, release a mutex a destructor owns (also one `pthread_cond_wait`
/// reacquired), run them before `thread_local` and key destructors, interleave with
/// `pthread_cleanup_push` (C++ form), reset a `pthread_once` whose routine exited, make
/// `std::async`'s future report `broken_promise`, and run the main thread's destructors before the
/// process ends; the thread's result is the one it exited with.
#[test]
fn cxx_frames_unwind_when_a_thread_exits_or_is_cancelled() {
    if !have("g++") {
        return;
    }
    let dir = scratch("cxx");
    let (reference, niji) = build(&dir, "cxx", "g++", "forced_unwind.cc", &["-std=c++17"]);
    let main_exit = "main-exit: dtor ran\n";
    assert_eq!(run(&reference, false, &[]), format!("{CXX_EXPECTED}{main_exit}"), "the reference differs from the expected text");
    assert_eq!(run(&niji, true, &[]), format!("{CXX_EXPECTED}{main_exit}"));
    // D-22.19 (ruling #6884): a handler that ends without rethrowing the exit ends the process with
    // a fatal diagnostic and SIGABRT; the frames inside the handler's try block were unwound, no
    // code after the handler runs (libniji only).
    let c = format!("ulimit -c 0; exec timeout {} env LD_LIBRARY_PATH={} {} swallow", timeout::secs(60), products().display(), niji.display());
    let out = Command::new("sh").args(["-c", &c]).output().unwrap();
    let (stdout, stderr) = (String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    assert_eq!(stdout, format!("{CXX_EXPECTED}swallow: handler entered, dtors=1\n"), "swallow: stderr:\n{stderr}");
    assert!(stderr.contains("FATAL: exception not rethrown\n"), "swallow: stderr:\n{stderr}");
    assert!(out.status.code() == Some(134) || std::os::unix::process::ExitStatusExt::signal(&out.status) == Some(6), "swallow: status {:?}", out.status);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// s22_1_c_2, s22_3_c_4: C cleanup handlers (`pthread_cleanup_push`) run innermost first before
/// the key destructor, in a program without an unwinder (built plainly: only the handlers run, the
/// cleanup attribute is not an unwind cleanup), in one built with `-fexceptions` (the handlers
/// and the cleanup attribute's function run in their frames' order), and in one whose handlers
/// use the jump-buffer form (`__EXCEPTIONS` undefined) while the cleanup attribute is an unwind
/// cleanup (the unwind must reach each buffer in its own frame's turn, not jump to it from a
/// frame nearer the exit; each function keeps its own frame).
#[test]
fn c_cleanups_run_in_frame_order_when_a_thread_exits_or_is_cancelled() {
    if !have("gcc") {
        return;
    }
    let dir = scratch("c");
    for (name, flags, expected) in [
        ("plain", &[][..], "exit: order=31K result=9\ncancel: order=31K result=-1\n"),
        ("fexceptions", &["-fexceptions"][..], "exit: order=3a1K result=9\ncancel: order=3a1K result=-1\n"),
        ("buffers", &["-fexceptions", "-U__EXCEPTIONS", "-fno-inline"][..], "exit: order=3a1K result=9\ncancel: order=3a1K result=-1\n"),
    ] {
        let (reference, niji) = build(&dir, name, "gcc", "forced_unwind.c", flags);
        // The plain program loads no unwinder (libniji's exit without one is what it tests); the
        // other two need the compiler's.
        let unwinder = needed(&niji).iter().any(|n| n.starts_with("libgcc_s"));
        assert_eq!(unwinder, name != "plain", "{name}: DT_NEEDED {:?}", needed(&niji));
        // The buffers build registers jump buffers (the case under test), the -fexceptions one
        // does not.
        assert_eq!(imports(&niji, "__pthread_register_cancel"), name != "fexceptions", "{name}: cleanup handler form");
        assert_eq!(run(&reference, false, &[]), expected, "{name}: the reference differs from the expected text");
        assert_eq!(run(&niji, true, &[]), expected, "{name}");
    }
    std::fs::remove_dir_all(&dir).unwrap();
}
