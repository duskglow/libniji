//! The constants of the native C ABI (D-X.10). Their values are the kernel's (⚑ KERN) or the
//! library's own; `spec/abi/native.tsv` repeats them for the header, and the tests check that
//! the two agree.

#![allow(missing_docs)] // each constant is documented in spec/abi/native.tsv and the header

use niji_kernel::uapi;

/// The native interface's newest version node: `NIJI_NATIVE_<n>`.
pub const NIJI_NATIVE_VERSION: i32 = 1;

/// The alignment every heap block has at least.
pub const NIJI_HEAP_MIN_ALIGN: usize = niji_rt::heap::ALIGN;

pub const NIJI_O_RDONLY: u32 = uapi::O_RDONLY;
pub const NIJI_O_WRONLY: u32 = uapi::O_WRONLY;
pub const NIJI_O_RDWR: u32 = uapi::O_RDWR;
pub const NIJI_O_CREAT: u32 = uapi::O_CREAT;
pub const NIJI_O_EXCL: u32 = uapi::O_EXCL;
pub const NIJI_O_TRUNC: u32 = uapi::O_TRUNC;
pub const NIJI_O_APPEND: u32 = uapi::O_APPEND;
pub const NIJI_O_NONBLOCK: u32 = uapi::O_NONBLOCK;
pub const NIJI_O_DIRECTORY: u32 = uapi::O_DIRECTORY;
pub const NIJI_O_NOFOLLOW: u32 = uapi::O_NOFOLLOW;
pub const NIJI_O_CLOEXEC: u32 = uapi::O_CLOEXEC;

pub const NIJI_SEEK_SET: u32 = uapi::SEEK_SET;
pub const NIJI_SEEK_CUR: u32 = uapi::SEEK_CUR;
pub const NIJI_SEEK_END: u32 = uapi::SEEK_END;

pub const NIJI_FS_NOFOLLOW: u32 = uapi::AT_SYMLINK_NOFOLLOW;

pub const NIJI_S_IFMT: u32 = uapi::S_IFMT;
pub const NIJI_S_IFSOCK: u32 = uapi::S_IFSOCK;
pub const NIJI_S_IFLNK: u32 = uapi::S_IFLNK;
pub const NIJI_S_IFREG: u32 = uapi::S_IFREG;
pub const NIJI_S_IFBLK: u32 = uapi::S_IFBLK;
pub const NIJI_S_IFDIR: u32 = uapi::S_IFDIR;
pub const NIJI_S_IFCHR: u32 = uapi::S_IFCHR;
pub const NIJI_S_IFIFO: u32 = uapi::S_IFIFO;

// The C locale categories (spec Appendix C: `LC_*`), and `LC_ALL` for the composite name.
pub const NIJI_LC_CTYPE: i32 = 0;
pub const NIJI_LC_NUMERIC: i32 = 1;
pub const NIJI_LC_TIME: i32 = 2;
pub const NIJI_LC_COLLATE: i32 = 3;
pub const NIJI_LC_MONETARY: i32 = 4;
pub const NIJI_LC_MESSAGES: i32 = 5;
pub const NIJI_LC_ALL: i32 = 6;
pub const NIJI_LC_PAPER: i32 = 7;
pub const NIJI_LC_NAME: i32 = 8;
pub const NIJI_LC_ADDRESS: i32 = 9;
pub const NIJI_LC_TELEPHONE: i32 = 10;
pub const NIJI_LC_MEASUREMENT: i32 = 11;
pub const NIJI_LC_IDENTIFICATION: i32 = 12;

/// The capacity of `struct niji_tm`'s abbreviation.
pub const NIJI_TZ_ABBR_CAP: usize = super::types::ABBR_CAP;


pub const NIJI_NSCACHE_HOSTS: u32 = 0;
pub const NIJI_NSCACHE_PASSWD: u32 = 1;
pub const NIJI_NSCACHE_GROUP: u32 = 2;
pub const NIJI_NSCACHE_SERVICES: u32 = 3;
pub const NIJI_NSCACHE_PROTOCOLS: u32 = 4;
pub const NIJI_NSCACHE_NETWORKS: u32 = 5;
pub const NIJI_NSCACHE_ALL: u32 = u32::MAX;
pub const NIJI_NSCACHE_MODE_OFF: u32 = niji_legacy::nscache::MODE_OFF;
pub const NIJI_NSCACHE_MODE_IDLE: u32 = niji_legacy::nscache::MODE_IDLE;
pub const NIJI_NSCACHE_MODE_PRIVATE: u32 = niji_legacy::nscache::MODE_PRIVATE;
pub const NIJI_NSCACHE_MODE_SHARED: u32 = niji_legacy::nscache::MODE_SHARED;
pub const NIJI_NSCACHE_MODE_UNAVAILABLE: u32 = niji_legacy::nscache::MODE_UNAVAILABLE;
pub const NIJI_NSCACHE_REASON_NONE: u32 = niji_legacy::nscache::REASON_NONE;
pub const NIJI_NSCACHE_REASON_NO_REGION: u32 = niji_legacy::nscache::REASON_NO_REGION;
pub const NIJI_NSCACHE_REASON_SECURE: u32 = niji_legacy::nscache::REASON_SECURE;
pub const NIJI_NSCACHE_REASON_REFUSED: u32 = niji_legacy::nscache::REASON_REFUSED;
pub const NIJI_NSCACHE_REASON_UNAVAILABLE: u32 = niji_legacy::nscache::REASON_UNAVAILABLE;

/// `getpass`'s default bound on its line, in bytes (D-17.8).
pub const NIJI_GETPASS_DEFAULT_LIMIT: usize = crate::users::PASS_DEFAULT_LIMIT;
