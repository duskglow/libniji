//! `niji_nscache_*`: the process's integrated name-service cache (D-16.10; the logic is
//! `niji_legacy::nscache`, the glue `crate::nscache`).
//!
//! The cache keeps the answers of the built-in sources (`files`, `dns`) for hosts, passwd,
//! group (with group lists), services, protocols and networks, in a table of the process or in
//! the shared region of its effective user (`/dev/shm/niji-nscache-<euid>`, used only when it
//! can be trusted). It never changes an answer: a lookup sees a change of a watched file at most
//! about a second late (at once in a process started after it, or after
//! `niji_nscache_invalidate`), and DNS data no longer than its TTL. These calls switch it
//! (`NIJI_NSCACHE=0` in the environment switches it off at start, except under `AT_SECURE`),
//! invalidate it and report its state.

use super::abi;
use super::types::NijiNscacheInfo;
use crate::export;
use crate::nscache;

export! { "NIJI_NATIVE_1";
/// Enable (`on` non-zero) or disable the cache for the process: 1 if it was enabled, else 0.
/// Disabled, nothing is mapped, `stat`ed or kept, and a lookup costs one atomic load.
pub extern "C" fn niji_nscache_enable(on: u32) -> i64 {
    abi::count(nscache::enable(on))
}}

export! { "NIJI_NATIVE_1";
/// Enable or disable one category (`NIJI_NSCACHE_HOSTS` ... `NIJI_NSCACHE_NETWORKS`): 1 if
/// it was enabled, else 0; `-EINVAL` for another number.
pub extern "C" fn niji_nscache_category(category: u32, on: u32) -> i64 {
    abi::value(nscache::set_category(category, on))
}}

export! { "NIJI_NATIVE_1";
/// Invalidate one category or every one (`NIJI_NSCACHE_ALL`), here and in the user's shared
/// region; the next lookup validates the watched files at once: 0, or `-EINVAL`.
pub extern "C" fn niji_nscache_invalidate(category: u32) -> i64 {
    abi::done(nscache::invalidate(category))
}}

export! { "NIJI_NATIVE_1";
/// The cache's state into `*out`: 0, or `-EFAULT` for NULL.
pub extern "C" fn niji_nscache_status(out: Option<&mut NijiNscacheInfo>) -> i64 {
    abi::store(out, Ok(nscache::status()))
}}
