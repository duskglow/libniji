//! Writing a file a test then runs. Included with `#[path]` by the test files that write or copy a
//! program, script or loader and then exec it (`crates/*/tests`).
//!
//! The race: a file written by this process (`std::fs::copy`, `std::fs::write`) is open for
//! writing in this process until the write returns. The test binary runs its tests on parallel
//! threads, and a sibling thread that forks or spawns in that window gives its child a copy of the
//! descriptor (`O_CLOEXEC` closes it only at the child's own `exec`). An `exec` of the new file in
//! this thread, before that child has exec'd, then fails with ETXTBSY (seen once in
//! `reloc_ldso_kernel_started_without_proc`, finding #6787, and in STATIC-RELRO, #742).
//! The files here are written by a `cp` process that has exited before the function returns, so
//! no process of the test holds the file open for writing.
#![allow(dead_code)]

use std::path::Path;
use std::process::{Command, Stdio};

/// Copy `from` to `to` (mode and contents) through `cp`, so `to` can be exec'd at once.
pub(crate) fn copy_for_exec(from: &Path, to: &Path) {
    let st = Command::new("cp").arg("--").arg(from).arg(to).stdin(Stdio::null()).status().expect("cp");
    assert!(st.success(), "cp {} {} failed: {st:?}", from.display(), to.display());
}

/// Write `bytes` to `to` with `mode`, through a sibling file and `cp`, so `to` can be exec'd at once.
pub(crate) fn write_for_exec(to: &Path, bytes: &[u8], mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    let tmp = to.with_extension("staged-bytes");
    std::fs::write(&tmp, bytes).unwrap();
    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(mode)).unwrap();
    copy_for_exec(&tmp, to);
    std::fs::remove_file(&tmp).unwrap();
}
