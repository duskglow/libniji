//! `libthread_db.so.1` end to end (spec §22.10, decision record SYS-THREAD-DB-2-1): gdb loads the
//! products' library into its own process and debugs `tests/oracle/thread_db_target.c`, linked
//! statically against the products, stopped with four threads (the initial thread and three it
//! created, each with its own value of the thread-local `tv`): `info threads` lists all four as
//! thread-library threads, each one's backtrace reaches its own function, `tv` reads per thread;
//! then gdb writes a core file and a second gdb lists the same threads and values from it.
//! Dynamic programs need the loader's debugger rendezvous (`_r_debug`), which libniji's `ld.so`
//! does not offer yet (pending: DL-23): this test proves static programs only.
//! Needs the products (`cargo xtask so`), a C compiler and gdb; without either tool the test says
//! so and passes.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing, clippy::arithmetic_side_effects, missing_docs)]
#[path = "support/skip_note.rs"]
mod skip_note;

#[path = "support/oracle.rs"]
mod oracle;

#[path = "support/timeout.rs"]
mod timeout;

use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The values of `tv` the target sets: the initial thread's, then its three threads'.
const VALUES: [i64; 4] = [7, 111, 122, 133];

/// gdb in batch mode on `args`, loading libthread_db only from the products directory, under a
/// hang guard (a thread library that loops would otherwise hold the gate). Its output, both streams.
fn gdb(products: &Path, args: &[&str]) -> String {
    let lib_dir = format!("set libthread-db-search-path {}", products.display());
    let o = Command::new("timeout")
        .args(["-k", "5", &timeout::secs(120), "gdb", "-batch", "-nx"])
        .args(["-iex", "set auto-load safe-path /", "-iex", &lib_dir, "-iex", "set startup-with-shell off"])
        .args(["-iex", "set debuginfod enabled off", "-iex", "set pagination off", "-iex", "set width 0"])
        .args(args)
        .env("LC_ALL", "C")
        .output()
        .unwrap();
    let text = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    assert!(o.status.code() != Some(124) && o.status.code() != Some(137), "gdb did not finish (hang guard):\n{text}");
    text
}

/// The text between the `echo`ed marker `==name` and the next marker.
fn section<'a>(text: &'a str, name: &str) -> &'a str {
    let marker = format!("=={name}\n");
    let start = text.find(&marker).map_or_else(|| panic!("no {name} section in gdb's output:\n{text}"), |i| i + marker.len());
    let rest = &text[start..];
    rest.find("\n==").map_or(rest, |e| &rest[..=e])
}

/// The `info threads` rows naming a thread-library thread (`Thread 0x<pthread_t> (LWP <n>)`).
fn thread_rows(s: &str) -> Vec<&str> {
    s.lines()
        .filter(|l| {
            let l = l.trim_start_matches(['*', ' ']);
            l.split_once("Thread 0x").is_some_and(|(id, rest)| {
                id.trim().parse::<u32>().is_ok() && rest.split_once(" (LWP ").is_some_and(|(hex, lwp)| u64::from_str_radix(hex, 16).is_ok() && lwp.split(')').next().is_some_and(|n| n.parse::<u32>().is_ok()))
            })
        })
        .collect()
}

/// The values `thread apply all print tv` printed (`$<n> = <v>`), sorted.
fn tv_values(s: &str) -> Vec<i64> {
    let mut v: Vec<i64> = s.lines().filter_map(|l| l.strip_prefix('$')?.split_once(" = ")?.1.trim().parse().ok()).collect();
    v.sort_unstable();
    v
}

#[test]
fn gdb_lists_threads_backtraces_and_tls_of_a_static_program_and_its_core() {
    // Clauses: no spec clause (spec §22.10 names libthread_db.so.1's interface, not a behaviour clause).
    let r = root();
    let products = r.join("target/niji");
    assert!(products.join("libc.a").exists() && products.join("libthread_db.so.1").exists(), "the products are missing: run `cargo xtask so`");
    if skip_note::missing("cc") {
        skip_note::skipped("skipped: no C compiler");
        return;
    }
    if skip_note::missing("gdb") {
        skip_note::skipped("skipped: no gdb");
        return;
    }
    let dir = r.join("target/test-oracles");
    std::fs::create_dir_all(&dir).unwrap();
    let exe = dir.join("thread_db_target");
    let core = dir.join("thread_db_target.core");
    let _ = std::fs::remove_file(&core);
    let mut cc = Command::new("cc");
    cc.args(["-std=gnu17", "-O0", "-g", "-static", "-nostdlib", "-fno-builtin"])
        .arg(products.join("niji-crt1.o"))
        .arg(r.join("crates/core/tests/oracle/thread_db_target.c"))
        .arg(products.join("libc.a"))
        .arg("-lgcc");
    assert!(oracle::build(cc, &exe), "thread_db_target.c does not link against the products");
    let (exe_s, core_s) = (exe.display().to_string(), core.display().to_string());
    let gcore = format!("generate-core-file {core_s}");

    let live = gdb(&products, &[
        "-ex", "break niji_tdb_stop", "-ex", "run",
        "-ex", "echo ==threads\\n", "-ex", "info threads",
        "-ex", "echo ==tv\\n", "-ex", "thread apply all print tv",
        "-ex", "echo ==bt\\n", "-ex", "thread apply all bt",
        "-ex", "echo ==core\\n", "-ex", &gcore,
        "-ex", "echo ==end\\n", "-ex", "kill",
        &exe_s,
    ]);
    if live.contains("ptrace: Operation not permitted") {
        skip_note::skipped("skipped: ptrace is not permitted here");
        return;
    }
    assert!(live.contains("Thread debugging using libthread_db enabled"), "gdb did not load the products' libthread_db:\n{live}");
    let rows = thread_rows(section(&live, "threads"));
    assert_eq!(rows.len(), 4, "info threads, live:\n{live}");
    assert_eq!(tv_values(section(&live, "tv")), VALUES, "tv per thread, live:\n{live}");
    let bt = section(&live, "bt");
    assert_eq!(bt.matches(" niji_tdb_worker (").count(), 3, "each created thread's backtrace reaches its start routine:\n{live}");
    assert!(bt.contains(" niji_tdb_stop (") && bt.contains(" main ("), "the initial thread's backtrace:\n{live}");

    assert!(core.exists(), "gdb wrote no core file:\n{live}");
    let dead = gdb(&products, &["-ex", "echo ==threads\\n", "-ex", "info threads", "-ex", "echo ==tv\\n", "-ex", "thread apply all print tv", "-ex", "echo ==end\\n", &exe_s, &core_s]);
    let _ = std::fs::remove_file(&core);
    assert!(dead.contains("Thread debugging using libthread_db enabled"), "gdb did not load the products' libthread_db for the core:\n{dead}");
    assert_eq!(thread_rows(section(&dead, "threads")).len(), 4, "info threads, core file:\n{dead}");
    assert_eq!(tv_values(section(&dead, "tv")), VALUES, "tv per thread, core file:\n{dead}");
}
