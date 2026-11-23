//! Containment for the root and user-namespace runs xtask starts itself (ROOT-HARNESS-FOLLOWUP; the rule, the
//! inventory and why: `docs/testing/harness-containment.md`). The same two kinds as the tests'
//! `crates/core/tests/support/contain.rs`: the command is the first process of a PID namespace of its own, so
//! that when it ends, or xtask is killed (SIGKILL included), the kernel ends every process in it
//! (pid_namespaces(7)).
//!
//! * Root ([`output`]): the command runs through `crates/core/tests/oracle/contain.sh`, as the caller, in a PID
//!   namespace that root made (so the root processes it starts, `sudo -n ...`, are in it). Its standard input is
//!   one end of a socket pair whose other end this process holds until it exits ([`HOLD`], close-on-exec, so no
//!   child inherits it); contain.sh's root watcher ends the namespace when that end closes (end of file).
//! * User namespace ([`user`]): `setpriv --pdeathsig KILL -- unshare --user --map-current-user --pid --fork
//!   --kill-child --mount-proc -- program`. The program keeps the caller's ids and its own `unshare -r` nests;
//!   the `unshare` dies with the thread that spawned it (spawn and wait in one thread), and `--kill-child` takes
//!   the namespace with it.
//!
//! The command is process 1 of its namespace: a default-action signal that a process of the namespace sends it
//! (its own `raise`, a `kill` from a child) is ignored; a fault or a signal from outside still ends it.

use std::ffi::{OsStr, OsString};
use std::io;
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;

/// The PATH a contained command gets when its own command sets none.
const PATH: &str = "/usr/sbin:/usr/bin:/sbin:/bin";

/// The arguments [`user`] puts before the program.
pub(crate) const USER: [&str; 11] = ["--pdeathsig", "KILL", "--", "unshare", "--user", "--map-current-user", "--pid", "--fork", "--kill-child", "--mount-proc", "--"];

/// The socket pair that ties every root run to this process: the first end is each run's standard input, the other
/// is never written and closes only when this process exits.
static HOLD: OnceLock<(UnixStream, UnixStream)> = OnceLock::new();

/// `crates/core/tests/oracle/contain.sh` of this tree.
pub(crate) fn script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../crates/core/tests/oracle/contain.sh")
}

/// A command that runs `program` contained in a user and PID namespace (the module comment); configure it as
/// `Command::new(program)`: arguments, environment, working directory and standard streams pass through, and so
/// does the exit status.
pub(crate) fn user(program: impl AsRef<OsStr>) -> Command {
    let mut c = Command::new("setpriv");
    c.args(USER).arg(program);
    c
}

/// The contained form of `cmd` (program, arguments, working directory and the variables set on it), run through
/// contain.sh: the command gets `PATH` and only the variables set on `cmd`, as `env NAME=VALUE` arguments
/// (contain.sh clears the rest). Its standard input is still to be set ([`output`] does).
pub(crate) fn root(cmd: &Command) -> Command {
    let mut c = Command::new("sh");
    c.arg(script()).env_clear().env("PATH", PATH);
    let mut vars: Vec<OsString> = Vec::new();
    for (k, v) in cmd.get_envs() {
        match v {
            Some(v) if k == "PATH" => {
                c.env("PATH", v);
            }
            Some(v) => {
                let mut kv = k.to_os_string();
                kv.push("=");
                kv.push(v);
                vars.push(kv);
            }
            None => {}
        }
    }
    if !vars.is_empty() {
        c.arg("env").args(vars);
    }
    c.arg(cmd.get_program()).args(cmd.get_args());
    if let Some(d) = cmd.get_current_dir() {
        c.current_dir(d);
    }
    c
}

/// `cmd` run contained ([`root`]), its output collected as `Command::output` does.
pub(crate) fn output(cmd: &Command) -> io::Result<Output> {
    let hold = match HOLD.get() {
        Some(h) => h,
        None => {
            let pair = UnixStream::pair()?;
            HOLD.get_or_init(|| pair)
        }
    };
    root(cmd).stdin(Stdio::from(OwnedFd::from(hold.0.try_clone()?))).output()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Set (to `<kind>:<token>`) in the helper child: there the test runs the contained run of its kind and waits
    /// for it until the parent kills it.
    const HELPER: &str = "XTASK_CONTAIN_HELPER";
    /// A shell loop that ends by itself after 300 s.
    const LOOPED: &str = "i=0; while [ $i -lt 300 ]; do sleep 1; i=$((i + 1)); done";

    /// The processes with `token` as an argument: (uid, argv).
    fn carrying(token: &str) -> Vec<(u32, Vec<String>)> {
        let mut found = Vec::new();
        for e in std::fs::read_dir("/proc").unwrap().flatten() {
            if e.file_name().to_str().and_then(|n| n.parse::<u32>().ok()).is_none() {
                continue;
            }
            let Ok(raw) = std::fs::read(e.path().join("cmdline")) else { continue };
            let argv: Vec<String> = raw.split(|&b| b == 0).filter(|a| !a.is_empty()).map(|a| String::from_utf8_lossy(a).into_owned()).collect();
            if argv.iter().any(|a| a == token) {
                let status = std::fs::read_to_string(e.path().join("status")).unwrap_or_default();
                let uid = status.lines().find_map(|l| l.strip_prefix("Uid:")).and_then(|v| v.split_whitespace().next()).and_then(|v| v.parse().ok()).unwrap_or(u32::MAX);
                found.push((uid, argv));
            }
        }
        found
    }

    /// Polls `done` (a hang guard: 2,000 rounds of 20 ms); false if it never held.
    fn wait_until(mut done: impl FnMut() -> bool) -> bool {
        for _ in 0..2000 {
            if done() {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        false
    }

    /// Re-runs `test` of this binary as the helper of `kind`, waits until `started` holds for the processes
    /// carrying the token, SIGKILLs the helper and checks that none of them remains.
    fn kill_mid_run(test: &str, kind: &str, started: impl Fn(&[(u32, Vec<String>)]) -> bool) {
        let token = format!("xtask-contain-{kind}-{}", std::process::id());
        // reexec-guard: allow(the child is killed mid-run, so it never prints its summary; that it ran the test is
        // shown by `started`: the processes carrying the token appear only from this test's helper side)
        let mut helper = Command::new(std::env::current_exe().unwrap()).args([test, "--exact", "--test-threads=1"]).env(HELPER, format!("{kind}:{token}")).stdin(Stdio::null()).stdout(Stdio::null()).spawn().unwrap();
        let ran = wait_until(|| started(&carrying(&token)));
        helper.kill().unwrap();
        helper.wait().unwrap();
        assert!(ran, "the contained {kind} run never started: {:?}", carrying(&token));
        assert!(wait_until(|| carrying(&token).is_empty()), "processes of a killed {kind} run remain (they end by themselves within 300 s): {:#?}", carrying(&token));
    }

    fn helper_token(kind: &str) -> Option<String> {
        std::env::var(HELPER).ok().and_then(|v| v.strip_prefix(&format!("{kind}:")).map(str::to_string))
    }

    // ROOT-HARNESS-FOLLOWUP: no spec clause (developer tooling). A root run of xtask (`output`, the bench's
    // private-/dev/shm route) whose xtask process is killed mid-run leaves no process behind, root ones included.
    #[test]
    fn a_killed_xtask_root_run_leaves_no_process() {
        if let Some(token) = helper_token("root") {
            let body = format!("sudo -n unshare -m sh -c ': root; {LOOPED}' sh \"$1\" & {LOOPED}");
            let mut cmd = Command::new("sh");
            cmd.args(["-c", &body, "sh", &token]).env_clear().env("PATH", PATH);
            let _ = output(&cmd);
            return;
        }
        if !Command::new("sudo").args(["-n", "true"]).stdin(Stdio::null()).status().is_ok_and(|s| s.success()) {
            println!("SKIP a_killed_xtask_root_run_leaves_no_process: no passwordless sudo (the root runs it contains need it)");
            return;
        }
        kill_mid_run("contain::tests::a_killed_xtask_root_run_leaves_no_process", "root", |p| p.iter().any(|p| p.0 == 0 && p.1.get(2).is_some_and(|a| a.starts_with(": root;"))));
    }

    // ROOT-HARNESS-FOLLOWUP: no spec clause (developer tooling). A user-namespace run of xtask (`user`: the
    // bench's names/locale route, namespaced transcripts) whose xtask process is killed mid-run leaves no process
    // behind, a forked child included.
    #[test]
    fn a_killed_xtask_user_namespace_run_leaves_no_process() {
        if let Some(token) = helper_token("user") {
            let body = format!(": user; ({LOOPED}) & {LOOPED}");
            let _ = user("unshare").args(["-r", "-m", "sh", "-c", &body, "sh", &token]).stdin(Stdio::null()).status();
            return;
        }
        let have = Command::new("setpriv").arg("--version").output().is_ok_and(|o| o.status.success());
        if !have || !Command::new("unshare").args(["-r", "-m", "true"]).stdin(Stdio::null()).status().is_ok_and(|s| s.success()) {
            println!("SKIP a_killed_xtask_user_namespace_run_leaves_no_process: no setpriv or no unprivileged user and mount namespaces (`unshare -r -m`)");
            return;
        }
        kill_mid_run("contain::tests::a_killed_xtask_user_namespace_run_leaves_no_process", "user", |p| p.iter().filter(|p| p.1.get(2).is_some_and(|a| a.starts_with(": user;"))).count() >= 2);
    }

    // ROOT-HARNESS-FOLLOWUP: no spec clause (developer tooling). `root` keeps the program, its arguments, PATH and
    // the other variables (as `env` arguments, since contain.sh clears the environment).
    #[test]
    fn root_form_keeps_arguments_and_variables() {
        let mut cmd = Command::new("sudo");
        cmd.args(["-n", "x"]).env_clear().env("PATH", "/p").env("HOME", "/h");
        let c = root(&cmd);
        let args: Vec<String> = c.get_args().map(|a| a.to_string_lossy().into_owned()).collect();
        assert_eq!(c.get_program(), "sh");
        assert!(args[0].ends_with("oracle/contain.sh") && Path::new(&args[0]).is_file(), "{args:?}");
        assert_eq!(&args[1..], ["env", "HOME=/h", "sudo", "-n", "x"]);
        assert!(c.get_envs().any(|(k, v)| k == "PATH" && v == Some(OsStr::new("/p"))));
    }
}
