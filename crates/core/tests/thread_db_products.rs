//! `libthread_db.so.1` end to end (spec §22.10, D-22.17): gdb loads the
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

/// `tests/oracle/thread_db_client.c` built and run: on the reference (the build host's headers and
/// `-lthread_db`), or on libniji (`products`: its staged headers alone with the compiler's own and
/// the kernel UAPI directory, as niji-gcc orders them; linked dynamically against the products'
/// `libthread_db.so` and `libc.so.6` under the products' loader). Its stdout.
fn client(products: Option<&Path>, dir: &Path) -> String {
    let src = root().join("crates/core/tests/oracle/thread_db_client.c");
    let exe = dir.join(if products.is_some() { "thread_db_client-niji" } else { "thread_db_client-ref" });
    let mut cc = Command::new("cc");
    cc.args(["-std=gnu17", "-O1", "-Wall", "-Wextra", "-Werror"]);
    match products {
        None => {
            cc.arg(&src).arg("-lthread_db");
        }
        Some(lib) => {
            let gccinc = Command::new("cc").arg("-print-file-name=include").output().unwrap().stdout;
            let gccinc = String::from_utf8(gccinc).unwrap();
            cc.args(["-nostdinc", "-isystem"]).arg(lib.join("include")).args(["-isystem", gccinc.trim(), "-isystem"]).arg(lib.join("uapi"));
            cc.args(["-nostdlib", "-fno-builtin"]).arg(lib.join("niji-crt1.o")).arg(&src).arg(format!("-L{}", lib.display()));
            cc.arg(format!("-Wl,--dynamic-linker={}", lib.join("ld-linux-x86-64.so.2").display()));
            cc.arg("-lthread_db").arg(lib.join("libc.so.6")).arg("-lgcc");
        }
    }
    assert!(oracle::build(cc, &exe), "thread_db_client.c does not build ({})", if products.is_some() { "libniji" } else { "reference" });
    let mut run = Command::new("timeout");
    run.arg(timeout::secs(60));
    if let Some(lib) = products {
        // libc.so.6 is a pinned soname: a development build is reached through LD_LIBRARY_PATH (D-27.16), set by
        // `env` for the client alone (the host's `timeout` keeps the host's libc.so.6).
        run.arg("env").arg(format!("LD_LIBRARY_PATH={}", lib.display()));
    }
    let o = run.arg(&exe).output().unwrap();
    assert!(o.status.success(), "thread_db_client exits {:?}: {}", o.status, String::from_utf8_lossy(&o.stderr));
    String::from_utf8(o.stdout).unwrap()
}

#[test]
fn a_debugger_shaped_client_builds_on_libniji_headers_and_matches_the_reference() {
    // Clauses: no spec clause (spec §22.10 names libthread_db.so.1's interface; plan/headers.md slice 16: <thread_db.h>
    // and <proc_service.h> as gdb's thread support includes them). The client defines every ps_* callback with the
    // header's prototype, calls td_init, td_ta_new and links td_thr_get_info; its sizes, offsets, event-set macro
    // results and td_ta_new's refusal of a target without symbols must equal the reference's.
    let r = root();
    let products = r.join("target/niji");
    for p in ["libc.so.6", "libthread_db.so", "libthread_db.so.1", "ld-linux-x86-64.so.2", "niji-crt1.o", "include/thread_db.h", "uapi"] {
        assert!(products.join(p).exists(), "{p} is missing from the products: run `cargo xtask so`");
    }
    if skip_note::missing("cc") {
        skip_note::skipped("skipped: no C compiler");
        return;
    }
    let dir = r.join("target/test-oracles");
    std::fs::create_dir_all(&dir).unwrap();
    let want = client(None, &dir);
    let got = client(Some(&products.canonicalize().unwrap()), &dir);
    assert!(want.starts_with("td_init ok\ntd_ta_new refused, looked up\n"), "the reference's client:\n{want}");
    assert_eq!(got, want, "libniji's client (left) against the reference's (right)");
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
