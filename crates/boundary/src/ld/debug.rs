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

use core::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use core::sync::atomic::{AtomicI32, AtomicUsize};

use niji_ld::debug::{State, BRK, LDBASE, MAP, NEXT, SIZE, SIZE_EXT, STATE, VERSION, VERSION_COPY};

use super::{_dl_debug_state, _r_debug};

/// `<link.h>`'s `struct r_debug` (the first 40 bytes of the rendezvous), every field an atomic: a
/// program reads it while another thread's `dlopen` writes it. A program's copy of `_r_debug` made
/// against the 40-byte symbol is only this.
#[repr(C)]
pub struct RDebugBase {
    r_version: AtomicI32,
    r_map: AtomicUsize,
    r_brk: AtomicUsize,
    r_state: AtomicI32,
    r_ldbase: AtomicUsize,
}

/// `ld.so`'s `_r_debug`: `r_debug_extended` (version 2), the base structure and `r_next`.
#[repr(C)]
pub struct RDebug {
    base: RDebugBase,
    /// The next namespace's rendezvous; null: this is the only (base) namespace. Never written
    /// while `dlmopen` (DL-22) is absent.
    r_next: AtomicUsize,
}

const _: () = {
    use core::mem::offset_of;
    assert!(size_of::<RDebugBase>() == SIZE && core::mem::align_of::<RDebugBase>() == 8);
    assert!(offset_of!(RDebugBase, r_map) == MAP && offset_of!(RDebugBase, r_brk) == BRK);
    assert!(offset_of!(RDebugBase, r_state) == STATE && offset_of!(RDebugBase, r_ldbase) == LDBASE);
    assert!(size_of::<RDebug>() == SIZE_EXT && offset_of!(RDebug, r_next) == NEXT && offset_of!(RDebug, base) == 0);
};

impl RDebug {
    /// The zero structure `ld.so`'s `_r_debug` starts as ([`super::_r_debug`]).
    pub(super) const fn new() -> Self {
        Self {
            base: RDebugBase { r_version: AtomicI32::new(0), r_map: AtomicUsize::new(0), r_brk: AtomicUsize::new(0), r_state: AtomicI32::new(0), r_ldbase: AtomicUsize::new(0) },
            r_next: AtomicUsize::new(0),
        }
    }
}

/// `<link.h>`'s public `struct link_map` prefix, for the one link map that is not an object the
/// loader maps: the vDSO's.
#[repr(C)]
struct LinkMap {
    l_addr: AtomicUsize,
    l_name: AtomicUsize,
    l_ld: AtomicUsize,
    l_next: AtomicUsize,
    l_prev: AtomicUsize,
}

/// The vDSO's link map (probe: the reference lists it second, as `linux-vdso.so.1`, as
/// `dl_iterate_phdr` does, P-12). It stays in the list for the process's lifetime.
static VDSO_MAP: LinkMap = LinkMap {
    l_addr: AtomicUsize::new(0),
    l_name: AtomicUsize::new(0),
    l_ld: AtomicUsize::new(0),
    l_next: AtomicUsize::new(0),
    l_prev: AtomicUsize::new(0),
};

/// Fill the vDSO's link map (the image at `vdso`, `AT_SYSINFO_EHDR`) after `prev`, the program's
/// link map; its `l_next` is set by [`vdso_next`]. Its address.
pub(super) fn vdso_node(vdso: usize, prev: usize) -> usize {
    use crate::posix::s27_dynlink::{vdso_dynamic, vdso_headers};
    let (phdr, phnum, bias) = vdso_headers(vdso);
    let m = &VDSO_MAP;
    m.l_addr.store(bias, Relaxed);
    m.l_name.store(c"linux-vdso.so.1".as_ptr().expose_provenance(), Relaxed);
    m.l_ld.store(vdso_dynamic(phdr, phnum, bias), Relaxed);
    m.l_prev.store(prev, Relaxed);
    core::ptr::from_ref(m).expose_provenance()
}

/// Link `next` after the vDSO's link map.
pub(super) fn vdso_next(next: usize) {
    VDSO_MAP.l_next.store(next, Release);
}

/// `at`, a link map's `l_next`, or when `at` is the vDSO's link map, the one after it: the
/// loader's walks visit its own nodes only.
pub(super) fn past_vdso(at: usize) -> usize {
    if at == core::ptr::from_ref(&VDSO_MAP).expose_provenance() {
        return VDSO_MAP.l_next.load(Acquire);
    }
    at
}

/// The program's copy of `_r_debug` (0: none), from its `COPY` relocation.
static COPY: AtomicUsize = AtomicUsize::new(0);

/// The size of that copy: [`SIZE`] (made against the reference's symbol: no `r_next`) or [`SIZE_EXT`].
static COPY_LEN: AtomicUsize = AtomicUsize::new(0);

fn own() -> usize {
    core::ptr::from_ref(&_r_debug).expose_provenance()
}

/// A `COPY` relocation wrote `len` bytes from `from` to `to`: when it copied `_r_debug`, `to` is
/// the program's copy, written from now on as well (once [`consistent`] checked it). Out of the
/// relocator's line: a `COPY` relocation is rare.
#[cold]
#[inline(never)]
pub(super) fn note_copy(to: usize, from: usize, len: usize) {
    if from == own() && (len == SIZE || len == SIZE_EXT) {
        COPY_LEN.store(len, Relaxed);
        COPY.store(to, Relaxed);
    }
}

/// The program's copy, when there is one.
fn copy() -> Option<&'static RDebugBase> {
    let at = COPY.load(Relaxed);
    if at == 0 {
        return None;
    }
    // UNSAFE(slice): the program's copy of `_r_debug`.
    #[allow(unsafe_code)]
    // SAFETY: `at` is the destination of a COPY relocation of `_r_debug` (`note_copy`): the relocator
    // checked [at, at + len) lies in a writable segment of the program (len at least 40), and `consistent` kept it only
    // when it is 8-byte aligned and outside the program's RELRO range (never made read-only). The
    // program's memory stays mapped for the process; its fields are written only as atomics.
    Some(unsafe { &*core::ptr::with_exposed_provenance::<RDebugBase>(at) })
}

/// Every place the rendezvous lives: `ld.so`'s own, and the program's copy.
fn each(mut f: impl FnMut(&RDebugBase)) {
    f(&_r_debug.base);
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
/// call `r_brk` with `RT_ADD`. Run once.
#[cold]
#[inline(never)]
pub(super) fn start(map: usize, ldbase: usize, dt_debug: usize) {
    let r = &_r_debug.base;
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
    let len = COPY_LEN.load(Relaxed);
    let end = at.saturating_add(len);
    if at % 8 != 0 || (at < hi && lo < end) {
        COPY.store(0, Relaxed);
        return;
    }
    if let Some(c) = copy() {
        let r = &_r_debug.base;
        // Without room for `r_next` the copy is the version-1 structure (a reader of version 2
        // would read the program's next variable as `r_next`).
        c.r_version.store(if len == SIZE { VERSION_COPY } else { r.r_version.load(Relaxed) }, Relaxed);
        c.r_map.store(r.r_map.load(Relaxed), Relaxed);
        c.r_brk.store(r.r_brk.load(Relaxed), Relaxed);
        c.r_ldbase.store(r.r_ldbase.load(Relaxed), Relaxed);
    }
}
