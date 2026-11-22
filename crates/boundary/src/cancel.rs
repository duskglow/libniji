//! Cleanup buffers and the unwinding of an exiting or cancelled thread (spec §22.3; D-22.2,
//! D-22.11).
//!
//! `pthread_cleanup_push` (C, glibc's headers) fills a `__pthread_unwind_buf_t` with
//! `__sigsetjmp(buf, 0)` and registers it (`__pthread_register_cancel`). When the thread exits
//! (`pthread_exit`) or acts on cancellation, its stack is unwound with the program's unwinder
//! (`_Unwind_ForcedUnwind`, D-22.19): C++ destructors and `catch` handlers (which rethrow) and
//! `-fexceptions` cleanups run frame by frame, and when the unwind reaches the frame holding the
//! newest buffer, that buffer is jumped to; the handler code there runs the routine and calls
//! `__pthread_unwind_next(buf)`, which unwinds on from there. After the outermost frame, the
//! old-style records still registered (`_pthread_cleanup_push`) and the thread's exit sequence
//! (`crate::thread::finish`) run. A static program unwinds with the unwinder it links (D-22.21).
//! A process without an unwinder (a C program that loaded or linked none) skips the unwind: the
//! buffers are jumped to in turn, newest first. No Rust
//! frame is unwound: each jump, landing pad or exit leaves only this module's frames and the
//! exports', which hold nothing to drop (S11).

use core::sync::atomic::{AtomicUsize, Ordering::{AcqRel, Acquire, Relaxed, Release}};

use niji_kernel::errno::KResult;
use niji_abi::ctypes::c_int;
use niji_rt::thread::{CANCEL_ACTING, CANCEL_ASYNC, CANCEL_HIT};

use niji_kernel::errno::Errno;
use niji_kernel::sys::Sys;
use niji_rt::thread::{CANCEL_DISABLED, CANCEL_REQUESTED};

use crate::sys::CSys;
use crate::{ffi, raw, tb, thread};

/// `__pthread_unwind_buf_t` words: the jump buffer (0..8, mask flag at 8), then `prev` (9),
/// `cleanup` (10) and `canceltype` (11, an `int`).
const W_PREV: usize = 9;
const W_TYPE: usize = 11;
/// `_pthread_cleanup_buffer` words: routine, argument, `canceltype`, `prev`.
const O_ROUTINE: usize = 0;
const O_ARG: usize = 1;
const O_TYPE: usize = 2;
const O_PREV: usize = 3;

fn word(w: &[AtomicUsize], i: usize) -> usize {
    w.get(i).map_or(0, |x| x.load(Relaxed))
}

fn set(w: &[AtomicUsize], i: usize, v: usize) {
    if let Some(x) = w.get(i) {
        x.store(v, Relaxed);
    }
}

fn addr(w: &[AtomicUsize]) -> usize {
    w.as_ptr().expose_provenance()
}

/// `__pthread_register_cancel(buf)`.
pub fn register(buf: KResult<&[AtomicUsize]>) {
    let (Ok(b), Ok(d)) = (buf, thread::desc()) else { return };
    set(b, W_PREV, d.cleanup_jmp.load(Acquire));
    d.cleanup_jmp.store(addr(b), Release);
}

/// `__pthread_unregister_cancel(buf)`.
pub fn unregister(buf: KResult<&[AtomicUsize]>) {
    let (Ok(b), Ok(d)) = (buf, thread::desc()) else { return };
    d.cleanup_jmp.store(word(b, W_PREV), Release);
}

/// `__pthread_register_cancel_defer(buf)`: also switches to deferred cancellation, saving the
/// type in the buffer.
pub fn register_defer(buf: KResult<&[AtomicUsize]>) {
    let (Ok(b), Ok(d)) = (buf, thread::desc()) else { return };
    let old = d.cancel.fetch_and(!CANCEL_ASYNC, AcqRel);
    set(b, W_TYPE, usize::from(old & CANCEL_ASYNC != 0));
    set(b, W_PREV, d.cleanup_jmp.load(Acquire));
    d.cleanup_jmp.store(addr(b), Release);
}

/// `__pthread_unregister_cancel_restore(buf)`: restores the saved type (acting on a pending
/// request if that makes it asynchronous, D-22.11).
pub fn unregister_restore(buf: KResult<&[AtomicUsize]>) {
    let (Ok(b), Ok(d)) = (buf, thread::desc()) else { return };
    d.cleanup_jmp.store(word(b, W_PREV), Release);
    if word(b, W_TYPE) & 1 != 0 {
        d.cancel.fetch_or(CANCEL_ASYNC, AcqRel);
        test_cancel();
    }
}

/// `__pthread_unwind_next(buf)`: continue with the buffer below, or end the thread.
pub fn unwind_next(buf: KResult<&[AtomicUsize]>) -> ! {
    let next = buf.map_or(0, |b| word(b, W_PREV));
    let result = thread::desc().map_or(0, |d| {
        d.cleanup_jmp.store(next, Release);
        d.result.load(Acquire)
    });
    continue_unwind(next, result, false)
}

/// The thread exits with `result` (`pthread_exit`, `thrd_exit`, or acting on cancellation with
/// `PTHREAD_CANCELED`): the cleanup buffers, newest first, then the exit sequence.
pub fn unwind(result: usize) -> ! {
    let head = thread::desc().map_or(0, |d| {
        d.result.store(result, Release);
        d.cancel.fetch_or(CANCEL_ACTING, AcqRel);
        d.cleanup_jmp.load(Acquire)
    });
    continue_unwind(head, result, true)
}

/// `first`: the exit's first step (from [`unwind`]), where a process without an unwinder checks
/// that the thread's frames need none (D-22.23); a later step (from a cleanup buffer's handler)
/// comes after that check passed.
fn continue_unwind(buf: usize, result: usize, first: bool) -> ! {
    // Returns only when the process has no unwinder or the unwind could not start.
    forced_unwind(first);
    fallback(buf, result)
}

/// The exit without an unwinder: the newest cleanup buffer, or (after the last) the old-style
/// records and the exit sequence.
fn fallback(buf: usize, result: usize) -> ! {
    if buf != 0 {
        // The buffer was filled by `__sigsetjmp(buf, 0)` in a live frame of this thread.
        ffi::niji_machine_siglongjmp(buf, 1);
    }
    run_old_records();
    thread::finish(result)
}

/// The old-style records still registered (`_pthread_cleanup_push`), newest first.
fn run_old_records() {
    let Ok(d) = thread::desc() else { return };
    loop {
        let rec = d.cleanup_old.load(Acquire);
        if rec == 0 {
            return;
        }
        let w = tb::registered(rec, 4);
        d.cleanup_old.store(word(w, O_PREV), Release);
        call(word(w, O_ROUTINE), word(w, O_ARG));
    }
}

/// The old-style records registered in frames the forced unwind has left: those below `limit`
/// (a canonical frame address the stop function passed), newest first.
fn run_old_records_below(d: &niji_rt::thread::Thread, limit: usize) {
    loop {
        let rec = d.cleanup_old.load(Acquire);
        if rec == 0 || rec >= limit {
            return;
        }
        let w = tb::registered(rec, 4);
        d.cleanup_old.store(word(w, O_PREV), Release);
        call(word(w, O_ROUTINE), word(w, O_ARG));
    }
}

// ------------------------------------------------------------------------------ forced unwind

/// `_UA_END_OF_STACK` (Itanium C++ ABI §1.3): the stop function's last call.
const UA_END_OF_STACK: c_int = 16;
/// The exception class of a thread's exit (vendor `NIJI`, language `EXIT`; ABI §1.2: the high
/// four bytes name the vendor).
const EXIT_CLASS: u64 = u64::from_be_bytes(*b"NIJIEXIT");

fn as_i64(v: usize) -> i64 {
    i64::from_ne_bytes(v.to_ne_bytes())
}

/// The process's unwinder: in a static program, `_Unwind_ForcedUnwind` and `_Unwind_GetCFA` as
/// it links them (machine/unwinder.S, D-22.21: libc.a's weak references; 0 everywhere else);
/// otherwise as the program's own references resolve them (D-22.19, `dlsym(RTLD_DEFAULT, ..)`:
/// `libgcc_s.so.1`, or another unwinder the program loaded), looked up now and never cached (a
/// later `dlopen` or `dlclose` changes the answer). `None` in a process without an unwinder.
fn unwinder() -> Option<(usize, usize)> {
    let force = ffi::niji_machine_unwind_force();
    if force != 0 {
        let cfa = ffi::niji_machine_unwind_get_cfa();
        return (cfa != 0).then_some((force, cfa));
    }
    // The loader's `dlsym` entry, as `niji_dl_sym` calls it: RTLD_DEFAULT (0), no version, the
    // program as the requester (0).
    let entry = crate::posix::s27_dynlink::LOADER_DLSYM.load(Acquire);
    if entry == 0 {
        return None;
    }
    let find = |name: &[u8]| usize::from_ne_bytes(ffi::niji_machine_call4(entry, 0, as_i64(name.as_ptr().expose_provenance()), 0, 0).to_ne_bytes());
    let force = find(b"_Unwind_ForcedUnwind\0");
    if force == 0 {
        return None;
    }
    let cfa = find(b"_Unwind_GetCFA\0");
    (cfa != 0).then_some((force, cfa))
}

/// Unwind the calling thread's stack with the program's unwinder (D-22.19, Itanium C++ ABI §1.3
/// `_Unwind_ForcedUnwind`): every frame's cleanups run (C++ destructors, `catch` handlers that
/// rethrow, `-fexceptions` cleanups), the C cleanup buffers in their frames' turn
/// ([`stop`]), and the exit sequence after the outermost frame. Returns only when there is no
/// unwinder or the unwind could not start; the caller then exits without it. Without an
/// unwinder, on the exit's `first` step, [`no_unwinder`] decides whether it may.
#[cold]
#[inline(never)]
fn forced_unwind(first: bool) {
    let Ok(d) = thread::desc() else { return };
    let Some((force, get_cfa)) = unwinder() else {
        if first {
            no_unwinder();
        }
        return;
    };
    let w = &d.unwind.words;
    set(w, niji_rt::thread::UNWIND_CLASS, usize::from_ne_bytes(EXIT_CLASS.to_ne_bytes()));
    set(w, niji_rt::thread::UNWIND_CLEANUP, swallowed_addr());
    set(w, niji_rt::thread::UNWIND_PRIVATE1, 0);
    set(w, niji_rt::thread::UNWIND_PRIVATE2, 0);
    set(w, niji_rt::thread::UNWIND_LAST_CFA, 0);
    let _ = ffi::niji_machine_call3(force, as_i64(addr(w)), as_i64(stop_addr()), as_i64(get_cfa));
}

/// The stop function (`_Unwind_Stop_Fn`) of [`forced_unwind`], called for each frame before its
/// cleanups run, with `_Unwind_GetCFA` as its parameter. The old-style records of the frames
/// already left run first; then, if the newest cleanup buffer lies in this frame (below its
/// canonical frame address), the unwind continues there (its handler calls
/// `__pthread_unwind_next`, which starts a new unwind). After the outermost frame the exit goes
/// on as without an unwinder. Its own frames hold nothing to drop (S11).
extern "C" fn stop(_version: c_int, actions: c_int, _class: u64, _exc: usize, ctx: usize, get_cfa: usize) -> c_int {
    let Ok(d) = thread::desc() else { return 0 };
    if actions & UA_END_OF_STACK != 0 {
        fallback(d.cleanup_jmp.load(Acquire), d.result.load(Acquire));
    }
    let cfa = usize::from_ne_bytes(ffi::niji_machine_call3(get_cfa, as_i64(ctx), 0, 0).to_ne_bytes());
    run_old_records_below(d, word(&d.unwind.words, niji_rt::thread::UNWIND_LAST_CFA));
    let head = d.cleanup_jmp.load(Acquire);
    if head != 0 && head < cfa {
        // The buffer was filled by `__sigsetjmp(buf, 0)` in this frame, which is still live.
        ffi::niji_machine_siglongjmp(head, 1);
    }
    set(&d.unwind.words, niji_rt::thread::UNWIND_LAST_CFA, cfa);
    0
}

/// The exit without an unwinder (D-22.23, rulings #7118 and #7119): when a frame of the thread's
/// stack has unwind work (`-fexceptions` cleanups, C++ destructors or handlers, found by this
/// library's own call-frame walker), going on would skip it (a mutex a destructor owns stays
/// locked, a promise is never broken), so the process ends: a message naming the cause and the
/// fix on standard error, then `abort` (SIGABRT). A thread with nothing to unwind (C cleanup
/// buffers and old-style records only) goes on to the exit without an unwinder. A program that
/// links its own unwinder hidden (`-static-libgcc`) and loads no `libgcc_s.so.1` lands here.
/// Only in a program this library's `ld.so` started: a static program with unwind work links
/// its unwinder (D-22.21), and a process another loader started (the in-process test runtime)
/// has no lookup to find one.
#[cold]
#[inline(never)]
fn no_unwinder() {
    if crate::posix::s27_dynlink::LOADER_DLSYM.load(Acquire) == 0 {
        return;
    }
    if crate::backtrace::handler_on_stack() {
        niji_rt::fatal::fatal(&CSys, &[NO_UNWINDER], crate::ret::trap)
    }
}

/// [`no_unwinder`]'s message.
const NO_UNWINDER: &[u8] = b"FATAL: an exiting or cancelled thread has frames to unwind (cleanups, destructors or handlers) and no unwinder can be found; link libgcc_s dynamically (not -static-libgcc) in a program whose threads exit or are cancelled\n";

/// The exception's cleanup function: a handler caught the thread's exit and ended without
/// rethrowing it (C++ requires the rethrow), or the exception was deleted. The thread can neither
/// go on (it was told to exit) nor exit from here (the frames outside the handler would not
/// unwind), so the process ends: a fatal diagnostic, then `abort` (D-22.19, ruling #6884).
#[cold]
extern "C" fn swallowed(_reason: c_int, _exc: usize) {
    niji_rt::fatal::fatal(&CSys, &[b"FATAL: exception not rethrown\n"], crate::ret::trap)
}

#[allow(clippy::as_conversions)] // a function pointer's address: the only spelling Rust has
fn stop_addr() -> usize {
    stop as extern "C" fn(c_int, c_int, u64, usize, usize, usize) -> c_int as usize
}

#[allow(clippy::as_conversions)] // a function pointer's address: the only spelling Rust has
fn swallowed_addr() -> usize {
    swallowed as extern "C" fn(c_int, usize) as usize
}

/// A foreign exception (C++) unwound out of the frame that pushed the newest old-style record
/// whose routine is `routine` (machine/once.S, during the unwinder's cleanup phase): the records
/// of the frames it already left (newer ones, now dead stack) are dropped, that record is
/// removed and its routine runs. Nothing happens when no such record is registered.
pub fn unwound_through(routine: usize) {
    let Ok(d) = thread::desc() else { return };
    loop {
        let rec = d.cleanup_old.load(Acquire);
        if rec == 0 {
            return;
        }
        let w = tb::registered(rec, 4);
        d.cleanup_old.store(word(w, O_PREV), Release);
        if word(w, O_ROUTINE) == routine {
            call(routine, word(w, O_ARG));
            return;
        }
    }
}

/// The type word of a guard record (machine/guard.S): `unwound_guard` stops at the newest one.
const GUARD_TAG: usize = 0x0047_5541_5244;

/// machine/guard.S (`niji_guard_begin`), before the body: `record` (the 4-word buffer in that
/// frame) is registered, so that an unwind leaving the frame, or a cancellation acting inside
/// it, runs `undo(ctx)`.
pub fn guard_begin(record: KResult<&[AtomicUsize]>, undo: usize, ctx: usize) {
    push_old(record, undo, ctx);
    if let Ok(b) = record {
        set(b, O_TYPE, GUARD_TAG);
    }
}

/// machine/guard.S (`niji_guard_end`), after the body returned: the record is removed.
pub fn guard_end(record: KResult<&[AtomicUsize]>) {
    pop_old(record, 0);
}

/// The personality routine of `niji_machine_guard`'s frame (`niji_guard_personality`; as
/// [`crate::sync::once_personality`]): in the cleanup phase it removes the newest guard record
/// (and the records of the frames the unwind already left) and runs its undo routine; the
/// unwind goes on. The search phase changes nothing.
pub fn guard_personality(version: c_int, actions: c_int) -> c_int {
    const UA_CLEANUP_PHASE: c_int = 2;
    if version != 1 {
        return if actions & UA_CLEANUP_PHASE != 0 { 2 } else { 3 };
    }
    if actions & UA_CLEANUP_PHASE != 0 {
        unwound_guard();
    }
    8
}

fn unwound_guard() {
    let Ok(d) = thread::desc() else { return };
    loop {
        let rec = d.cleanup_old.load(Acquire);
        if rec == 0 {
            return;
        }
        let w = tb::registered(rec, 4);
        d.cleanup_old.store(word(w, O_PREV), Release);
        if word(w, O_TYPE) == GUARD_TAG {
            call(word(w, O_ROUTINE), word(w, O_ARG));
            return;
        }
    }
}

fn call(f: usize, arg: usize) {
    if f != 0 {
        let _ = ffi::niji_machine_call3(f, i64::from_ne_bytes(arg.to_ne_bytes()), 0, 0);
    }
}

/// `_pthread_cleanup_push(buffer, routine, arg)`.
pub fn push_old(buf: KResult<&[AtomicUsize]>, routine: usize, arg: usize) {
    let (Ok(b), Ok(d)) = (buf, thread::desc()) else { return };
    set(b, O_ROUTINE, routine);
    set(b, O_ARG, arg);
    set(b, O_PREV, d.cleanup_old.load(Acquire));
    d.cleanup_old.store(addr(b), Release);
}

/// `_pthread_cleanup_pop(buffer, execute)`.
pub fn pop_old(buf: KResult<&[AtomicUsize]>, execute: i32) {
    let (Ok(b), Ok(d)) = (buf, thread::desc()) else { return };
    d.cleanup_old.store(word(b, O_PREV), Release);
    if execute != 0 {
        call(word(b, O_ROUTINE), word(b, O_ARG));
    }
}

/// `_pthread_cleanup_push_defer`: as [`push_old`], switching to deferred cancellation.
pub fn push_old_defer(buf: KResult<&[AtomicUsize]>, routine: usize, arg: usize) {
    let Ok(d) = thread::desc() else { return };
    let old = d.cancel.fetch_and(!CANCEL_ASYNC, AcqRel);
    if let Ok(b) = buf {
        set(b, O_TYPE, usize::from(old & CANCEL_ASYNC != 0));
    }
    push_old(buf, routine, arg);
}

/// `_pthread_cleanup_pop_restore`: as [`pop_old`], restoring the saved type.
pub fn pop_old_restore(buf: KResult<&[AtomicUsize]>, execute: i32) {
    let restore = buf.as_ref().map_or(0, |b| word(b, O_TYPE));
    pop_old(buf, execute);
    if restore & 1 != 0 {
        if let Ok(d) = thread::desc() {
            d.cancel.fetch_or(CANCEL_ASYNC, AcqRel);
        }
        test_cancel();
    }
}

/// `__pthread_cleanup_routine(frame)`: the C++/`-fexceptions` form's per-frame call (routine,
/// argument, `__do_it`, `__cancel_type`).
pub fn cleanup_routine(frame: KResult<&[AtomicUsize]>) {
    let Ok(f) = frame else { return };
    if word(f, 2) & 0xffff_ffff != 0 {
        call(word(f, 0), word(f, 1));
    }
}

/// Act on a due cancellation request now (`pthread_testcancel` and the async-cancel-safe
/// calls; filled in with the internal signal's handler).
pub fn test_cancel() {
    if let Ok(d) = thread::desc() {
        if niji_rt::thread::cancel_due(d.cancel.load(Acquire)) {
            unwind(usize::MAX);
        }
    }
}

/// A cancellation point's result (C0.2): if the call found a request due (`CANCEL_HIT`), the
/// thread acts on it now and exits with `PTHREAD_CANCELED`; otherwise the result is returned.
/// The export's own frames hold nothing to drop (S11).
///
/// `CANCEL_HIT` is only ever set by the calling thread itself (the cancellation window's exit in
/// `sys`), never by another thread, so a plain load that sees it clear is final: the atomic
/// clear (a locked read-modify-write, ~20 cycles on every read/write/close) runs only when the
/// bit is set (WL-RESID).
pub fn point<T>(v: T) -> T {
    if let Some(d) = thread::current() {
        if d.cancel.load(Acquire) & CANCEL_HIT != 0 && d.cancel.fetch_and(!CANCEL_HIT, AcqRel) & CANCEL_HIT != 0 {
            unwind(usize::MAX);
        }
    }
    v
}

/// A fast cancellation point (CANCEL-FAST, `crate::sys::point4` and `close_point`) found a
/// request due: `errno` is set from the call's raw result `r` (if it is an error), as the export's
/// result conversion did before [`point`] acted, and the thread acts now.
#[cold]
#[inline(never)]
pub(crate) fn act(r: i64) -> ! {
    if let Err(e) = Errno::decode(r) {
        crate::errno::set(e);
    }
    unwind(usize::MAX)
}

// ------------------------------------------------------------------------------ requests (§22.3)

/// The library's internal signal (D-20.1): cancellation requests, and the setxid broadcast.
pub const SIG_INTERNAL: i32 = 32;

/// The internal signal's handler. A due cancellation request interrupting a cancellation
/// point inside the window moves the saved program counter to the window's cancelled exit, so
/// the call returns `-ECANCELED` with no effect (D-22.11). Asynchronous requests take effect at
/// the thread's next safe point (owner ruling, D-22.11). Async-signal-safe: atomics and the
/// setxid service only.
extern "C" fn internal(_sig: i32, _info: usize, ctx: usize) {
    let Some(d) = thread::current() else { return };
    crate::setxid::serve(d);
    if !niji_rt::thread::cancel_due(d.cancel.load(Acquire)) {
        return;
    }
    let mut b = [0usize; 3];
    ffi::niji_machine_cancel_bounds(&mut b);
    if let Some(pc) = tb::saved_pc(ctx) {
        let at = pc.load(Relaxed);
        if b[0] <= at && at < b[1] {
            pc.store(b[2], Relaxed);
        }
    }
}

#[allow(clippy::as_conversions)] // a function pointer's address: the only spelling Rust has
fn internal_addr() -> usize {
    internal as extern "C" fn(i32, usize, usize) as usize
}

/// Whether the internal signal's handler is installed ([`install`]). A fork child inherits both
/// the handler and this word, so it never installs again.
static INSTALLED: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

/// Install the internal signal's handler, once per process: every signal blocked while it runs,
/// restarting interrupted calls, on the alternate stack if one is set. Lazy (owner ruling #7075,
/// D-20.1): the library sends the signal only to a thread it created (a cancellation request,
/// the setxid broadcast), so thread creation installs it before its first clone and a start
/// makes no `rt_sigaction`. The reservation itself is fixed from the start (OD-8). Racing
/// creators both install the same action, which is harmless.
#[inline]
pub fn install() {
    if !INSTALLED.load(Relaxed) {
        install_now();
    }
}

#[cold]
#[inline(never)]
fn install_now() {
    use niji_kernel::uapi::{SigSet, SA_ONSTACK, SA_RESTART};
    if raw::install_action(SIG_INTERNAL, internal_addr(), SA_RESTART | SA_ONSTACK, SigSet(!0)).is_ok() {
        INSTALLED.store(true, Relaxed);
    }
}

/// `pthread_cancel`: mark the request; a thread that can act is interrupted with the internal
/// signal (its cancellation point then returns without effect). The calling thread with
/// asynchronous cancellation acts at once (`pthread_cancel` is async-cancel-safe).
pub fn request(tp: usize) -> KResult<()> {
    let d = thread::find(tp).ok_or(Errno::ESRCH)?;
    if d.has(niji_rt::thread::DESC_EXITED) {
        return Err(Errno::ESRCH);
    }
    let now = d.cancel.fetch_or(CANCEL_REQUESTED, AcqRel) | CANCEL_REQUESTED;
    if tp == tb::tp() {
        if niji_rt::thread::cancel_due(now) && now & CANCEL_ASYNC != 0 {
            unwind(usize::MAX);
        }
        return Ok(());
    }
    if niji_rt::thread::cancel_due(now) {
        if let Ok(tid) = thread::tid_of(tp) {
            let _ = CSys.tgkill(CSys.getpid(), tid, SIG_INTERNAL);
        }
    }
    Ok(())
}

/// `pthread_setcancelstate`: the previous state (`PTHREAD_CANCEL_ENABLE` 0, `DISABLE` 1).
/// Enabling with asynchronous cancellation and a request pending acts (a safe point).
pub fn set_state(state: i32) -> KResult<i32> {
    let d = thread::desc()?;
    let old = match state {
        0 => d.cancel.fetch_and(!CANCEL_DISABLED, AcqRel),
        1 => d.cancel.fetch_or(CANCEL_DISABLED, AcqRel),
        _ => return Err(Errno::EINVAL),
    };
    if state == 0 && d.cancel.load(Acquire) & CANCEL_ASYNC != 0 {
        test_cancel();
    }
    Ok(i32::from(old & CANCEL_DISABLED != 0))
}

/// `pthread_setcanceltype`: the previous type (`DEFERRED` 0, `ASYNCHRONOUS` 1). Switching to
/// asynchronous with a request pending acts (a safe point).
pub fn set_type(kind: i32) -> KResult<i32> {
    let d = thread::desc()?;
    let old = match kind {
        0 => d.cancel.fetch_and(!CANCEL_ASYNC, AcqRel),
        1 => d.cancel.fetch_or(CANCEL_ASYNC, AcqRel),
        _ => return Err(Errno::EINVAL),
    };
    if kind == 1 {
        test_cancel();
    }
    Ok(i32::from(old & CANCEL_ASYNC != 0))
}

/// `pthread_kill`: signal 0 tests for existence; the internal signal is `EINVAL`. A thread that
/// has exited but not been joined (still listed) gives 0 and is sent nothing (§20.4, 2.34+),
/// also when it exits between the lookup and `tgkill`.
pub fn kill(tp: usize, sig: i32) -> KResult<()> {
    if sig != 0 {
        niji_legacy::signal_compat::check(sig)?;
    }
    let exited = |e: Errno| if e == Errno::ESRCH && thread::find(tp).is_some() { Ok(()) } else { Err(e) };
    let tid = match thread::tid_of(tp) {
        Ok(tid) => tid,
        Err(e) => return exited(e),
    };
    if sig == 0 {
        return Ok(());
    }
    CSys.tgkill(CSys.getpid(), tid, sig).or_else(exited)
}

/// `pthread_sigqueue`: the signal with a value (`SI_QUEUE`) to one thread.
pub fn sigqueue(tp: usize, sig: i32, value: u64) -> KResult<()> {
    if sig != 0 {
        niji_legacy::signal_compat::check(sig)?;
    }
    let tid = thread::tid_of(tp)?;
    let info = niji_kernel::uapi::SigInfo::queued(sig, CSys.getpid(), CSys.getuid(), value);
    CSys.tgsigqueueinfo(CSys.getpid(), tid, sig, &info)
}
