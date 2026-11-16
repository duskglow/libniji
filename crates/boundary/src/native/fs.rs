//! `niji_fs_*`: file-system calls on paths. A path is a byte slice without NUL, relative to
//! the current directory unless it starts with `/`; it is terminated for the kernel in a
//! buffer of `PATH_MAX` bytes (a longer path is `-ENAMETOOLONG`, an interior NUL `-EINVAL`).

use niji_kernel::sys::Sys;
use niji_kernel::uapi::AT_REMOVEDIR;
use niji_kernel::Fd;

use super::abi;
use super::types::NijiStat;
use crate::conv;
use crate::export;
use crate::sys::CSys;

export! { "NIJI_NATIVE_1";
/// Open the file at `path` (`flags`: `NIJI_O_*`, the kernel's; `mode`: the permission bits
/// of a created file, before the umask): the new descriptor. `NIJI_O_CLOEXEC` is not added
/// implicitly.
pub extern "C" fn niji_fs_open(path: *const u8, path_len: usize, flags: u32, mode: u32) -> i64 {
    // UNSAFE(slice) SAFETY: path holds path_len bytes (D-X.10).
    abi::fd(abi::path(unsafe { conv::buf(path.cast(), path_len) }).and_then(|p| CSys.openat(Fd::CWD, p.as_kstr(), flags, mode)))
}}

export! { "NIJI_NATIVE_1";
/// The status of the file at `path` into `*out`. `flags` is 0 (a final symbolic link is
/// followed) or `NIJI_FS_NOFOLLOW` (the link itself).
pub extern "C" fn niji_fs_stat(path: *const u8, path_len: usize, flags: u32, out: Option<&mut NijiStat>) -> i64 {
    // UNSAFE(slice) SAFETY: path holds path_len bytes (D-X.10).
    abi::store(out, abi::path(unsafe { conv::buf(path.cast(), path_len) }).and_then(|p| CSys.fstatat(Fd::CWD, p.as_kstr(), flags)))
}}

export! { "NIJI_NATIVE_1";
/// Create the directory `path` with permission bits `mode` (before the umask).
pub extern "C" fn niji_fs_mkdir(path: *const u8, path_len: usize, mode: u32) -> i64 {
    // UNSAFE(slice) SAFETY: path holds path_len bytes (D-X.10).
    abi::done(abi::path(unsafe { conv::buf(path.cast(), path_len) }).and_then(|p| CSys.mkdirat(Fd::CWD, p.as_kstr(), mode)))
}}

export! { "NIJI_NATIVE_1";
/// Remove the directory entry `path` (not a directory).
pub extern "C" fn niji_fs_unlink(path: *const u8, path_len: usize) -> i64 {
    // UNSAFE(slice) SAFETY: path holds path_len bytes (D-X.10).
    abi::done(abi::path(unsafe { conv::buf(path.cast(), path_len) }).and_then(|p| CSys.unlinkat(Fd::CWD, p.as_kstr(), 0)))
}}

export! { "NIJI_NATIVE_1";
/// Remove the empty directory `path`.
pub extern "C" fn niji_fs_rmdir(path: *const u8, path_len: usize) -> i64 {
    // UNSAFE(slice) SAFETY: path holds path_len bytes (D-X.10).
    abi::done(abi::path(unsafe { conv::buf(path.cast(), path_len) }).and_then(|p| CSys.unlinkat(Fd::CWD, p.as_kstr(), AT_REMOVEDIR)))
}}

export! { "NIJI_NATIVE_1";
/// Rename `from` to `to`, replacing `to` if it exists (as `rename`).
pub extern "C" fn niji_fs_rename(from: *const u8, from_len: usize, to: *const u8, to_len: usize) -> i64 {
    // UNSAFE(slice) SAFETY: from holds from_len bytes and to to_len bytes (D-X.10).
    abi::done(abi::path(unsafe { conv::buf(from.cast(), from_len) }).and_then(|f| abi::path(unsafe { conv::buf(to.cast(), to_len) }).and_then(|t| CSys.renameat2(Fd::CWD, f.as_kstr(), Fd::CWD, t.as_kstr(), 0))))
}}
