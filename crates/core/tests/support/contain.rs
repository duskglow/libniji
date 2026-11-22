//! Containment for the test harnesses that start processes in namespaces (ROOT-HARNESS-CONTAIN;
//! the rule, the inventory and why: `docs/testing/harness-containment.md`). A test killed mid-run
//! (SIGKILL: no cleanup code runs), or a hung one that its runner ends, must leave nothing behind.
//! A root orphan cannot be signalled by the developer or by `timeout`; a user-namespace orphan can,
//! but holds files, ports and memory until someone notices. Both kinds put the command in a PID
//! namespace of its own, whose end the kernel turns into the end of every process in it, however
//! deep (pid_namespaces(7)), and tie that namespace to this test process:
//!
//! * Root (`sudo -n`, anywhere below the command): [`output`] and [`status`] run the command through
//!   `oracle/contain.sh`, as the calling user, as the first process of its own PID namespace (made by
//!   root, so the root processes below are in it too). The namespace ends when the command ends, and
//!   when this process goes: the command's standard input is a pipe whose write end only this
//!   process holds (close-on-exec), and contain.sh's root watcher ends the namespace on its end.
//! * User namespace (`unshare -r`, `--map-root-user`, a script that runs one): [`user`] returns a
//!   command that runs the program as the first process of a new PID namespace inside a user
//!   namespace that maps only the caller's own ids (so the program still runs as the caller, and
//!   its own `unshare -r` nests as before), with its own /proc. The `unshare` holding it gets the
//!   parent-death signal SIGKILL (setpriv), so it dies with the thread that spawned it, and
//!   `--kill-child` ends the namespace with it. No root is needed.
//!
//! The command is then process 1 of its namespace: a signal with the default action that a process
//! of the namespace sends it (its own `raise`, a `kill` from a child) is ignored (pid_namespaces(7));
//! a signal from outside or a fault still ends it. The parent-death signal follows the spawning
//! thread (PR_SET_PDEATHSIG, prctl(2)): spawn and wait in the same thread, as every caller here does.
//!
//! Included with `#[path]` (`oracle.rs` includes it as `oracle::contain`).
#![allow(dead_code)]

use std::ffi::{OsStr, OsString};
use std::io;
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Output, Stdio};

/// The PATH a contained command gets when its own command sets none.
const PATH: &str = "/usr/sbin:/usr/bin:/sbin:/bin";

/// `oracle/contain.sh`, from any crate of the workspace.
pub(crate) fnscript() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../core/tests/oracle/contain.sh")
}

/// `cmd` (its program, arguments, working directory and the variables set on it) run through
/// `contain.sh`. `cmd` is expected to have had `env_clear()`: the command gets `PATH` and only the
/// variables set on `cmd` (sudo would reset the rest). Run it with [`output`] or [`status`], which
/// hold its standard input.
pub(crate) fnroot(cmd: &Command) -> Command {
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

/// Waits for a child of [`root`]: its standard input is taken first, so that waiting does not
/// close it, and dropped only once the child has ended.
fn hold<T>(mut child: Child, wait: impl FnOnce(Child) -> io::Result<T>) -> io::Result<T> {
    let alive = child.stdin.take();
    let r = wait(child);
    drop(alive);
    r
}

/// `cmd` run contained ([`root`]), its output collected as `Command::output` does.
pub(crate) fnoutput(cmd: &Command) -> io::Result<Output> {
    let child = root(cmd).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
    hold(child, Child::wait_with_output)
}

/// `cmd` run contained ([`root`]) with the given standard output and error; its exit status.
pub(crate) fnstatus(cmd: &Command, stdout: Stdio, stderr: Stdio) -> io::Result<ExitStatus> {
    let child = root(cmd).stdin(Stdio::piped()).stdout(stdout).stderr(stderr).spawn()?;
    hold(child, |mut c| c.wait())
}

/// The arguments [`user`] puts before the program.
pub(crate) const USER: [&str; 11] = ["--pdeathsig", "KILL", "--", "unshare", "--user", "--map-current-user", "--pid", "--fork", "--kill-child", "--mount-proc", "--"];

/// A command that runs `program` contained in a user and PID namespace (the module comment);
/// configure it as `Command::new(program)`: arguments, environment, working directory and
/// standard streams pass through, and so does the exit status (a death by signal included).
pub(crate) fnuser(program: impl AsRef<OsStr>) -> Command {
    let mut c = Command::new("setpriv");
    c.args(USER).arg(program);
    c
}
