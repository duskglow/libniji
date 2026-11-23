//! Typed futex wait/wake over `Sys` (spec §22 kernel notes): plain waits, waits with an
//! absolute deadline on a chosen clock (`FUTEX_WAIT_BITSET`), and wakes.
//!
//! These are thin on purpose: the kernel's answer (`EAGAIN`, `EINTR`, `ETIMEDOUT`) is handed back
//! unchanged and every caller re-checks its own condition after any return. The one answer
//! decided here is a deadline with a negative `tv_sec` ([`past`]: `ETIMEDOUT`, no call); callers
//! sort results with [`woken`], so an error that is not a wake-up ends their wait.

use core::sync::atomic::AtomicU32;

use niji_kernel::errno::{Errno, KResult};
use niji_kernel::sys::{FutexArg, Sys};
use niji_kernel::uapi::{Timespec, FUTEX_BITSET_MATCH_ANY, FUTEX_CLOCK_REALTIME, FUTEX_PRIVATE_FLAG, FUTEX_WAIT, FUTEX_WAIT_BITSET, FUTEX_WAKE};

/// Who may wait on a word: this process only (the kernel can skip the shared-mapping lookup),
/// or any process mapping it (`PTHREAD_PROCESS_SHARED`).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Scope {
    /// Process-private.
    #[default]
    Private,
    /// Process-shared.
    Shared,
}

impl Scope {
    /// The operation flag this scope adds.
    const fn flag(self) -> u32 {
        match self {
            Scope::Private => FUTEX_PRIVATE_FLAG,
            Scope::Shared => 0,
        }
    }
}

/// Sleep while `word` holds `expected`. Returns when woken, interrupted (`EINTR`) or when the
/// word already differed (`EAGAIN`); callers re-check their condition in every case.
#[inline]
pub fn wait<S: Sys>(sys: &S, word: &AtomicU32, expected: u32, scope: Scope) -> KResult<()> {
    sys.futex(word, FUTEX_WAIT | scope.flag(), expected, FutexArg::None, None, 0).map(drop)
}

/// Wake up to `n` waiters on `word`; returns how many were woken.
#[inline]
pub fn wake<S: Sys>(sys: &S, word: &AtomicU32, n: u32, scope: Scope) -> KResult<u32> {
    let woken = sys.futex(word, FUTEX_WAKE | scope.flag(), n, FutexArg::None, None, 0)?;
    // The kernel never reports more than it was asked to wake.
    Ok(u32::try_from(woken).unwrap_or(n))
}

/// Wake every waiter on `word`.
#[inline]
pub fn wake_all<S: Sys>(sys: &S, word: &AtomicU32, scope: Scope) -> KResult<u32> {
    // The kernel takes the count as a signed int: `i32::MAX` means "all".
    wake(sys, word, i32::MAX.unsigned_abs(), scope)
}

/// The clock of an absolute deadline (§22.4–§22.5: `CLOCK_REALTIME` or `CLOCK_MONOTONIC`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Clock {
    /// `CLOCK_REALTIME`.
    Realtime,
    /// `CLOCK_MONOTONIC`.
    Monotonic,
}

/// Sleep while `word` holds `expected`, until `deadline` (absolute, on `clock`) if one is
/// given. `ETIMEDOUT` once the deadline passed; otherwise as [`wait`].
pub fn wait_until<S: Sys>(sys: &S, word: &AtomicU32, expected: u32, scope: Scope, deadline: Option<(Clock, &Timespec)>) -> KResult<()> {
    let Some((clock, at)) = deadline else { return wait(sys, word, expected, scope) };
    // A negative tv_sec is before every reading of either clock (neither is ever negative: the
    // realtime clock cannot be set before the epoch, the monotonic one starts at 0), so the
    // deadline has passed: decided here, without the call the kernel would reject (EINVAL).
    if past(at) {
        return Err(Errno::ETIMEDOUT);
    }
    // FUTEX_WAIT takes a relative time; the bitset form is the one that takes an absolute one,
    // on CLOCK_MONOTONIC unless FUTEX_CLOCK_REALTIME is given.
    let clock_flag = match clock {
        Clock::Realtime => FUTEX_CLOCK_REALTIME,
        Clock::Monotonic => 0,
    };
    let op = FUTEX_WAIT_BITSET | clock_flag | scope.flag();
    sys.futex(word, op, expected, FutexArg::Timeout(at), None, FUTEX_BITSET_MATCH_ANY).map(drop)
}

/// Whether an absolute deadline has certainly passed on either clock without reading it: a
/// negative `tv_sec` (whatever its `tv_nsec`). The kernel rejects such a deadline (`EINVAL`).
#[inline]
pub const fn past(at: &Timespec) -> bool {
    at.sec < 0
}

/// What a wait's result means to a caller that re-checks its condition after it: `Ok` for a
/// wake-up (woken, `EAGAIN`: the word already differed, `EINTR`: a signal handler ran), the error
/// otherwise (`ETIMEDOUT`, `ECANCELED`, or a kernel rejection such as `EINVAL`, `EFAULT` or
/// `ENOSYS` from a filter). The wait then ends with that error: retrying it could never succeed.
#[inline]
pub fn woken(r: KResult<()>) -> KResult<()> {
    match r {
        Err(Errno::EAGAIN | Errno::EINTR) => Ok(()),
        r => r,
    }
}
