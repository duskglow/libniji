//! The loader's lock-free readers (phase 14 DL-16c `DL-DLOPEN-CONCURRENT`; plan/ph14/dl-16-dlopen.md
//! §4; D-27.1, D-27.30). `dl_iterate_phdr` and `_dl_find_object` of a program `ld.so` started reach
//! [`iterate`] and [`find_object`] through `StartInfo::dl_iterate` and `dl_find_object`.
//!
//! * **The table.** One entry per loaded object in chain order (node `i` of `super::dlopen` is
//!   entry `i`): load bias, name, program header table and count. Append-only like the chain: a
//!   `dlopen` writes its new objects' entries while they are invisible ([`stage`]) and makes them
//!   visible with one Release store of [`PUBLISHED`] at the set's publication point, after pass 2
//!   and before the constructors ([`publish`]). Readers load [`PUBLISHED`] with Acquire and read
//!   only entries below it, which never change again; they never form a reference to the
//!   writer's `Loader`, so a `dlopen` in progress cannot be observed half done.
//! * **The start-time objects** fill their entries at the first reader or `dlopen` (nothing at
//!   start): every filler stores the same words, so racing first readers need no lock.
//! * **The walk is live** (probe `tools/probe/dl/readers`, R-2): each step reloads [`PUBLISHED`]
//!   and the counters, so a callback that calls `dlopen` sees the new object later in the same
//!   walk, and the callback runs without any lock (ruling #2607; s27_1_c_8 says the reference
//!   holds one: libniji's callbacks never block a `dlopen` in another thread).
//! * **Counters** (dynlink.md `== phdr`): `dlpi_adds` counts every object ever mapped (the start
//!   objects and the vDSO, then each object a `dlopen` maps, also in an attempt that fails),
//!   `dlpi_subs` every object unmapped (a failed attempt's).
//! * **Unloading** (DL-17, handoff/DL-DLCLOSE.md): `dlclose` makes an entry invisible again
//!   ([`unpublish`]: its live word to 0, `dlpi_subs` moves) and its object is unmapped only after every
//!   reader that may have seen it has left: every read goes through [`read_section`], which counts
//!   the reader in a two-parity counter striped by thread pointer; the writer flips the parity
//!   ([`flip`]) after unpublishing and unmaps once the old parity's counters are drained
//!   ([`drained`]), never waiting under the loader's locks (a callback may call `dlclose`). A slot
//!   is reused only after that. Past a retired threshold a `dlopen`/`dlclose` waits for the drain
//!   at its entry, holding no lock, unless its thread is itself inside a read section ([`inside`],
//!   the thread block's `dl_reading` counts) or holds the writer lock (ruling #5651).

use super::dlopen::{node_address, reader_entry, CAP};
use super::index::Found;
use super::{read_auxv, read_image, AUXV_WORDS_MAX, OWN_BASE, START_ARGS, START_INTERP, START_N, START_PAGE, START_RUN_PATH, THREAD_MODS_LEN};
use crate::posix::s27_dynlink::{find_in, int_result, vdso_headers, Extent};
use crate::{conv, tb};
use core::sync::atomic::Ordering::{Acquire, Relaxed, Release, SeqCst};
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize};
use niji_abi::layout::{struct_dl_find_object, struct_dl_phdr_info};
use niji_kernel::auxv::Auxv;
use niji_kernel::uapi::AT_SYSINFO_EHDR;

const PT_LOAD: u64 = 1;
const PT_DYNAMIC: u64 = 2;
const PF_W: u64 = 2;
const PT_TLS: u64 = 7;
const DT_STRTAB: u64 = 5;
const DT_SONAME: u64 = 14;

/// Per object: `[load bias, name address, program header table address, header count, live]`; a
/// reader reads the first four words only after it saw `live` set (Acquire), and they do not change
/// until the slot is reused, after a grace period (DL-17). `live` is 0, [`LIVE`] or [`LIVE_TLS`]
/// (the object has a `PT_TLS` header: the walk looks for its module only then).
/// The first [`CAP`] entries (generation 0); later ones are in generations the loader maps and
/// lists in its directory before staging them (DL-PROCESS-OBJECTS part 2, [`slot`]).
static TABLE: [[AtomicUsize; 5]; CAP] = [const { [const { AtomicUsize::new(0) }; 5] }; CAP];
/// Bytes of one entry of the readers' table.
pub(super) const ENTRY_BYTES: usize = size_of::<[AtomicUsize; 5]>();
/// The `live` word of a published object without a `PT_TLS` header.
const LIVE: usize = 1;
/// The `live` word of a published object with a `PT_TLS` header.
const LIVE_TLS: usize = 3;

/// The `live` word for an object with these program headers (written once, at its publication).
fn live_word(phdr: usize, phnum: usize) -> usize {
    if headers(phdr, phnum).any(|h| h.0 == PT_TLS) {
        LIVE_TLS
    } else {
        LIVE
    }
}

/// Entry `i` of the readers' table: below [`CAP`] the static table, else one directory load; `None`
/// when its generation is not mapped.
fn slot(i: usize) -> Option<&'static [AtomicUsize; 5]> {
    if let Some(e) = TABLE.get(i) {
        return Some(e);
    }
    let at = reader_entry(i);
    if at == 0 {
        return None;
    }
    // UNSAFE(slice): one entry of a mapped generation of the readers' table.
    #[allow(unsafe_code)]
    // SAFETY: `at` is entry `i` of a generation the loader mapped (read-write, page aligned, never
    // moved or unmapped) and published in its directory with Release before staging any entry in
    // it; the entry is five atomic words (all-zero valid), readable and writable through a shared
    // reference by the writer and every reader.
    Some(unsafe { &*core::ptr::with_exposed_provenance::<[AtomicUsize; 5]>(at) })
}
/// Entries a reader looks at (0: the start-time entries are not filled yet).
static PUBLISHED: AtomicUsize = AtomicUsize::new(0);

/// [`PUBLISHED`] now: a bound on the objects a walk visits (`dladdr` picks the walk or the index).
pub(super) fn published() -> usize {
    PUBLISHED.load(Relaxed)
}
/// The reader parity (DL-17): bit 0 says which counter of its stripe a reader entering now uses.
static PARITY: AtomicUsize = AtomicUsize::new(0);
/// Readers inside [`read_section`], per parity, striped by thread pointer over cache lines.
#[repr(align(64))]
struct Stripe([AtomicUsize; 2]);
const STRIPES: usize = 32;
static READERS: [Stripe; STRIPES] = [const { Stripe([const { AtomicUsize::new(0) }; 2]) }; STRIPES];
/// Objects `dlopen` calls have made visible or mapped and unmapped again ([`publish`],
/// [`count_unmapped`]); the start-time objects are counted apart.
static ADDS: AtomicU64 = AtomicU64::new(0);
static SUBS: AtomicU64 = AtomicU64::new(0);
/// The vDSO's address (`AT_SYSINFO_EHDR`, 0: none); `usize::MAX` until read.
static VDSO: AtomicUsize = AtomicUsize::new(usize::MAX);

/// What a reader sees: the published objects and the counters. The reader is counted for the whole
/// closure (module documentation), so nothing it may see is unmapped before it leaves. A callback
/// that leaves by `longjmp` stays counted: the objects closed meanwhile are then never unmapped (a
/// leak, never a fault), and past the retired threshold a later `dlopen`/`dlclose` waits for it.
pub(super) fn read_section<R>(f: impl FnOnce(&View) -> R) -> R {
    let Some(stripe) = own_stripe() else { return f(&View { start: fill_start() }) };
    let mut p = PARITY.load(SeqCst) & 1;
    loop {
        let Some(c) = stripe.0.get(p) else { return f(&View { start: fill_start() }) };
        c.fetch_add(1, SeqCst);
        // A flip between the load and the count: leave and enter with the new parity (the writer
        // may already have found the old one drained).
        let q = PARITY.load(SeqCst) & 1;
        if q == p {
            break;
        }
        c.fetch_sub(1, Release);
        p = q;
    }
    // The thread's own count for that parity ([`inside`], [`after_fork_child`]): written by this
    // thread only.
    let own = tb::current().dl_reading.get(p);
    if let Some(o) = own {
        o.store(o.load(Relaxed).wrapping_add(1), Relaxed);
    }
    let r = f(&View { start: fill_start() });
    if let Some(o) = own {
        o.store(o.load(Relaxed).wrapping_sub(1), Relaxed);
    }
    if let Some(c) = stripe.0.get(p) {
        c.fetch_sub(1, Release);
    }
    r
}

/// The calling thread's stripe: Fibonacci hashing of the thread pointer, whose top 5 bits pick one
/// of the 32.
fn own_stripe() -> Option<&'static Stripe> {
    let h = (u64::try_from(tb::tp()).unwrap_or(0) >> 6).wrapping_mul(0x9e37_79b9_7f4a_7c15) >> 59;
    READERS.get(usize::try_from(h).unwrap_or(0) % STRIPES)
}

/// Whether the calling thread is inside a [`read_section`] (a `dl_iterate_phdr` callback, at any
/// depth). Such a thread must never wait for [`drained`]: it may be the reader it waits for.
pub(super) fn inside() -> bool {
    tb::current().dl_reading.iter().any(|o| o.load(Relaxed) != 0)
}

/// In a `fork` child (its only thread is the caller): the readers other threads counted are gone
/// with them. Every count is cleared, then the caller's own read sections (it may fork from inside
/// a callback) are counted again from its thread block. Otherwise a parity could never drain
/// again: every unloaded object would stay mapped, and a `dlopen` past the retired threshold would
/// wait for ever.
pub(super) fn after_fork_child() {
    for s in &READERS {
        for c in &s.0 {
            c.store(0, Relaxed);
        }
    }
    if let Some(s) = own_stripe() {
        for (c, o) in s.0.iter().zip(&tb::current().dl_reading) {
            c.store(o.load(Relaxed), SeqCst);
        }
    }
}

/// The writer's step (under the loader's locks, only once [`drained`]): readers entering from now on
/// count in the other parity and cannot see what was unpublished before.
pub(super) fn flip() {
    PARITY.fetch_add(1, SeqCst);
}

/// Whether every reader that entered before the last [`flip`] has left: the parity readers no
/// longer enter with is empty. What was unpublished before that flip may then be unmapped, and only
/// then may the parity flip again: a second flip while that parity still counts an early reader
/// would make new readers count in it too, and that reader would be taken for a new one.
pub(super) fn drained() -> bool {
    let old = (PARITY.load(SeqCst) & 1) ^ 1;
    READERS.iter().all(|s| s.0.get(old).is_none_or(|c| c.load(SeqCst) == 0))
}

/// A reader's view (see [`read_section`]).
pub(super) struct View {
    /// The start-time entries (the program, its dependencies, `ld.so`); 0 when `link` did not run.
    start: usize,
}

/// One published object.
#[derive(Clone, Copy)]
struct Entry {
    bias: usize,
    name: usize,
    phdr: usize,
    phnum: usize,
    /// The `live` word ([`LIVE`] or [`LIVE_TLS`]).
    live: usize,
}

impl View {
    /// Entries visible now (reloaded at every step: the walk is live).
    fn len(&self) -> usize {
        PUBLISHED.load(Acquire)
    }

    /// Entries of generation 0 visible now (the walks' first loop: the index is tested against
    /// [`CAP`] in the loop condition, so [`View::entry`] needs no test of its own).
    fn len0(&self) -> usize {
        self.len().min(CAP)
    }

    /// Entry `i` of generation 0, below [`View::len0`], when it is live.
    #[inline(always)]
    fn entry(&self, i: usize) -> Option<Entry> {
        Self::read(TABLE.get(i)?)
    }

    /// Entry `i` past generation 0, below [`View::len`], when it is live.
    #[inline(never)]
    fn entry_past(&self, i: usize) -> Option<Entry> {
        Self::read(slot(i)?)
    }

    /// The words of a live entry.
    #[inline(always)]
    fn read(e: &[AtomicUsize; 5]) -> Option<Entry> {
        let live = e.get(4)?.load(Acquire);
        if live == 0 {
            return None;
        }
        let w: [usize; 4] = core::array::from_fn(|j| e.get(j).map_or(0, |x| x.load(Relaxed)));
        Some(Entry { bias: w[0], name: w[1], phdr: w[2], phnum: w[3], live })
    }

    /// The start-time objects and the vDSO: the part of `dlpi_adds` that never changes (a walk
    /// reads it once, [`View::counters`] adds the `dlopen`s' at every step).
    fn start_adds(&self) -> u64 {
        u64::try_from(self.start).unwrap_or(0).saturating_add(u64::from(vdso() != 0))
    }

    /// `(dlpi_adds, dlpi_subs)` now, from [`View::start_adds`]: the start-time objects and the vDSO,
    /// then the `dlopen`s'.
    #[inline(always)]
    fn counters(start: u64) -> (u64, u64) {
        (start.saturating_add(ADDS.load(Acquire)), SUBS.load(Acquire))
    }
}

/// The vDSO's address from the start's auxiliary vector (0: none).
fn vdso() -> usize {
    let v = VDSO.load(Relaxed);
    if v != usize::MAX {
        return v;
    }
    let auxv = START_ARGS.get(3).map_or(0, |w| w.load(Relaxed));
    let mut copy = [0u64; AUXV_WORDS_MAX];
    let words = if auxv == 0 { 0 } else { read_auxv(auxv, &mut copy).unwrap_or(0) };
    let aux = Auxv::from_words(copy.get(..words).unwrap_or(&[]));
    let v = aux.get(AT_SYSINFO_EHDR).and_then(|v| usize::try_from(v).ok()).unwrap_or(0);
    VDSO.store(v, Relaxed);
    v
}

/// The count of start-time entries, filling them first (`super::index`).
pub(super) fn start_count() -> usize {
    fill_start()
}

/// Entry `i` when it is live: `(load bias, program header table, header count)` (`super::index`;
/// inside a read section, or under the loader's lock).
pub(super) fn live(i: usize) -> Option<(usize, usize, usize)> {
    let e = View::read(slot(i)?)?;
    Some((e.bias, e.phdr, e.phnum))
}

/// The program headers of an object as `(p_type, p_vaddr, p_memsz, p_flags)`.
pub(super) fn headers(phdr: usize, phnum: usize) -> impl Iterator<Item = (u64, u64, u64, u64)> {
    // UNSAFE(slice): a loaded object's program header table (⚑ KERN for the program, `map::load`
    // or the start's loader for the others), mapped while a reader may see its entry (DL-17: unmapped
    // only after the readers that saw it left, `read_section`).
    #[allow(unsafe_code)]
    // SAFETY: `phdr`/`phnum` come from a live entry read inside `read_section`: the object stays
    // mapped until this reader has left.
    let w = unsafe { conv::words_at(phdr, phnum.saturating_mul(7)) };
    w.chunks_exact(7).map(|h| {
        let w0 = h.first().copied().unwrap_or(0);
        (w0 & 0xffff_ffff, h.get(2).copied().unwrap_or(0), h.get(5).copied().unwrap_or(0), w0 >> 32)
    })
}

/// How the lookups that resolve an address to an object (`_dl_find_object`, `dladdr`, the index)
/// match entry `i` (`super::index`). The kernel mapped the program it started and `ld.so` itself
/// and left the pages between their segment runs unmapped, so another object or mapping may lie
/// there (probe R-6, `spec/behaviour/dynlink.md`): those match by the run that holds the address.
/// Every other object `map::load` mapped in one reservation that keeps its gaps: its whole range
/// (probe R-5).
pub(super) fn extent(i: usize) -> Extent {
    let n = START_N.load(Relaxed);
    let interp = START_INTERP.load(Relaxed);
    let own = if interp < n { interp } else { n };
    let kernel = n != 0 && ((i == 0 && START_RUN_PATH.load(Relaxed) == 0) || i == own);
    if kernel {
        Extent::Run(usize::try_from(START_PAGE.load(Relaxed)).unwrap_or(4096).max(1))
    } else {
        Extent::Whole
    }
}

/// Whether object `i`'s [`extent`] holds `a`, for a caller that found `a` inside its whole range
/// (`super::dlopen::dladdr`): always for a whole-range object; else its run that holds `a`.
pub(super) fn extent_holds(i: usize, a: usize) -> bool {
    match extent(i) {
        Extent::Whole => true,
        // A segment that holds `a` (the common case, one pass), else the run's gaps.
        e => live(i).is_some_and(|(bias, phdr, phnum)| {
            let at = |v: u64| bias.wrapping_add(usize::try_from(v).unwrap_or(0));
            headers(phdr, phnum).any(|h| h.0 == PT_LOAD && at(h.1) <= a && a < at(h.1).wrapping_add(usize::try_from(h.2).unwrap_or(0)))
                || find_in(phdr, phnum, bias, a, e).is_some()
        }),
    }
}

/// The `DT_SONAME` of the object (`bias`, program headers at `phdr`) as a run-time address, from its
/// dynamic section in memory (its `DT_STRTAB` a run-time address when the array is published,
/// [`niji_ld::elf::dynamic_published`], a link-time one otherwise); 0: none.
fn soname_at(bias: usize, phdr: usize, phnum: usize) -> usize {
    let Some((_, vaddr, memsz, _)) = headers(phdr, phnum).find(|h| h.0 == PT_DYNAMIC) else { return 0 };
    // A published array (DL-DYNAMIC-RELOC: every start-time object's once `link` ended) holds
    // `DT_STRTAB` as a run-time address already.
    let writable = headers(phdr, phnum).filter(|h| h.0 == PT_LOAD && h.3 & PF_W != 0).map(|h| (h.1, h.2));
    let strtab_bias = if niji_ld::elf::dynamic_published(vaddr, memsz, writable) { 0 } else { bias };
    let at = bias.wrapping_add(usize::try_from(vaddr).unwrap_or(0));
    // UNSAFE(slice): the object's dynamic section, inside one of its mapped PT_LOADs.
    #[allow(unsafe_code)]
    // SAFETY: PT_DYNAMIC of a start-time object, which `link` read and relocated; `memsz / 8` words.
    let d = unsafe { conv::words_at(at, usize::try_from(memsz / 8).unwrap_or(0)) };
    let (mut strtab, mut soname) = (None, None);
    for e in d.chunks_exact(2) {
        match e.first().copied().unwrap_or(0) {
            0 => break,
            DT_STRTAB => strtab = e.get(1).copied(),
            DT_SONAME => soname = e.get(1).copied(),
            _ => {}
        }
    }
    match (strtab, soname) {
        (Some(s), Some(o)) => strtab_bias.wrapping_add(usize::try_from(s.wrapping_add(o)).unwrap_or(0)),
        _ => 0,
    }
}

/// Fill the start-time entries (the order of `super::dlopen::adopt_start`: the objects `link`
/// recorded, `ld.so` at its index or after them) and publish them unless more is published: the
/// count of start-time entries (0: `link` did not run). Idempotent and lock-free.
fn fill_start() -> usize {
    let n = START_N.load(Acquire);
    if n == 0 || n >= CAP {
        return 0;
    }
    let interp = START_INTERP.load(Relaxed);
    let total = if interp < n { n } else { n.saturating_add(1) };
    if PUBLISHED.load(Acquire) != 0 {
        return total;
    }
    for k in 0..total {
        let (phdr, phnum, bias) = if k == interp || k == n {
            let base = OWN_BASE.load(Relaxed);
            let mut header = [0u8; niji_ld::elf::EHDR_SIZE];
            read_image(base, &mut header);
            let (phoff, phnum) = niji_ld::boot::own_headers(&header).unwrap_or((0, 0));
            (base.wrapping_add(usize::try_from(phoff).unwrap_or(0)), phnum, base)
        } else {
            let w = super::start_obj(k).map(|x| usize::try_from(x).unwrap_or(0));
            (w[0], w[1], w[2])
        };
        // The link map's name (DL-23: the rendezvous and this walk agree); `DT_SONAME` if none.
        let linked = super::dlopen::start_name(k);
        let name = if k == 0 { c"".as_ptr().expose_provenance() } else if linked != 0 { linked } else { soname_at(bias, phdr, phnum) };
        let name = if name == 0 { c"".as_ptr().expose_provenance() } else { name };
        stage(k, bias, name, phdr, phnum);
        if let Some(live) = TABLE.get(k).and_then(|e| e.get(4)) {
            live.store(live_word(phdr, phnum), Release);
        }
    }
    let _ = PUBLISHED.compare_exchange(0, total, Release, Relaxed);
    total
}

/// Write entry `k` of an object not visible yet (`super::dlopen::candidate`): a slot no reader can
/// be reading (never used, or freed after a grace period, DL-17).
pub(super) fn stage(k: usize, bias: usize, name: usize, phdr: usize, phnum: usize) {
    if let Some(e) = slot(k) {
        for (w, v) in e.iter().zip([bias, name, phdr, phnum]) {
            w.store(v, Relaxed);
        }
    }
}

/// Copy staged entry `from` to `to` (a set moved below the top before its publication,
/// `super::dlopen::settle`): both invisible, `to` a slot freed after a grace period.
pub(super) fn restage(from: usize, to: usize) {
    if let (Some(a), Some(b)) = (slot(from), slot(to)) {
        for (w, v) in b.iter().zip(a.iter()).take(4) {
            w.store(v.load(Relaxed), Relaxed);
        }
    }
}

/// Index order is no longer load order (a set went below the top, `super::dlopen::settle`):
/// [`iterate`] walks the chain from now on. Never cleared.
static REORDERED: AtomicBool = AtomicBool::new(false);

/// Mark [`REORDERED`] (before the set's publication).
pub(super) fn reorder() {
    REORDERED.store(true, Release);
}

/// The publication point of one `dlopen` (`super::dlopen::load_set`, after pass 2, before the
/// constructors): the entries `base..len` (staged) become visible, `len` is the loader's count.
pub(super) fn publish(base: usize, len: usize) {
    fill_start();
    ADDS.fetch_add(u64::try_from(len.saturating_sub(base)).unwrap_or(0), Relaxed);
    for e in (base..len).filter_map(slot) {
        let [_, _, phdr, phnum, live] = e;
        live.store(live_word(phdr.load(Relaxed), phnum.load(Relaxed)), Release);
    }
    PUBLISHED.fetch_max(len, Release);
    super::index::added(base, len);
}

/// `dlclose` unloads object `k` (DL-17): its entry is invisible to readers entering from now on,
/// and `dlpi_subs` counts it (probe `== close` C-1: as `dlclose` returns).
pub(super) fn unpublish(k: usize) {
    if let Some(live) = slot(k).and_then(|e| e.get(4)) {
        live.store(0, Release);
    }
    super::index::removed(k);
    SUBS.fetch_add(1, Release);
}

/// A failed attempt mapped `n` objects and unmapped them again (dynlink.md: both counters move).
pub(super) fn count_unmapped(n: usize) {
    let n = u64::try_from(n).unwrap_or(0);
    ADDS.fetch_add(n, Relaxed);
    SUBS.fetch_add(n, Release);
}

/// The TLS module ID and the calling thread's block of the object, when it has a `PT_TLS` segment:
/// DL-15's records of the start-time modules, which DL-16d's `dlopen` extends with each new module
/// before [`publish`] lists its object; `(0, 0)` otherwise.
fn tls_of(e: &Entry) -> (u64, usize) {
    if e.live != LIVE_TLS {
        return (0, 0);
    }
    tls_module(e)
}

/// [`tls_of`] for an object with a `PT_TLS` header (out of the walk's loop).
#[inline(never)]
fn tls_module(e: &Entry) -> (u64, usize) {
    let Some((_, vaddr, _, _)) = headers(e.phdr, e.phnum).find(|h| h.0 == PT_TLS) else { return (0, 0) };
    let image = u64::try_from(e.bias).unwrap_or(0).wrapping_add(vaddr);
    let n = THREAD_MODS_LEN.load(Acquire);
    for m in (0..n).filter_map(super::thread_mod) {
        let [src, _, _, module, off]: [u64; 5] = core::array::from_fn(|i| m.get(i).map_or(0, |w| w.load(Relaxed)));
        if src == image {
            if niji_ld::tls::dynamic_align(off).is_some() {
                // An on-demand block (DL-19): the calling thread's, or null when it has none yet
                // (dl_iterate_phdr(3): "if it has been allocated in the calling thread").
                let p = niji_ld::tls::Placed { module, tp_offset: niji_ld::tls::NO_STATIC };
                return (module, super::tls::thread_address(p, 0, false));
            }
            let tp = u64::try_from(tb::tp()).unwrap_or(0);
            return (module, usize::try_from(tp.wrapping_add_signed(i64::from_ne_bytes(off.to_ne_bytes()))).unwrap_or(0));
        }
    }
    (0, 0)
}

/// The address of [`iterate`], for `StartInfo::dl_iterate`.
pub(super) fn iterate_address() -> usize {
    #[allow(clippy::as_conversions)] // a function's address has no conversion other than `as`
    (iterate as *const ()).expose_provenance()
}

/// The address of [`find_object`], for `StartInfo::dl_find_object`.
pub(super) fn find_object_address() -> usize {
    #[allow(clippy::as_conversions)] // a function's address has no conversion other than `as`
    (find_object as *const ()).expose_provenance()
}

/// Call `cb(info, size, data)`: its `int` result.
fn call(cb: usize, info: &struct_dl_phdr_info, data: usize) -> i32 {
    let w = |v: usize| i64::from_ne_bytes(u64::try_from(v).unwrap_or(0).to_ne_bytes());
    let at = core::ptr::from_ref(info).expose_provenance();
    int_result(crate::ffi::niji_machine_call3(cb, w(at), w(size_of::<struct_dl_phdr_info>()), w(data)))
}

/// `dl_iterate_phdr(cb, data)` in a program `ld.so` started: every published object in chain
/// order, the vDSO second (probe `== phdr`, P-12), until a callback returns non-zero (returned).
extern "C" fn iterate(cb: usize, data: usize) -> i64 {
    read_section(|v| {
        if REORDERED.load(Acquire) {
            return iterate_chain(v, cb, data);
        }
        let start = v.start_adds();
        let mut i = 0usize;
        while i < v.len0() {
            if let Some(e) = v.entry(i) {
                if let Some(r) = report(start, i, &e, cb, data) {
                    return r;
                }
            }
            i = i.wrapping_add(1);
        }
        if i < v.len() {
            return iterate_past(v, start, cb, data);
        }
        0
    })
}

/// [`iterate`] past generation 0 (entries [`CAP`] on).
#[cold]
#[inline(never)]
fn iterate_past(v: &View, start: u64, cb: usize, data: usize) -> i64 {
    let mut i = CAP;
    while i < v.len() {
        if let Some(e) = v.entry_past(i) {
            if let Some(r) = report(start, i, &e, cb, data) {
                return r;
            }
        }
        i = i.wrapping_add(1);
    }
    0
}

/// [`iterate`] once index order is not load order ([`REORDERED`]): the objects in chain order
/// (the walk `dladdr` makes, `super::dlopen::chain_find`), each reported when its entry is live.
/// A node a `dlclose` unlinks keeps its `l_next` until its slot is reused after this reader has
/// left, so the walk goes on to the live chain; an object a callback opens is linked at the end.
#[cold]
#[inline(never)]
fn iterate_chain(v: &View, cb: usize, data: usize) -> i64 {
    let start = v.start_adds();
    let mut out = 0i64;
    let _ = super::dlopen::chain_find(|i| {
        let e = if i < CAP { v.entry(i) } else { v.entry_past(i) };
        match e.and_then(|e| report(start, i, &e, cb, data)) {
            Some(r) => {
                out = r;
                true
            }
            None => false,
        }
    });
    out
}

/// Call `cb` for live entry `i` (and the vDSO after entry 0): `Some` with the callback's nonzero
/// result, which ends the walk. `start`: [`View::start_adds`]; the `dlopen`s' counters are read at
/// every step (the walk is live).
#[inline(always)]
fn report(start: u64, i: usize, e: &Entry, cb: usize, data: usize) -> Option<i64> {
    let (adds, subs) = View::counters(start);
    let (modid, block) = tls_of(e);
    let info = struct_dl_phdr_info { dlpi_addr: u64::try_from(e.bias).unwrap_or(0), dlpi_name: e.name, dlpi_phdr: e.phdr, dlpi_phnum: u16::try_from(e.phnum).unwrap_or(0), dlpi_adds: adds, dlpi_subs: subs, dlpi_tls_modid: modid, dlpi_tls_data: block };
    let r = call(cb, &info, data);
    if r != 0 {
        return Some(i64::from(r));
    }
    if i == 0 {
        return report_vdso(start, cb, data);
    }
    None
}

/// [`report`]'s step after entry 0: the vDSO, when the process has one.
#[inline(never)]
fn report_vdso(start: u64, cb: usize, data: usize) -> Option<i64> {
    let vd = vdso();
    if vd == 0 {
        return None;
    }
    let (vphdr, vphnum, bias) = vdso_headers(vd);
    let (adds, subs) = View::counters(start);
    let info = struct_dl_phdr_info { dlpi_addr: u64::try_from(bias).unwrap_or(0), dlpi_name: c"linux-vdso.so.1".as_ptr().expose_provenance(), dlpi_phdr: vphdr, dlpi_phnum: u16::try_from(vphnum).unwrap_or(0), dlpi_adds: adds, dlpi_subs: subs, dlpi_tls_modid: 0, dlpi_tls_data: 0 };
    let r = call(cb, &info, data);
    if r != 0 {
        return Some(i64::from(r));
    }
    None
}

/// [`find_object`]'s walk over the table, when the index cannot answer (`super::index`).
#[cold]
#[inline(never)]
fn find_walk(v: &View, pc: usize) -> Option<(usize, usize, usize, usize)> {
    let n = v.len();
    (0..n.min(CAP))
        .find_map(|i| {
            let e = v.entry(i)?;
            find_in(e.phdr, e.phnum, e.bias, pc, extent(i)).map(|(lo, hi, eh)| (lo, hi, eh, node_address(i)))
        })
        .or_else(|| find_past(v, n, pc))
}

/// [`find_object`]'s walk past generation 0 (entries [`CAP`] to `n`).
#[cold]
#[inline(never)]
fn find_past(v: &View, n: usize, pc: usize) -> Option<(usize, usize, usize, usize)> {
    (CAP..n).find_map(|i| {
        let e = v.entry_past(i)?;
        find_in(e.phdr, e.phnum, e.bias, pc, extent(i)).map(|(lo, hi, eh)| (lo, hi, eh, node_address(i)))
    })
}

/// `_dl_find_object(pc, result)` in a program `ld.so` started: the published object holding `pc`
/// (a `PT_LOAD`), its whole `PT_LOAD` span, `PT_GNU_EH_FRAME` and link map (probe R-1; the link
/// map is 0 before the first `dlopen` makes the loader's nodes, and for the vDSO); 0, or -1 with
/// `*result` untouched.
extern "C" fn find_object(pc: usize, result: usize) -> i64 {
    let found = read_section(|v| match super::index::find(pc) {
        Found::Hit(h) => Some((h.lo, h.hi, h.eh, node_address(h.i))),
        Found::Miss => None,
        Found::Unknown => find_walk(v, pc),
    });
    let found = found.or_else(|| {
        let vd = vdso();
        if vd == 0 {
            return None;
        }
        let (vphdr, vphnum, bias) = vdso_headers(vd);
        find_in(vphdr, vphnum, bias, pc, Extent::Whole).map(|(lo, hi, eh)| (lo, hi, eh, 0))
    });
    let Some((lo, hi, eh, map)) = found else { return -1 };
    // UNSAFE(slice): the caller's `struct dl_find_object` (§27.1).
    #[allow(unsafe_code)]
    // SAFETY: `result` is the caller's pointer to a writable `struct dl_find_object` (the contract of
    // `_dl_find_object`); NULL is refused.
    let Some(r) = (unsafe { core::ptr::with_exposed_provenance_mut::<struct_dl_find_object>(result).as_mut() }) else { return -1 };
    *r = struct_dl_find_object { dlfo_flags: 0, dlfo_map_start: lo, dlfo_map_end: hi, dlfo_link_map: map, dlfo_eh_frame: eh, __dflo_reserved: [0; 7] };
    0
}
