//! The bound on `getpass`'s line (`niji_getpass_set_limit`, D-17.8, D-17.10): none by
//! default; once the process sets one, a C `getpass` in the program reads at most that many
//! bytes before the newline. A longer line is never returned cut short: it is read to its end and discarded, every buffer of the library's that held it is
//! zeroed, and `getpass` fails with `EOVERFLOW`.

#![forbid(unsafe_code)]

use crate::buf;
use crate::ffi;
use crate::Errno;

/// The default bound: 0, none (`NIJI_GETPASS_DEFAULT_LIMIT`; D-17.9).
pub const DEFAULT_LIMIT: usize = 0;

/// Set the process's bound to `bytes` (0: none, [`DEFAULT_LIMIT`]), for the life of the
/// process and its `fork` children: the bound that was in force (0 for none), or `EINVAL` above
/// `isize::MAX`.
pub fn set_limit(bytes: usize) -> Result<usize, Errno> {
    buf::len(ffi::niji_getpass_set_limit(bytes))
}
