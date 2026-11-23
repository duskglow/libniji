//! `getaddrinfo_a`, `gai_suspend`, `gai_error`, `gai_cancel` (spec §16.2; libanl, part of libc
//! since 2.34; D-16.8): lookups on worker threads, with `getaddrinfo(3)`'s semantics each.
//!
//! **Requests.** Each `struct gaicb` of the list is queued, its `__return` set to
//! `EAI_INPROGRESS`; the library keeps its state in `__glibc_reserved` (queued, running, done)
//! and the request's group. A worker takes the oldest request, runs `getaddrinfo` with
//! `ar_name`, `ar_service` and `ar_request`, then stores `ar_result` and `__return` (the result
//! code; `gai_error` reads it). Workers are the library's own threads (at most
//! [`MAX_WORKERS`], made as requests wait and kept for later ones; they do not count as running
//! threads, and every application signal is blocked in them). Requests run in parallel up to the
//! number of workers.
//!
//! **Modes** (`getaddrinfo_a(3)`). `GAI_WAIT` returns when every request of the list is done
//! (no notification); `GAI_NOWAIT` returns at once, and the `sigevent` notifies once when the
//! whole list is done (observed): `SIGEV_SIGNAL` queues the signal to the process with
//! `si_code` `SI_ASYNCNL` and the event's value; `SIGEV_THREAD` calls the function with the
//! value on a new detached thread (made as the `SIGEV_THREAD` timers make theirs, D-23.3);
//! `SIGEV_NONE` or no event: nothing. Any other mode is `EAI_SYSTEM` with `EINVAL`. NULL
//! entries are skipped; a list without requests is 0. When no worker can be made (another
//! `ld.so`, D-22.10), `GAI_WAIT` runs the requests in the calling thread, one after the other,
//! and `GAI_NOWAIT` is `EAI_AGAIN` with each request's `__return` `EAI_AGAIN`.
//!
//! **Waiting** (`gai_suspend`, `getaddrinfo_a(3)`): 0 as soon as one listed request is done
//! (the man page; the reference reports `EAI_ALLDONE` for a request completed before the call:
//! a deliberate difference); `EAI_ALLDONE` for a list without requests; `EAI_AGAIN` when the
//! timeout expires (the man page; the reference reports `EAI_SYSTEM`: a deliberate
//! difference); `EAI_INTR` when a signal interrupts the wait.
//!
//! **Cancelling** (`gai_cancel`): a queued request is taken off the queue, `EAI_CANCELED`
//! (its `__return` too, and its group's notification counts it); a running one is not
//! cancelled (`EAI_NOTCANCELED`); a done one is `EAI_ALLDONE`. NULL cancels every queued request
//! of the process: `EAI_CANCELED` if any was, else `EAI_NOTCANCELED` while one runs, else
//! `EAI_ALLDONE` (observed).
//!
//! **Processes.** A child of `fork` starts with no workers and an empty queue (requests of the
//! parent are not run in it).

use core::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering::{AcqRel, Acquire, Relaxed, Release}};

use niji_abi::layout::struct_addrinfo;
use niji_kernel::errno::Errno;
use niji_kernel::sys::Sys;
use niji_kernel::uapi::{SigEvent, SigInfo, Timespec, CLOCK_MONOTONIC, SIGEV_SIGNAL, SIGEV_THREAD};
use niji_rt::chunks::Chunks;
use niji_rt::futex::{self, Clock, Scope};
use niji_rt::sync::{RawCondvar, RawMutex};
use niji_rt::thread::Attr;

use crate::conv;
use crate::errno;
use crate::ffi;
use crate::fork;
use crate::heap::KernelPages;
use crate::netdb;
use crate::sys::CSys;
use crate::thread;

/// `EAI_AGAIN`.
const EAI_AGAIN: i32 = -3;
/// `EAI_MEMORY`.
const EAI_MEMORY: i32 = -10;
/// `EAI_SYSTEM`.
const EAI_SYSTEM: i32 = -11;
/// `EAI_INPROGRESS`.
pub const EAI_INPROGRESS: i32 = -100;
/// `EAI_CANCELED`.
pub const EAI_CANCELED: i32 = -101;
/// `EAI_NOTCANCELED`.
pub const EAI_NOTCANCELED: i32 = -102;
/// `EAI_ALLDONE`.
pub const EAI_ALLDONE: i32 = -103;
/// `EAI_INTR`.
pub const EAI_INTR: i32 = -104;
/// `GAI_WAIT`.
pub const GAI_WAIT: i32 = 0;
/// `GAI_NOWAIT`.
pub const GAI_NOWAIT: i32 = 1;
/// `SI_ASYNCNL`: the `si_code` of the completion signal.
const SI_ASYNCNL: i32 = -60;
/// Every signal but the library's internal one (as for the timers' threads, D-23.3).
const APP_SIGNALS: u64 = !(1 << 31);

/// The most worker threads.
pub const MAX_WORKERS: usize = 32;
/// Queue slots in the library itself; more are mapped as requests wait (no limit but memory).
const QUEUE_INLINE: usize = 64;
/// Notification groups in the library itself; more are mapped as lists wait (no limit but
/// memory).
const GROUPS_INLINE: usize = 16;

// The words of a `struct gaicb` (56 bytes): `ar_name`, `ar_service`, `ar_request`,
// `ar_result`, then `__return` with the first reserved int (the state), then the group.
const W_NAME: usize = 0;
const W_SERVICE: usize = 1;
const W_REQUEST: usize = 2;
const W_RESULT: usize = 3;
const W_STATUS: usize = 4;
const W_GROUP: usize = 5;
const WORDS: usize = 7;

const ST_DONE: usize = 0;
const ST_QUEUED: usize = 1;
const ST_RUNNING: usize = 2;

static LOCK: RawMutex = RawMutex::new();
static WORK: RawCondvar = RawCondvar::new();
/// The queue, under [`LOCK`]: a ring of [`CAP`] slots (a request's address, 0 for a hole),
/// [`LEN`] of them from slot [`HEAD`] on. Its first [`QUEUE_INLINE`] slots are here, the rest in
/// [`QUEUE_MORE`]; a full ring grows by the next chunk (D-16.8).
static QUEUED: [AtomicU64; QUEUE_INLINE] = [const { AtomicU64::new(0) }; QUEUE_INLINE];
static QUEUE_MORE: Chunks = Chunks::new();
static CAP: AtomicUsize = AtomicUsize::new(QUEUE_INLINE);
static HEAD: AtomicUsize = AtomicUsize::new(0);
static LEN: AtomicUsize = AtomicUsize::new(0);
static WORKERS: AtomicUsize = AtomicUsize::new(0);
static IDLE: AtomicUsize = AtomicUsize::new(0);
/// Advances with every completion; `gai_suspend` and `GAI_WAIT` sleep on it.
static DONE: AtomicU32 = AtomicU32::new(0);

// A list awaiting its notification (a group): whether it is in use, the requests not done yet,
// the `sigevent`'s notification, signal, value and function. Group `g` is read without a lock
// (by the worker finishing its last request and by the notifier), so groups never move: the
// first [`GROUPS_INLINE`] are here, the rest in chunks mapped as they are needed (D-16.8).
const G_USED: usize = 0;
const G_REMAINING: usize = 1;
const G_NOTIFY: usize = 2;
const G_SIGNO: usize = 3;
const G_VALUE: usize = 4;
const G_FUNCTION: usize = 5;
const G_WORDS: usize = 6;

static GROUP: [AtomicU64; G_WORDS * GROUPS_INLINE] = [const { AtomicU64::new(0) }; G_WORDS * GROUPS_INLINE];
static GROUP_MORE: Chunks = Chunks::new();
/// Where the search for a free group starts: past the last group taken, lowered to any group
/// freed. Lists waiting at once then take a group each in constant time, not a scan of every
/// group in use (D-16.8). Only a hint: the search goes round once before mapping more groups.
static GROUP_HINT: AtomicUsize = AtomicUsize::new(0);

/// Group `g` is free again.
fn free_group(g: usize, grp: &[AtomicU64]) {
    gw(grp, G_USED).store(0, Release);
    GROUP_HINT.fetch_min(g, Relaxed);
}

/// Group `g`'s words; `None` past the groups mapped so far.
fn group_at(g: usize) -> Option<&'static [AtomicU64]> {
    match g.checked_sub(GROUPS_INLINE) {
        None => GROUP.get(g.checked_mul(G_WORDS)?..g.checked_add(1)?.checked_mul(G_WORDS)?),
        Some(j) => GROUP_MORE.get(&KernelPages, j, G_WORDS),
    }
}

/// Word `k` of a group.
fn gw(g: &[AtomicU64], k: usize) -> &AtomicU64 {
    static NONE: AtomicU64 = AtomicU64::new(0);
    g.get(k).unwrap_or(&NONE)
}

/// An `int` kept in a group word (its bit pattern).
fn int_of(w: u64) -> i32 {
    i32::from_ne_bytes(u32::try_from(w & 0xffff_ffff).unwrap_or(0).to_ne_bytes())
}

/// A group word holding an `int`.
fn word_of(v: i32) -> u64 {
    u64::from(u32::from_ne_bytes(v.to_ne_bytes()))
}

/// A word as an address or count.
fn us(w: u64) -> usize {
    usize::try_from(w).unwrap_or(0)
}

/// An address or count as a word.
fn wd(v: usize) -> u64 {
    u64::try_from(v).unwrap_or(0)
}

/// Under [`LOCK`]: queue slot `i` (below [`CAP`]).
fn qslot(i: usize) -> Option<&'static AtomicU64> {
    match i.checked_sub(QUEUE_INLINE) {
        None => QUEUED.get(i),
        Some(j) => QUEUE_MORE.get(&KernelPages, j, 1)?.first(),
    }
}

/// Under [`LOCK`]: the slot of the `k`-th queued request (from the oldest).
fn queued(k: usize) -> Option<&'static AtomicU64> {
    qslot(HEAD.load(Relaxed).checked_add(k)?.checked_rem(CAP.load(Relaxed))?)
}

fn cb(addr: usize) -> Option<&'static [AtomicUsize]> {
    conv::named_words(addr, WORDS)
}

fn status_word(ret: i32, state: usize) -> usize {
    usize::try_from(u32::from_ne_bytes(ret.to_ne_bytes())).unwrap_or(0) | state << 32
}

fn status(c: &[AtomicUsize]) -> (i32, usize) {
    let w = c.get(W_STATUS).map_or(0, |w| w.load(Acquire));
    (i32::from_ne_bytes(u32::try_from(w & 0xffff_ffff).unwrap_or(0).to_ne_bytes()), w >> 32)
}

fn set_status(c: &[AtomicUsize], ret: i32, state: usize) {
    if let Some(w) = c.get(W_STATUS) {
        w.store(status_word(ret, state), Release);
    }
}

fn completed() {
    DONE.fetch_add(1, Release);
    let _ = futex::wake_all(&CSys, &DONE, Scope::Private);
}

/// A request is done: store its result, wake the waiters, count it in its group.
fn finish(c: &[AtomicUsize], ret: i32, result: usize) {
    if let Some(w) = c.get(W_RESULT) {
        w.store(result, Relaxed);
    }
    let group = c.get(W_GROUP).map_or(0, |w| w.swap(0, Relaxed));
    set_status(c, ret, ST_DONE);
    completed();
    if let Some(g) = group.checked_sub(1).and_then(group_at) {
        if gw(g, G_REMAINING).fetch_sub(1, AcqRel) == 1 {
            notify(group.saturating_sub(1));
        }
    }
}

/// Run one request in the calling thread.
fn run(addr: usize) {
    let Some(c) = cb(addr) else { return };
    let word = |i: usize| c.get(i).map_or(0, |w| w.load(Relaxed));
    let node = conv::named_text(word(W_NAME));
    let service = conv::named_text(word(W_SERVICE));
    let hints = conv::named_bytes(word(W_REQUEST), size_of::<struct_addrinfo>()).map(|b| {
        let int = |at: usize| i32::from_ne_bytes(<[u8; 4]>::try_from(b.get(at..at.saturating_add(4)).unwrap_or(&[0; 4])).unwrap_or([0; 4]));
        struct_addrinfo { ai_flags: int(0), ai_family: int(4), ai_socktype: int(8), ai_protocol: int(12), ai_addrlen: 0, ai_addr: 0, ai_canonname: 0, ai_next: 0 }
    });
    let mut head = 0usize;
    let ret = netdb::getaddrinfo(Ok(node), Ok(service), hints.as_ref(), Some(&mut head));
    finish(c, ret, if ret == 0 { head } else { 0 });
}

/// The worker's start routine.
extern "C" fn worker(_: usize) -> usize {
    loop {
        let next = {
            let _g = LOCK.lock(&CSys);
            while LEN.load(Relaxed) == 0 {
                IDLE.fetch_add(1, Relaxed);
                WORK.wait(&CSys, &LOCK, Scope::Private);
                IDLE.fetch_sub(1, Relaxed);
            }
            pop()
        };
        if next != 0 {
            run(next);
        }
    }
}

#[allow(clippy::as_conversions)] // a function pointer's address: the only spelling Rust has
fn worker_addr() -> usize {
    worker as extern "C" fn(usize) -> usize as usize
}

#[allow(clippy::as_conversions)] // a function pointer's address: the only spelling Rust has
fn notifier_addr() -> usize {
    notifier as extern "C" fn(usize) -> usize as usize
}

/// Under the lock: the oldest queued request (marked running), or 0.
fn pop() -> usize {
    while LEN.load(Relaxed) > 0 {
        let a = queued(0).map_or(0, |s| us(s.swap(0, Relaxed)));
        HEAD.store(HEAD.load(Relaxed).wrapping_add(1).checked_rem(CAP.load(Relaxed)).unwrap_or(0), Relaxed);
        LEN.fetch_sub(1, Relaxed);
        if a != 0 {
            if let Some(c) = cb(a) {
                set_status(c, EAI_INPROGRESS, ST_RUNNING);
            }
            return a;
        }
    }
    0
}

/// Under the lock: queue `addr`; false without memory for a full queue to grow.
fn push(addr: usize) -> bool {
    let len = LEN.load(Relaxed);
    if len >= CAP.load(Relaxed) && !grow() {
        return false;
    }
    if let Some(s) = queued(len) {
        s.store(wd(addr), Relaxed);
    }
    LEN.store(len.saturating_add(1), Relaxed);
    true
}

/// Under the lock, the ring full: add the next chunk's slots to it; false without memory. The
/// requests in slots `HEAD..CAP` then `0..HEAD` move from `0..HEAD` to just past the old end,
/// so they stay in order (the new chunk is larger than the whole old ring).
#[cold]
#[inline(never)]
fn grow() -> bool {
    let cap = CAP.load(Relaxed);
    let Some(j) = cap.checked_sub(QUEUE_INLINE) else { return false };
    let Some((_, _, _, more)) = Chunks::place(j, 1) else { return false };
    if !QUEUE_MORE.reserve(&KernelPages, j, 1) {
        return false;
    }
    for t in 0..HEAD.load(Relaxed) {
        let v = qslot(t).map_or(0, |s| s.swap(0, Relaxed));
        if let Some(s) = cap.checked_add(t).and_then(qslot) {
            s.store(v, Relaxed);
        }
    }
    CAP.store(cap.saturating_add(more), Relaxed);
    true
}

/// Under the lock: take `addr` off the queue (leaving a hole the workers skip).
fn unqueue(addr: usize) -> bool {
    for k in 0..LEN.load(Relaxed) {
        if let Some(s) = queued(k) {
            if us(s.load(Relaxed)) == addr {
                s.store(0, Relaxed);
                return true;
            }
        }
    }
    false
}

/// Under the lock: start workers while requests wait and none is idle. Whether a worker can
/// run the queue.
fn workers() -> bool {
    while IDLE.load(Relaxed) < LEN.load(Relaxed) && WORKERS.load(Relaxed) < MAX_WORKERS {
        let a = Attr { detached: true, stack_size: 256 << 10, sigmask: Some(APP_SIGNALS), ..Attr::default() };
        if thread::create_service(&a, worker_addr(), 0).is_err() {
            break;
        }
        WORKERS.fetch_add(1, Relaxed);
        IDLE.fetch_add(1, Relaxed);
    }
    WORKERS.load(Relaxed) > 0
}

/// A list's notification (its group `g` is then free).
fn notify(g: usize) {
    let Some(grp) = group_at(g) else { return };
    match int_of(gw(grp, G_NOTIFY).load(Relaxed)) {
        SIGEV_SIGNAL => {
            let pid = CSys.getpid();
            let signo = int_of(gw(grp, G_SIGNO).load(Relaxed));
            let mut info = SigInfo::queued(signo, pid, CSys.getuid(), gw(grp, G_VALUE).load(Relaxed));
            info.code = SI_ASYNCNL;
            let _ = CSys.sigqueueinfo(pid, signo, &info);
            free_group(g, grp);
        }
        SIGEV_THREAD => {
            let a = Attr { detached: true, sigmask: Some(APP_SIGNALS), ..Attr::default() };
            if thread::create(&a, notifier_addr(), g, false).is_err() {
                free_group(g, grp);
            }
        }
        _ => free_group(g, grp),
    }
}

/// A `SIGEV_THREAD` notification's start routine: the function with the value.
extern "C" fn notifier(g: usize) -> usize {
    let Some(grp) = group_at(g) else { return 0 };
    let (f, v) = (us(gw(grp, G_FUNCTION).load(Relaxed)), gw(grp, G_VALUE).load(Relaxed));
    free_group(g, grp);
    // `void f(union sigval)`: the 8-byte union is passed as an integer (psABI).
    let _ = ffi::niji_machine_call3(f, i64::from_ne_bytes(v.to_ne_bytes()), 0, 0);
    0
}

/// A free group for a list of `n` requests notifying as `e` says, mapping more groups when all
/// are in use (`None`: no memory for them). The search starts at [`GROUP_HINT`] and goes round
/// to group 0 once before it maps more.
fn group(e: &SigEvent, n: usize) -> Option<usize> {
    let start = GROUP_HINT.load(Relaxed);
    let mut i = start;
    let mut round = start == 0;
    loop {
        let g = match group_at(i) {
            Some(g) => g,
            None if !round => {
                round = true;
                i = 0;
                continue;
            }
            None => {
                let _g = LOCK.lock(&CSys);
                if !GROUP_MORE.reserve(&KernelPages, i.checked_sub(GROUPS_INLINE)?, G_WORDS) {
                    return None;
                }
                group_at(i)?
            }
        };
        if gw(g, G_USED).compare_exchange(0, 1, AcqRel, Relaxed).is_ok() {
            // Past this group, unless a group was freed (the hint lowered) meanwhile.
            let _ = GROUP_HINT.compare_exchange(start, i.saturating_add(1), Relaxed, Relaxed);
            gw(g, G_REMAINING).store(wd(n), Relaxed);
            gw(g, G_NOTIFY).store(word_of(e.notify), Relaxed);
            gw(g, G_SIGNO).store(word_of(e.signo), Relaxed);
            gw(g, G_VALUE).store(e.value, Relaxed);
            let [f, _] = e.thread_words();
            gw(g, G_FUNCTION).store(f, Relaxed);
            return Some(i);
        }
        i = i.checked_add(1)?;
    }
}

/// `getaddrinfo_a(mode, list, nitems, sevp)` (`list`: the entries' addresses, 0 for NULL).
pub fn getaddrinfo_a(mode: i32, list: KList<'_>, sig: Option<&SigEvent>) -> i32 {
    // Before any request exists: a `fork` child from here on resets the queue and the groups.
    fork::mark_used(fork::USED_GAI);
    if mode != GAI_WAIT && mode != GAI_NOWAIT {
        errno::set(Errno::EINVAL);
        return EAI_SYSTEM;
    }
    let Ok(list) = list else {
        errno::set(Errno::EFAULT);
        return EAI_SYSTEM;
    };
    let reqs = || entries(list).filter(|&a| a != 0);
    let n = reqs().count();
    if n == 0 {
        return 0;
    }
    let notifying = mode == GAI_NOWAIT && sig.is_some_and(|e| e.notify == SIGEV_SIGNAL || e.notify == SIGEV_THREAD);
    let g = match (notifying, sig) {
        (true, Some(e)) => match group(e, n) {
            Some(g) => Some(g),
            None => return EAI_MEMORY,
        },
        _ => None,
    };
    let inline = {
        let _g = LOCK.lock(&CSys);
        let mut ok = true;
        for a in reqs() {
            let Some(c) = cb(a) else { continue };
            set_status(c, EAI_INPROGRESS, ST_QUEUED);
            if let Some(w) = c.get(W_GROUP) {
                w.store(g.map_or(0, |g| g.saturating_add(1)), Relaxed);
            }
            if !push(a) {
                ok = false;
            }
        }
        let runnable = ok && workers();
        if runnable {
            WORK.notify_all(&CSys, Scope::Private);
            false
        } else {
            for a in reqs() {
                unqueue(a);
            }
            if mode == GAI_NOWAIT {
                // No memory for the queue, else no worker could be made.
                let failure = if ok { EAI_AGAIN } else { EAI_MEMORY };
                for a in reqs() {
                    if let Some(c) = cb(a) {
                        if let Some(w) = c.get(W_GROUP) {
                            w.store(0, Relaxed);
                        }
                        set_status(c, failure, ST_DONE);
                    }
                }
                if let Some((g, grp)) = g.and_then(|g| Some((g, group_at(g)?))) {
                    free_group(g, grp);
                }
                return failure;
            }
            true
        }
    };
    if inline {
        for a in reqs() {
            run(a);
        }
        return 0;
    }
    if mode == GAI_WAIT {
        loop {
            let seen = DONE.load(Acquire);
            if reqs().all(|a| cb(a).is_none_or(|c| status(c).1 == ST_DONE)) {
                break;
            }
            let _ = futex::wait(&CSys, &DONE, seen, Scope::Private);
        }
    }
    0
}

/// A request list: the bytes of its entries (8 each).
pub type KList<'a> = niji_kernel::errno::KResult<&'a [u8]>;

/// The entries of a list, as addresses (0 for NULL).
fn entries(list: &[u8]) -> impl Iterator<Item = usize> + Clone + '_ {
    list.chunks_exact(8).map(|c| usize::from_ne_bytes(<[u8; 8]>::try_from(c).unwrap_or([0; 8])))
}

/// `gai_error(req)`.
pub fn gai_error(req: usize) -> i32 {
    match cb(req) {
        Some(c) => status(c).0,
        None => {
            errno::set(Errno::EFAULT);
            EAI_SYSTEM
        }
    }
}

/// `gai_suspend(list, nitems, timeout)`.
pub fn gai_suspend(list: KList<'_>, timeout: Option<&Timespec>) -> i32 {
    let Ok(list) = list else { return EAI_ALLDONE };
    let reqs = || entries(list).filter(|&a| a != 0);
    if reqs().next().is_none() {
        return EAI_ALLDONE;
    }
    let deadline = timeout.map(|t| {
        let now = CSys.clock_gettime(CLOCK_MONOTONIC).unwrap_or_default();
        let ns = now.nsec.saturating_add(t.nsec.clamp(0, 999_999_999));
        Timespec { sec: now.sec.saturating_add(t.sec.max(0)).saturating_add(ns / 1_000_000_000), nsec: ns % 1_000_000_000 }
    });
    loop {
        let seen = DONE.load(Acquire);
        if reqs().any(|a| cb(a).is_some_and(|c| status(c).0 != EAI_INPROGRESS)) {
            return 0;
        }
        match futex::wait_until(&CSys, &DONE, seen, Scope::Private, deadline.as_ref().map(|d| (Clock::Monotonic, d))) {
            Err(e) if e == Errno::ETIMEDOUT => return EAI_AGAIN,
            Err(e) if e == Errno::EINTR => return EAI_INTR,
            _ => {}
        }
    }
}

/// `gai_cancel(req)` (0: every queued request).
pub fn gai_cancel(req: usize) -> i32 {
    let mut canceled: [usize; 64] = [0; 64];
    let mut n = 0usize;
    let r = {
        let _g = LOCK.lock(&CSys);
        if req == 0 {
            let mut running = WORKERS.load(Relaxed) > IDLE.load(Relaxed);
            for k in 0..LEN.load(Relaxed) {
                if let Some(s) = queued(k) {
                    let a = us(s.load(Relaxed));
                    if a != 0 && n < canceled.len() {
                        s.store(0, Relaxed);
                        if let Some(slot) = canceled.get_mut(n) {
                            *slot = a;
                        }
                        n = n.saturating_add(1);
                    } else if a != 0 {
                        running = true;
                    }
                }
            }
            if n > 0 {
                EAI_CANCELED
            } else if running {
                EAI_NOTCANCELED
            } else {
                EAI_ALLDONE
            }
        } else {
            match cb(req) {
                None => EAI_ALLDONE,
                Some(c) => match status(c) {
                    (EAI_INPROGRESS, ST_QUEUED) if unqueue(req) => {
                        if let Some(slot) = canceled.first_mut() {
                            *slot = req;
                        }
                        n = 1;
                        EAI_CANCELED
                    }
                    (EAI_INPROGRESS, _) => EAI_NOTCANCELED,
                    _ => EAI_ALLDONE,
                },
            }
        }
    };
    for &a in canceled.get(..n).unwrap_or(&[]) {
        if let Some(c) = cb(a) {
            finish(c, EAI_CANCELED, 0);
        }
    }
    r
}

/// The child of `fork`: no workers, an empty queue, no groups (the queue's and the groups' memory
/// stays, for the child's own requests; a slot outside the queued ones is never read). `used`:
/// `getaddrinfo_a` ever ran ([`fork::USED_GAI`]); before that the queue and the groups hold
/// their initial values and the child reads and writes none of their pages (D-18.8).
pub(crate) fn after_fork_child(used: bool) {
    LOCK.reset();
    if !used {
        return;
    }
    HEAD.store(0, Relaxed);
    LEN.store(0, Relaxed);
    WORKERS.store(0, Relaxed);
    IDLE.store(0, Relaxed);
    let mut g = 0usize;
    while let Some(grp) = group_at(g) {
        gw(grp, G_USED).store(0, Relaxed);
        g = g.saturating_add(1);
    }
    GROUP_HINT.store(0, Relaxed);
}
