//! A Rust `std` program run under libniji (STD-SMOKE). In the gate's native pass
//! (`NIJI_TEST_PASS=native`, PASS2-NATIVE-RUNNER, D-X.135) this binary is a copy whose program
//! interpreter is libniji's prefixed `ld.so`, which loads libniji's `libc.so.6` and the
//! toolchain's `libgcc_s.so.1` (the unwinder every `std` program names in `NEEDED`). The test
//! re-executes `current_exe()` as "the program": it runs threads with thread-local storage,
//! catches a panic (unwinding, with a destructor run on the way), runs a child through `std`'s
//! fork-then-exec path, writes and reads a file, reads its environment and exits with a status
//! of its own. The test checks the program's output line by line and its exit status, and in the
//! native pass that the program and its child were libniji processes. In the first pass the same
//! program runs on the host's C library and must print the same lines (the test's own check).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::print_stdout)]

use std::cell::Cell;
use std::io::Write as _;
use std::os::unix::fs::MetadataExt as _;
use std::os::unix::process::CommandExt as _;
use std::process::Command;
use std::sync::{mpsc, Arc, Barrier, Mutex};

const TEST: &str = "a_std_program_runs_threads_unwinds_forks_reexecs_and_exits";
const ROLE: &str = "STD_SMOKE_ROLE";
const PROGRAM_STATUS: i32 = 7;
const FORKED_STATUS: i32 = 3;
const THREADS: u64 = 8;

/// Runs this test binary again, only this test, as `role`.
fn again(role: &str) -> Command {
    let mut c = Command::new(std::env::current_exe().unwrap());
    c.args([TEST, "--exact", "--nocapture", "--test-threads=1", "-q"]).env(ROLE, role);
    c
}

/// Which loader and C library this process runs on: libniji's prefixed pair from the native
/// pass's root, or another (the host's).
fn loader() -> &'static str {
    let maps = std::fs::read_to_string("/proc/self/maps").unwrap();
    let files: Vec<&str> = maps.lines().filter_map(|l| l.split_whitespace().nth(5)).collect();
    let root = files.iter().find(|f| f.starts_with("/tmp/niji2-") && f.ends_with("/ld.so")).map(|f| f.trim_end_matches("/ld.so"));
    match root {
        Some(r) if files.iter().any(|f| *f == format!("{r}/lib/libc.so.6") || f.ends_with("/target/niji/libc.so.6")) => "libniji",
        _ => "other",
    }
}

fn lines_of(out: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(out).lines().filter(|l| l.starts_with("smoke: ")).map(str::to_owned).collect()
}

thread_local! {
    static OWN: Cell<u64> = const { Cell::new(0) };
}

struct Dropped(Arc<Mutex<bool>>);
impl Drop for Dropped {
    fn drop(&mut self) {
        *self.0.lock().unwrap() = true;
    }
}

fn threads() -> String {
    let barrier = Arc::new(Barrier::new(THREADS as usize));
    let counter = Arc::new(Mutex::new(0u64));
    let (tx, rx) = mpsc::channel();
    let handles: Vec<_> = (0..THREADS)
        .map(|_| {
            let (barrier, counter, tx) = (Arc::clone(&barrier), Arc::clone(&counter), tx.clone());
            std::thread::spawn(move || {
                barrier.wait();
                for _ in 0..1000 {
                    OWN.with(|c| c.set(c.get() + 1));
                    *counter.lock().unwrap() += 1;
                }
                tx.send((0..100_000u64).sum::<u64>()).unwrap();
                OWN.with(Cell::get)
            })
        })
        .collect();
    drop(tx);
    let own: Vec<u64> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    let sum: u64 = rx.iter().sum();
    let tls = if own.iter().all(|&n| n == 1000) && OWN.with(Cell::get) == 0 { "per-thread" } else { "shared" };
    format!("smoke: threads {THREADS} sum {sum} counter {} tls {tls}", *counter.lock().unwrap())
}

fn panics() -> String {
    std::panic::set_hook(Box::new(|_| {}));
    let flag = Arc::new(Mutex::new(false));
    let guard_flag = Arc::clone(&flag);
    let caught = std::panic::catch_unwind(move || {
        let _guard = Dropped(guard_flag);
        std::panic::panic_any("smoke-panic");
    });
    let payload = caught.err().and_then(|p| p.downcast_ref::<&str>().map(|s| (*s).to_owned())).unwrap_or_default();
    let in_thread = std::thread::spawn(|| -> u8 { panic!("thread") }).join().is_err();
    let _ = std::panic::take_hook();
    format!("smoke: panic payload {payload} destructor-ran {} thread-panic-seen {in_thread}", *flag.lock().unwrap())
}

fn fork_child() -> String {
    // A gid makes `std` start the child by fork, then exec (not posix_spawn): our own gid, which
    // setgid accepts without privilege.
    let gid = std::fs::metadata("/proc/self").unwrap().gid();
    let out = again("forked").gid(gid).output().unwrap();
    let lines = lines_of(&out.stdout);
    let theirs = lines.first().and_then(|l| l.strip_prefix("smoke: forked pid ")).and_then(|r| r.split(' ').next()).and_then(|p| p.parse::<u32>().ok());
    let other = theirs.is_some_and(|p| p != std::process::id());
    let child_loader = lines.first().and_then(|l| l.rsplit(' ').next()).unwrap_or("none").to_owned();
    format!("smoke: fork status {:?} other-pid {other} child-loader {child_loader}", out.status.code())
}

fn files_and_env() -> Vec<String> {
    let dir = std::path::PathBuf::from(std::env::var_os("STD_SMOKE_DIR").expect("STD_SMOKE_DIR"));
    let path = dir.join("note.txt");
    std::fs::write(&path, b"written by std\n").unwrap();
    let mut f = std::fs::OpenOptions::new().append(true).open(&path).unwrap();
    f.write_all(b"appended\n").unwrap();
    drop(f);
    let back = std::fs::read_to_string(&path).unwrap();
    let names: Vec<String> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    std::fs::remove_file(&path).unwrap();
    let greeting = std::env::var("STD_SMOKE_GREETING").unwrap_or_default();
    let listed = std::env::vars().any(|(k, v)| k == "STD_SMOKE_GREETING" && v == greeting);
    let cwd = std::env::current_dir().is_ok_and(|d| d.is_absolute());
    vec![
        format!("smoke: file {} bytes {:?} entries {names:?} removed {}", back.len(), back.lines().collect::<Vec<_>>(), !path.exists()),
        format!("smoke: env greeting {greeting} listed {listed} cwd-absolute {cwd}"),
    ]
}

fn program() -> ! {
    let mut out = vec![threads(), panics(), fork_child()];
    out.extend(files_and_env());
    out.push(format!("smoke: loader {}", loader()));
    let mut stdout = std::io::stdout().lock();
    for l in out {
        writeln!(stdout, "{l}").unwrap();
    }
    stdout.flush().unwrap();
    std::process::exit(PROGRAM_STATUS)
}

#[test]
fn a_std_program_runs_threads_unwinds_forks_reexecs_and_exits() {
    // Trace: no spec clause (a Rust `std` program as a libniji process: docs/toolchain.md section 8).
    match std::env::var(ROLE).as_deref() {
        Ok("program") => program(),
        Ok("forked") => {
            println!("smoke: forked pid {} loader {}", std::process::id(), loader());
            std::io::stdout().flush().unwrap();
            std::process::exit(FORKED_STATUS)
        }
        _ => {}
    }
    let dir = std::env::temp_dir().join(format!("niji-std-smoke-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let out = again("program").env("STD_SMOKE_DIR", &dir).env("STD_SMOKE_GREETING", "hello-from-parent").output().unwrap();
    let _ = std::fs::remove_dir(&dir);
    let native = std::env::var("NIJI_TEST_PASS").as_deref() == Ok("native");
    let expect_loader = if native { "libniji" } else { "other" };
    let sum = THREADS * (0..100_000u64).sum::<u64>();
    let want = vec![
        format!("smoke: threads {THREADS} sum {sum} counter {} tls per-thread", THREADS * 1000),
        "smoke: panic payload smoke-panic destructor-ran true thread-panic-seen true".to_owned(),
        format!("smoke: fork status Some({FORKED_STATUS}) other-pid true child-loader {expect_loader}"),
        r#"smoke: file 24 bytes ["written by std", "appended"] entries ["note.txt"] removed true"#.to_owned(),
        "smoke: env greeting hello-from-parent listed true cwd-absolute true".to_owned(),
        format!("smoke: loader {expect_loader}"),
    ];
    let got = lines_of(&out.stdout);
    assert_eq!(got, want, "the std program's output differs (status {:?}); stderr:\n{}", out.status.code(), String::from_utf8_lossy(&out.stderr));
    assert_eq!(out.status.code(), Some(PROGRAM_STATUS), "the std program's exit status");
}
