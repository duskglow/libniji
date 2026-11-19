//! The debugger rendezvous (phase 14 DL-23; plan/ph14/design.md §13, M14-9; spec s27_4_c_0;
//! the values are `niji_ld::debug`'s). `ld.so` exports `_r_debug` (`GLIBC_2.2.5`) and
//! `_dl_debug_state` (`GLIBC_PRIVATE`), the function a debugger sets its breakpoint on.
//!
//! * **From the start.** [`super::link`] links the start-time objects' link maps (program first,
//!   then load order, `ld.so` at its place or last; [`super::dlopen::start_node`]), then
//!   [`start`] fills the structure, points the program's `DT_DEBUG` at it and calls `r_brk` with
//!   `RT_ADD`; [`consistent`] calls it with `RT_CONSISTENT` once relocation and RELRO are done,
//!   before any constructor. `dlopen` and `dlclose` call [`change`] around each change of the list
//!   (under the loader's lock), so `r_state` is `RT_CONSISTENT` only while the list is.
//! * **A program's copy.** A program that refers to `_r_debug` directly gets a copy of it by a
//!   `COPY` relocation ([`note_copy`] sees it). Every later write goes to `ld.so`'s own structure
//!   (where `DT_DEBUG` points) and to that copy, so both read the same at every `r_brk` call.

use core::sync::atomic::Ordering::{Relaxed, Release, SeqCst};
use core::sync::atomic::{compiler_fence, AtomicI32, AtomicUsize};

use niji_ld::debug::{State, BRK, LDBASE, MAP, SIZE, STATE, VERSION};

/// `<link.h>`'s `struct r_debug` (version 1), every field an atomic: a program reads it while
/// another thread's `dlopen` writes it.
#[repr(C)]
pub struct RDebug {
    r_version: AtomicI32,
    r_map: AtomicUsize,
    r_brk: AtomicUsize,
    r_state: AtomicI32,
    r_ldbase: AtomicUsize,
}

const _: () = {
    use core::mem::offset_of;
    assert!(size_of::<RDebug>() == SIZE && core::mem::align_of::<RDebug>() == 8);
    assert!(offset_of!(RDebug, r_map) == MAP && offset_of!(RDebug, r_brk) == BRK);
    assert!(offset_of!(RDebug, r_state) == STATE && offset_of!(RDebug, r_ldbase) == LDBASE);
};

/// `_r_debug` (spec s27_4_c_0): the rendezvous `DT_DEBUG` points at.
// UNSAFE(export): an exported data object of the ld.so product.
#[unsafe(no_mangle)]
#[allow(unsafe_code)]
#[allow(non_upper_case_globals)]
pub static _r_debug: RDebug = RDebug {
    r_version: AtomicI32::new(0),
    r_map: AtomicUsize::new(0),
    r_brk: AtomicUsize::new(0),
    r_state: AtomicI32::new(0),
    r_ldbase: AtomicUsize::new(0),
};

/// `_dl_debug_state` (spec s27_4_c_0): `r_brk`, called after each `r_state` change; it does nothing,
/// and is never inlined, so a debugger's breakpoint on it stops at every change.
// UNSAFE(export): an exported function of the ld.so product.
#[unsafe(no_mangle)]
#[allow(unsafe_code)]
#[inline(never)]
pub extern "C" fn _dl_debug_state() {
    compiler_fence(SeqCst);
}

/// The program's copy of `_r_debug` (0: none), from its `COPY` relocation.
static COPY: AtomicUsize = AtomicUsize::new(0);

fn own() -> usize {
    core::ptr::from_ref(&_r_debug).expose_provenance()
}

/// A `COPY` relocation wrote `len` bytes from `from` to `to`: when it copied `_r_debug`, `to` is
/// the program's copy, written from now on as well (once [`consistent`] checked it).
pub(super) fn note_copy(to: usize, from: usize, len: usize) {
    if from == own() && len == SIZE {
        COPY.store(to, Relaxed);
    }
}

/// The program's copy, when there is one.
fn copy() -> Option<&'static RDebug> {
    let at = COPY.load(Relaxed);
    if at == 0 {
        return None;
    }
    // UNSAFE(slice): the program's copy of `_r_debug`.
    #[allow(unsafe_code)]
    // SAFETY: `at` is the destination of a COPY relocation of `_r_debug` (`note_copy`): the relocator
    // checked [at, at + 40) lies in a writable segment of the program, and `consistent` kept it only
    // when it is 8-byte aligned and outside the program's RELRO range (never made read-only). The
    // program's memory stays mapped for the process; its fields are written only as atomics.
    Some(unsafe { &*core::ptr::with_exposed_provenance::<RDebug>(at) })
}

/// Every place the rendezvous lives: `ld.so`'s own, and the program's copy.
fn each(mut f: impl FnMut(&RDebug)) {
    f(&_r_debug);
    if let Some(c) = copy() {
        f(c);
    }
}

/// Call `r_brk` with `state` (`dlopen`, `dlclose`, the start): the list writes before it are
/// published with the state.
pub(super) fn change(state: State) {
    each(|r| r.r_state.store(state.word(), Release));
    _dl_debug_state();
}

/// The start, once the start-time list is linked from `map` (node 0, the program) and before any
/// object is relocated: fill the rendezvous, point the program's `DT_DEBUG` entry at it (`dt_debug`:
/// the entry's value word, 0 when the program has none or its dynamic array is not writable) and
/// call `r_brk` with `RT_ADD`.
pub(super) fn start(map: usize, ldbase: usize, dt_debug: usize) {
    let r = &_r_debug;
    r.r_version.store(VERSION, Relaxed);
    r.r_map.store(map, Relaxed);
    r.r_brk.store(super::fn_addr0(_dl_debug_state), Relaxed);
    r.r_ldbase.store(ldbase, Relaxed);
    if dt_debug != 0 {
        // UNSAFE(slice): the program's DT_DEBUG value word.
        #[allow(unsafe_code)]
        // SAFETY: `dt_debug` is the value word of a DT_DEBUG entry of the program's dynamic array,
        // which `super::link` found inside a writable segment of the program (8-byte aligned, as
        // every dynamic entry is); RELRO is not applied yet. One thread runs here.
        unsafe { &*core::ptr::with_exposed_provenance::<AtomicUsize>(dt_debug) }.store(own(), Relaxed);
    }
    change(State::Add);
}

/// After relocation and RELRO, while the program's headers are still readable: keep the program's
/// copy when it can be written for the process's lifetime (`relro` gives the program's RELRO range,
/// `[lo, hi)`; asked only when there is a copy) and bring it up to date.
pub(super) fn keep_copy(relro: impl FnOnce() -> (usize, usize)) {
    let at = COPY.load(Relaxed);
    if at != 0 {
        check_copy(at, relro());
    }
}

/// The start-time list is complete and relocated (before any constructor): call `r_brk` with
/// `RT_CONSISTENT`.
pub(super) fn consistent() {
    change(State::Consistent);
}

/// The rare program with a copy (a `COPY` relocation of `_r_debug`): dropped when it is misaligned
/// or in the program's RELRO range, else filled from `ld.so`'s own.
#[cold]
#[inline(never)]
fn check_copy(at: usize, (lo, hi): (usize, usize)) {
    let end = at.saturating_add(SIZE);
    if at % 8 != 0 || (at < hi && lo < end) {
        COPY.store(0, Relaxed);
        return;
    }
    if let Some(c) = copy() {
        let r = &_r_debug;
        c.r_version.store(r.r_version.load(Relaxed), Relaxed);
        c.r_map.store(r.r_map.load(Relaxed), Relaxed);
        c.r_brk.store(r.r_brk.load(Relaxed), Relaxed);
        c.r_ldbase.store(r.r_ldbase.load(Relaxed), Relaxed);
    }
}
