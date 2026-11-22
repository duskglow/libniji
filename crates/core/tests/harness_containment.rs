//! The containment rule for test harnesses (ROOT-HARNESS-CONTAIN, `docs/testing/harness-containment.md`):
//! a harness run whose test process is killed mid-run (SIGKILL, so no cleanup code runs) leaves no
//! process behind. One test per kind of `support/contain.rs`: a root run through `contain.sh`
//! (`sudo -n` below the command) and a user-namespace run through `contain::user`. Each re-runs this
//! binary as a helper that starts a harness-like run (a process and a forked child, carrying a token
//! on their command lines), kills the helper once they run, and checks that no process carrying the
//! token remains. The loops end by themselves after 300 s, so a run that does leave them behind
//! cleans up after itself. `xtask static` (`harness-contain`) checks that the tests use the helpers.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]
#[path = "support/contain.rs"]
mod contain;
#[path = "support/skip_note.rs"]
mod skip_note;
#[path = "support/timeout.rs"]
mod timeout;

use std::process::{Command, Stdio};

/// Set (to `<kind>:<token>`) in the helper child: there the test runs the contained run of its kind
/// and waits for it until the parent kills it.
const HELPER: &str = "NIJI_CONTAIN_HELPER";

/// A shell loop that ends by itself after 300 s.
const LOOPED: &str = "i=0; while [ $i -lt 300 ]; do sleep 1; i=$((i + 1)); done";

/// The processes whose command line has `token` as an argument: (pid, uid, argv).
fn carrying(token: &str) -> Vec<(u32, u32, Vec<String>)> {
    let mut found = Vec::new();
    for e in std::fs::read_dir("/proc").unwrap().flatten() {
        let Some(pid) = e.file_name().to_str().and_then(|n| n.parse::<u32>().ok()) else { continue };
        let Ok(raw) = std::fs::read(e.path().join("cmdline")) else { continue };
        let argv: Vec<String> = raw.split(|&b| b == 0).filter(|a| !a.is_empty()).map(|a| String::from_utf8_lossy(a).into_owned()).collect();
        if !argv.iter().any(|a| a == token) {
            continue;
        }
        let status = std::fs::read_to_string(e.path().join("status")).unwrap_or_default();
        let uid = status.lines().find_map(|l| l.strip_prefix("Uid:")).and_then(|v| v.split_whitespace().next()).and_then(|v| v.parse().ok()).unwrap_or(u32::MAX);
        found.push((pid, uid, argv));
    }
    found
}

/// Waits (a hang guard scaled by the load, `support/timeout.rs`) until `done` holds; false if it never did.
fn wait_until(mut done: impl FnMut() -> bool) -> bool {
    let end = timeout::deadline(std::time::Duration::from_secs(10));
    while !done() {
        if std::time::Instant::now() > end {
            return false;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    true
}

/// Re-runs `test` of this binary as the helper of `kind`, waits until `started` holds for the
/// processes carrying the token, SIGKILLs the helper and checks that none of them remains.
fn kill_mid_run(test: &str, kind: &str, started: impl Fn(&[(u32, u32, Vec<String>)]) -> bool) {
    let token = format!("niji-contain-{kind}-{}", std::process::id());
    // reexec-guard: allow(the child is killed mid-run, so it never prints its summary; that it ran the test is
    // shown by `started`: the processes carrying the token appear only from this test's helper side)
    let mut helper = Command::new(std::env::current_exe().unwrap())
        .args([test, "--exact", "--include-ignored", "--test-threads=1"])
        .env_clear()
        .env("PATH", "/usr/sbin:/usr/bin:/sbin:/bin")
        .env(HELPER, format!("{kind}:{token}"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    let ran = wait_until(|| started(&carrying(&token)));
    helper.kill().unwrap();
    helper.wait().unwrap();
    assert!(ran, "the contained {kind} run never started: {:?}", carrying(&token));
    let gone = wait_until(|| carrying(&token).is_empty());
    assert!(gone, "processes of a killed {kind} run remain (they end by themselves within 300 s): {:#?}", carrying(&token));
}

/// The helper side: the token when this process is the helper of `kind`.
fn helper_token(kind: &str) -> Option<String> {
    std::env::var(HELPER).ok().and_then(|v| v.strip_prefix(&format!("{kind}:")).map(str::to_string))
}

/// The processes whose script (`sh -c <script>`) starts with `mark`.
fn running(p: &[(u32, u32, Vec<String>)], mark: &str) -> usize {
    p.iter().filter(|p| p.2.get(2).is_some_and(|a| a.starts_with(mark))).count()
}

#[test]
fn a_killed_root_run_leaves_no_process() {
    // Trace: no spec clause (test harness: a root run whose test process is killed mid-run leaves
    // no process behind, root ones included; GAI-MASS-LOSS, ROOT-HARNESS-CONTAIN).
    if let Some(token) = helper_token("root") {
        let body = format!("sudo -n unshare -m -n sh -c ': root; {LOOPED}' sh \"$1\" & {LOOPED}");
        let mut cmd = Command::new("sh");
        cmd.args(["-c", &body, "sh", &token]).env_clear().env("PATH", "/usr/sbin:/usr/bin:/sbin:/bin");
        let _ = contain::output(&cmd);
        return;
    }
    if !Command::new("sudo").args(["-n", "true"]).status().is_ok_and(|s| s.success()) {
        skip_note::skipped("skipped: no passwordless sudo (the root runs it contains need it)");
        return;
    }
    kill_mid_run("a_killed_root_run_leaves_no_process", "root", |p| p.iter().any(|p| p.1 == 0 && p.2.get(2).is_some_and(|a| a.starts_with(": root;"))));
}

#[test]
fn a_killed_user_namespace_run_leaves_no_process() {
    // Trace: no spec clause (test harness: a user-namespace run (`unshare -r`) whose test process
    // is killed mid-run leaves no process behind, a forked child included; ROOT-HARNESS-CONTAIN).
    if let Some(token) = helper_token("user") {
        let body = format!(": user; ({LOOPED}) & {LOOPED}");
        let _ = contain::user("unshare").args(["-r", "-m", "sh", "-c", &body, "sh", &token]).stdin(Stdio::null()).status();
        return;
    }
    if skip_note::missing("setpriv") || !Command::new("unshare").args(["-r", "-m", "true"]).status().is_ok_and(|s| s.success()) {
        skip_note::skipped("skipped: no setpriv or no unprivileged user and mount namespaces (`unshare -r -m`)");
        return;
    }
    // The script's shell and its forked subshell (same command line) both run.
    kill_mid_run("a_killed_user_namespace_run_leaves_no_process", "user", |p| running(p, ": user;") >= 2);
}
