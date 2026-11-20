//! The program inputs of a verdict key (TVA-MATH-VERDICT-CACHE, D-10.57): the shared objects a
//! dynamic reference program loads. Kept apart from `verdict.rs` because it starts the program
//! (through the dynamic loader), and `verdict.rs` itself runs nothing, so a pure-Rust sweep can
//! include it and stay pure Rust. Included with `#[path]`, next to `verdict.rs`, by the C-oracle
//! sweeps only; a target that includes this file builds or runs programs and is never
//! `nextest::PASS0_ONLY`.
//!
//! Also the programs' outputs shared by the shards of one test run ([`shared_outputs`]).
#![allow(dead_code)]

use std::fs::File;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;

use niji_boundary::sys::CSys;
use niji_kernel::fd::Fd;
use niji_kernel::sys::Sys;
use niji_kernel::uapi::LOCK_EX;

use crate::verdict::Key;

/// The run every shard of one test run shares: nextest's run id (`NEXTEST_RUN_ID`, the same in
/// every test process of one `cargo nextest run`), or this process's id under `cargo test` (one
/// process runs every shard). Only characters safe in a file name.
fn this_run() -> String {
    let id = std::env::var("NEXTEST_RUN_ID").ok().filter(|v| !v.is_empty()).map_or_else(|| format!("pid{}", std::process::id()), |v| format!("nextest{v}"));
    id.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').collect()
}

/// Standard output of `prog`, run with an empty environment, and whether it exited with status 0.
fn run_program(prog: &Path) -> (String, bool) {
    let out = Command::new(prog).env_clear().output().unwrap_or_else(|e| panic!("{} cannot be started: {e}", prog.display()));
    (String::from_utf8(out.stdout).unwrap_or_else(|e| panic!("{} printed bytes that are not UTF-8: {e}", prog.display())), out.status.success())
}

/// The standard output of each program of `progs` (run with an empty environment), run once per
/// test run and read by every shard of the sweep `name` (MATH-ORACLE-PROFILE): every shard used
/// to run both programs over the whole grid before taking its share.
///
/// `key` holds every input the outputs depend on: the sweep's verdict key without the shard (the
/// test binary, the CPU flags, the kernel, the bytes of both programs and of every shared object
/// the reference program loads). The outputs are `dir/<name>-<key id>-<run>.<j>`, `<run>` from
/// [`this_run`], so an output never serves another test run, even one with equal inputs. Under
/// the lock file `dir/<name>.lock` (flock) the first shard runs the programs, writes each output
/// to a temporary name and renames it (rename(2): a name holds a complete output or nothing) and
/// removes the outputs of other keys and runs; a later shard reads them under the same lock. An
/// incomplete key, a program that exits with a status other than 0, or an output that cannot be
/// written runs here and is not shared, so a later shard runs it again.
pub(crate) fn shared_outputs(name: &str, key: &Key, progs: &[&Path], dir: &Path) -> Vec<String> {
    shared_outputs_in(name, key, progs, dir, &this_run())
}

/// [`shared_outputs`] for the test run `run` (a name of letters, digits and `-`).
pub(crate) fn shared_outputs_in(name: &str, key: &Key, progs: &[&Path], dir: &Path, run: &str) -> Vec<String> {
    if !key.complete() || std::fs::create_dir_all(dir).is_err() {
        return progs.iter().map(|p| run_program(p).0).collect();
    }
    let stem = format!("{name}-{}-{run}", key.id());
    let path = |j: usize| dir.join(format!("{stem}.{j}"));
    // Held until the function returns; without it (no flock on this file system) concurrent
    // shards each run the programs and rename complete outputs, which is still correct.
    let lock = File::options().create(true).truncate(false).write(true).open(dir.join(format!("{name}.lock")));
    if let Ok(f) = &lock {
        let _ = CSys.flock(Fd::new(f.as_raw_fd()), LOCK_EX);
    }
    let read: Option<Vec<String>> = (0..progs.len()).map(|j| std::fs::read_to_string(path(j)).ok()).collect();
    if let Some(outs) = read {
        crate::verdict::say(&format!("oracle outputs shared: {stem}\n"));
        return outs;
    }
    // Outputs of other keys or runs: a run in this tree needs only its own.
    let mine = format!("{stem}.");
    // FS-ORDER: each entry is judged and removed on its own; the order changes nothing.
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let n = e.file_name().to_string_lossy().into_owned();
        if n.starts_with(&format!("{name}-")) && !n.starts_with(&mine) {
            let _ = std::fs::remove_file(e.path());
        }
    }
    let mut outs = Vec::new();
    for (j, p) in progs.iter().enumerate() {
        let (text, ok) = run_program(p);
        let tmp = dir.join(format!(".{stem}.{j}.tmp{}", std::process::id()));
        if !ok || std::fs::write(&tmp, &text).is_err() || std::fs::rename(&tmp, path(j)).is_err() {
            let _ = std::fs::remove_file(&tmp);
        }
        outs.push(text);
    }
    crate::verdict::say(&format!("oracle outputs ran: {stem}\n"));
    outs
}

/// The shared objects program `prog` loads, as the dynamic loader resolves them
/// (`LD_TRACE_LOADED_OBJECTS=1`, ld.so(8): it lists the dependencies instead of running the
/// program); `None` if the program cannot be started, the loader reports a missing object, or
/// lists none.
pub(crate) fn loaded_objects(prog: &Path) -> Option<Vec<PathBuf>> {
    let out = Command::new(prog).env_clear().env("LD_TRACE_LOADED_OBJECTS", "1").output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8(out.stdout).ok()?;
    let mut objs = Vec::new();
    for l in text.lines().map(str::trim) {
        if l.contains("not found") {
            return None;
        }
        // `libm.so.6 => /usr/lib/.../libm.so.6 (0x...)`, or the loader: `/lib64/ld-linux-x86-64.so.2 (0x...)`.
        let path = match l.split_once("=> ") {
            Some((_, rest)) => rest,
            None => l,
        };
        let path = path.split(" (").next().unwrap_or("");
        if path.starts_with('/') {
            objs.push(PathBuf::from(path));
        }
    }
    objs.sort();
    objs.dedup();
    (!objs.is_empty()).then_some(objs)
}

impl Key {
    /// Adds the bytes of every shared object the dynamic program `prog` loads
    /// ([`loaded_objects`]); the key is incomplete if they cannot be resolved.
    #[must_use]
    pub(crate) fn loaded_by(self, prog: &Path) -> Self {
        self.resolved("loaded by", prog, loaded_objects)
    }
}
