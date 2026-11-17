//! Thread-local storage (design §7): the provisional thread block, the static TLS block and its
//! surplus for `dlopen`ed modules, the module table every new thread copies, the dtv and its
//! generation, and `__tls_get_addr`.

use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize};
use core::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use niji_ld::arch::{Arch, Native};
use niji_ld::init;
use niji_ld::tls::{self as ltls, Area, Module, Placed};
use niji_rt::tls::{ThreadBlock, FLAG_OWN};
use crate::{conv, raw};
use super::io::{lo32, refuse};
use super::state::PHDR_WORDS;

/// Words in the provisional thread block: libniji's whole thread block, so every `%fs`-relative
/// field (errno, canary, pointer guard, ...) lands inside it.
const TB_WORDS: usize = core::mem::size_of::<ThreadBlock>().div_ceil(8);

/// The provisional thread block (L14-2): zero in `.bss`, its first word the thread pointer itself
/// once installed. The final block replaces it after the TLS layout (DL-6 onwards).
#[repr(C, align(64))]
pub(super) struct Provisional(pub(super) [AtomicUsize; TB_WORDS]);

pub(super) static PROVISIONAL: Provisional = Provisional([const { AtomicUsize::new(0) }; TB_WORDS]);

/// The TLS generation (design §7): the dtv of every thread was built for it; loading or unloading a
/// module with TLS after start (DL-15/DL-16) moves it on. 0 for the initially loaded objects.
static GENERATION: AtomicU64 = AtomicU64::new(0);

/// Whether this loader started the process (D-X.58: ownership is a process-level fact, never a word
/// of a thread block): set by [`install_tls`] once the initial thread runs on the block it built.
/// Its threads then all have blocks of this runtime; in a process another loader started (this
/// object loaded as a library) it stays false and no thread block is read as this loader's.
static STARTED_HERE: AtomicBool = AtomicBool::new(false);

/// The bytes above the initial thread's pointer (D-27.26): the thread block, then room up to the
/// end of the rseq area at `RSEQ_OFFSET`, where the C library's descriptor puts it in every other
/// thread, so `__rseq_offset` is one value for all threads.
#[allow(clippy::as_conversions)] // const context: `try_from` is not const
pub(super) const INITIAL_TAIL: u64 = {
    let end = (niji_rt::tls::RSEQ_OFFSET + niji_rt::thread::RSEQ_SIZE as usize).next_multiple_of(64) as u64;
    if end > Native::TCB_SIZE { end } else { Native::TCB_SIZE }
};
const _: () = assert!(niji_rt::tls::RSEQ_OFFSET % 32 == 0, "the rseq area is 32-byte aligned");

// The thread-block slots the arch table names are the fields of libniji's block (D-4.15).
#[allow(clippy::as_conversions)] // const context: no `try_from`; the values are small offsets
const _: () = {
    assert!(ltls::DTV_HEADER == 2);
    assert!(Native::TCB_SIZE as usize == core::mem::size_of::<ThreadBlock>());
    assert!(Native::DTV_SLOT as usize == core::mem::offset_of!(ThreadBlock, dtv));
    assert!(matches!(Native::SELF_SLOT, Some(0)));
    assert!(matches!(Native::STACK_GUARD_SLOT, Some(s) if s as usize == core::mem::offset_of!(ThreadBlock, stack_guard)));
    assert!(matches!(Native::POINTER_GUARD_SLOT, Some(s) if s as usize == core::mem::offset_of!(ThreadBlock, pointer_guard)));
};

/// The initial thread's area for `area`, mapped read-write and zero-filled: its `area.align`-aligned
/// base (mapped before relocation, so the rseq area in it is registered and published first).
pub(super) fn map_area(area: &Area) -> usize {
    let fail = || -> ! { refuse(b"ld.so: cannot set up the initial thread's TLS\n") };
    let (Ok(len), Ok(align)) = (usize::try_from(area.len), usize::try_from(area.align)) else { fail() };
    let prot = i32::try_from(niji_kernel::uapi::PROT_READ | niji_kernel::uapi::PROT_WRITE).unwrap_or(0);
    let flags = i32::try_from(niji_kernel::uapi::MAP_PRIVATE | niji_kernel::uapi::MAP_ANONYMOUS).unwrap_or(0);
    let Some(map_len) = len.checked_add(align) else { fail() };
    let Ok(raw_base) = raw::mmap(0, map_len, prot, flags, -1, 0) else { fail() };
    raw_base.checked_next_multiple_of(align).unwrap_or_else(|| fail())
}

/// The program's `PT_TLS` from its header words, refused unless its image lies in the file part of a
/// `PT_LOAD` (word 0 of a header holds `p_type`; words 2, 4, 5, 6 `p_vaddr`, `p_filesz`, `p_memsz`,
/// `p_align`).
pub(super) fn program_tls(headers: &[u64], phnum: usize) -> Option<Module> {
    let hs = || headers.chunks_exact(PHDR_WORDS).take(phnum);
    let w = |h: &[u64], i: usize| h.get(i).copied().unwrap_or(0);
    let t = hs().find(|h| lo32(w(h, 0)) == niji_ld::elf::pt::TLS)?;
    let m = Module { vaddr: w(t, 2), filesz: w(t, 4), memsz: w(t, 5), align: w(t, 6) };
    let end = m.vaddr.checked_add(m.filesz);
    let inside = hs().any(|h| lo32(w(h, 0)) == niji_ld::elf::pt::LOAD && w(h, 2) <= m.vaddr && end.is_some_and(|e| e <= w(h, 2).saturating_add(w(h, 4))));
    if !inside {
        refuse(b"ld.so: the program's TLS image lies outside its segments\n");
    }
    Some(m)
}

/// Build the initial thread's area (design §7, D-27.20): map `area`, copy each placed module's
/// initialisation image from its loaded (and relocated) object (`biases[i]` + `p_vaddr`, `p_filesz`
/// bytes; the rest of the block stays zero), fill the thread block (self, dtv, the canary and pointer
/// guard from the provisional block, `FLAG_OWN`) and the dtv, and make it the thread pointer.
/// In the area [`map_area`] mapped at `base`.
pub(super) fn install_tls(mods: &[Option<Module>], biases: &[u64], placed: &[Option<Placed>], area: &Area, base: usize) {
    let fail = || -> ! { refuse(b"ld.so: cannot set up the initial thread's TLS\n") };
    let tp = base.wrapping_add(usize::try_from(area.tp).unwrap_or(0));
    let tp64 = u64::try_from(tp).unwrap_or(0);
    record_thread_tls(mods, biases, placed, area);
    let dtv_at = base.wrapping_add(usize::try_from(area.dtv).unwrap_or(0));
    // A fresh zero mapping: each block's bytes past its image are zero already (not written, so
    // a large `.tbss` costs no page until the program touches it).
    if fill_static(tp, dtv_at, false).is_none() {
        fail()
    }
    // UNSAFE(slice): the thread block at the new thread pointer.
    #[allow(unsafe_code)]
    // SAFETY: `ltls::layout` keeps `TCB_SIZE` bytes from `area.tp` for the block, inside the area
    // mapped above (zero-filled, 64-aligned as `area.align` >= `TCB_ALIGN`).
    let tb = unsafe { conv::bytes_at_mut(tp, core::mem::size_of::<ThreadBlock>()) };
    let mut put = |off: Option<u64>, v: u64| {
        if let Some(w) = off.and_then(|o| usize::try_from(o).ok()).and_then(|o| tb.get_mut(o..o.wrapping_add(8))) {
            w.copy_from_slice(&v.to_ne_bytes());
        }
    };
    let provisional = |slot: Option<u64>| slot.and_then(|s| PROVISIONAL.0.get(usize::try_from(s / 8).unwrap_or(usize::MAX))).map_or(0, |w| w.load(Relaxed));
    put(Native::SELF_SLOT, tp64);
    put(u64::try_from(core::mem::offset_of!(ThreadBlock, self_ptr)).ok(), tp64);
    put(Some(Native::DTV_SLOT), u64::try_from(dtv_at).unwrap_or(0));
    put(Native::STACK_GUARD_SLOT, u64::try_from(provisional(Native::STACK_GUARD_SLOT)).unwrap_or(0));
    put(Native::POINTER_GUARD_SLOT, u64::try_from(provisional(Native::POINTER_GUARD_SLOT)).unwrap_or(0));
    let flags_at = core::mem::offset_of!(ThreadBlock, flags);
    if let Some(w) = tb.get_mut(flags_at..flags_at.wrapping_add(4)) {
        w.copy_from_slice(&FLAG_OWN.to_ne_bytes());
    }
    if crate::arch::set_thread_pointer(tp).is_err() {
        fail()
    }
    STARTED_HERE.store(true, Release);
}

/// Every module's static block, which each thread's static area holds (DL-15 `DL-TLS-THREADS`,
/// DL-16d `DL-DLOPEN-TLS-SURPLUS`; design §7, D-27.20): per module `[image address, p_filesz,
/// p_memsz, module ID, block offset from TP]`, in module-ID order (IDs 1 onwards, entry `i` holds
/// ID `i + 1`). The initially loaded modules are written by [`record_thread_tls`] while the process
/// has one thread; a `dlopen`ed module is appended by [`tls_visit`] at the end of the C library's
/// thread walk, under its thread-list lock, which a thread's creation also holds while its
/// `tls_init` reads this table: the creation either reads the table before the walk (and the walk,
/// which then lists it, fills the new block) or after it (and fills the block itself).
pub(super) static THREAD_MODS: [[AtomicU64; 5]; ltls::MODULES_MAX] = [const { [const { AtomicU64::new(0) }; 5] }; ltls::MODULES_MAX];
/// Entries of [`THREAD_MODS`] in use (stored last, Release).
pub(super) static THREAD_MODS_LEN: AtomicUsize = AtomicUsize::new(0);
/// Words of every thread's dtv: `DTV_HEADER` + the module slots it has room for.
static THREAD_DTV_WORDS: AtomicU64 = AtomicU64::new(0);
/// Bytes a new thread's static area needs below its thread pointer: the blocks and the surplus as
/// in the initial area, then the dtv under them (`StartInfo::tls_size`).
pub(super) static THREAD_TLS_SIZE: AtomicU64 = AtomicU64::new(0);
/// The thread pointer's alignment (`StartInfo::tls_align`).
pub(super) static THREAD_TLS_ALIGN: AtomicU64 = AtomicU64::new(0);
/// The static surplus, `[lo, hi)` in bytes below every thread pointer (DL-16d, R14-4).
static SURPLUS_LO: AtomicU64 = AtomicU64::new(0);
static SURPLUS_HI: AtomicU64 = AtomicU64::new(0);

/// Record what [`fill_static`] needs to repeat the initial thread's static area in another thread:
/// each placed module's image (`biases[i]` + `p_vaddr`, `p_filesz`), `p_memsz`, module ID and offset.
fn record_thread_tls(mods: &[Option<Module>], biases: &[u64], placed: &[Option<Placed>], area: &Area) {
    let mut n = 0usize;
    for ((m, p), b) in mods.iter().zip(placed).zip(biases) {
        let (Some(m), Some(p), Some(slot)) = (m, p, THREAD_MODS.get(n)) else { continue };
        for (w, v) in slot.iter().zip([b.wrapping_add(m.vaddr), m.filesz, m.memsz, p.module, u64::from_ne_bytes(p.tp_offset.to_ne_bytes())]) {
            w.store(v, Relaxed);
        }
        n = n.wrapping_add(1);
    }
    // ARCH(x86_64): variant II, the blocks and the surplus in the `area.tp` bytes below TP; variant I
    // (A-4) places them above the TCB and needs its own thread layout.
    let size = area.dtv_words.checked_mul(8).and_then(|d| area.tp.checked_add(d)).unwrap_or(0);
    THREAD_DTV_WORDS.store(area.dtv_words, Relaxed);
    THREAD_TLS_SIZE.store(size, Relaxed);
    THREAD_TLS_ALIGN.store(area.align, Relaxed);
    SURPLUS_LO.store(area.used, Relaxed);
    SURPLUS_HI.store(area.tp, Relaxed);
    THREAD_MODS_LEN.store(n, Release);
}

/// Entry `i` of [`THREAD_MODS`]: `[image address, p_filesz, p_memsz, module ID, offset]`.
fn module_words(i: usize) -> Option<[u64; 5]> {
    let slot = THREAD_MODS.get(i)?;
    Some(core::array::from_fn(|j| slot.get(j).map_or(0, |w| w.load(Relaxed))))
}

/// The dtv word of module `module`.
fn dtv_slot(module: u64) -> Option<usize> {
    usize::try_from(ltls::DTV_HEADER.checked_add(module)?.checked_sub(1)?).ok()
}

/// The `n` words of a dtv at `at`, as atomics: a running thread reads its own vector while a
/// `dlopen` in another thread fills a new module's slot ([`tls_visit`]).
fn dtv_words(at: usize, n: usize) -> &'static [AtomicU64] {
    if at == 0 || at % 8 != 0 {
        return &[];
    }
    // UNSAFE(slice): a thread's dtv as atomic words.
    #[allow(unsafe_code)]
    // SAFETY: every dtv this loader makes is 8-aligned with `THREAD_DTV_WORDS` words reserved in its
    // thread's area for the thread's lifetime (`area.dtv` in the initial area, the lowest words of a
    // new thread's `tls_size` bytes), and every access to it goes through these atomics.
    unsafe {
        core::slice::from_raw_parts(core::ptr::with_exposed_provenance::<AtomicU64>(at), n)
    }
}

/// Write one module's block at `dst`: its image (`filesz` bytes from `src`), then (`zero`) zeros
/// up to `memsz`.
fn init_block(dst: u64, [src, filesz, memsz]: [u64; 3], zero: bool) -> Option<()> {
    let (dst, src, filesz, memsz) = (usize::try_from(dst).ok()?, usize::try_from(src).ok()?, usize::try_from(filesz).ok()?, usize::try_from(memsz).ok()?);
    let len = if zero { memsz } else { filesz };
    if len == 0 {
        return Some(());
    }
    // UNSAFE(slice): the module's block in a thread's static area.
    #[allow(unsafe_code)]
    // SAFETY: `p_memsz` bytes at the block's offset below TP (`ltls::layout` / `Surplus::place`) in a
    // thread's area: one not started yet, or one listed (mapped while listed) whose code cannot reach
    // a `dlopen`ed block before its module is published (see [`tls_visit`]).
    let block = unsafe { conv::bytes_at_mut(dst, len) };
    let (init, rest) = block.split_at_mut(filesz.min(len));
    if !init.is_empty() {
        // UNSAFE(slice): the module's TLS initialisation image in its loaded object.
        #[allow(unsafe_code)]
        // SAFETY: `ltls::image_in_load` / `program_tls` checked that [p_vaddr, p_vaddr + p_filesz)
        // lies in the file part of one of the object's PT_LOADs, mapped readable at bias + p_vaddr
        // for the process's lifetime (no object is unloaded before DL-17).
        init.copy_from_slice(unsafe { conv::bytes_at(src, init.len()) });
    }
    rest.fill(0);
    Some(())
}

/// Fill a thread's static area from [`THREAD_MODS`]: each block's image at `tp` + its offset, then
/// (`zero`) zeros up to its `p_memsz`; the dtv at `dtv_at` (generation, slot count, one block
/// address per module; the slots past the count are not written). `None` when a recorded value
/// does not fit (never for a recorded layout).
fn fill_static(tp: usize, dtv_at: usize, zero: bool) -> Option<()> {
    let tp64 = u64::try_from(tp).ok()?;
    let n = THREAD_MODS_LEN.load(Acquire);
    let dtv = dtv_words(dtv_at, usize::try_from(THREAD_DTV_WORDS.load(Relaxed)).ok()?);
    let mut count = 0u64;
    for i in 0..n {
        let [src, filesz, memsz, module, off] = module_words(i)?;
        if module == 0 {
            // A released ID (DL-17): no block until it is reused.
            continue;
        }
        let block = tp64.wrapping_add_signed(i64::from_ne_bytes(off.to_ne_bytes()));
        init_block(block, [src, filesz, memsz], zero)?;
        dtv.get(dtv_slot(module)?)?.store(block, Relaxed);
        count = count.max(module);
    }
    let [g, c, ..] = dtv else { return None };
    c.store(count, Relaxed);
    g.store(GENERATION.load(Relaxed), Relaxed);
    Some(())
}

/// The modules one `dlopen` adds (DL-16d): each `[image address, p_filesz, p_memsz, module ID,
/// offset]`, the IDs following [`THREAD_MODS`]'s, and the generation they make. Handed to the C
/// library's thread walk as [`tls_visit`]'s context.
pub(super) struct NewTls {
    pub(super) mods: [[u64; 5]; init::OBJECTS_MAX],
    pub(super) len: usize,
    generation: u64,
    /// The modules are unloaded ([`release_tls`], DL-17), not added.
    release: bool,
}

impl NewTls {
    pub(super) const NONE: NewTls = NewTls { mods: [[0; 5]; init::OBJECTS_MAX], len: 0, generation: 0, release: false };
}

/// The static surplus `[lo, hi)` below every thread pointer and the thread pointer's alignment
/// (DL-16d); all 0 before [`record_thread_tls`].
pub(super) fn surplus() -> (u64, u64, u64) {
    (SURPLUS_LO.load(Relaxed), SURPLUS_HI.load(Relaxed), THREAD_TLS_ALIGN.load(Relaxed))
}

/// The module IDs a `dlopen` may hand out (`ltls::ModuleIds::take`): the IDs [`THREAD_MODS`]
/// covers already (the floor), and the highest every thread's dtv has a slot for.
pub(super) fn module_room() -> (u64, u64) {
    let floor = u64::try_from(THREAD_MODS_LEN.load(Acquire)).unwrap_or(u64::MAX);
    let slots = THREAD_DTV_WORDS.load(Relaxed).saturating_sub(ltls::DTV_HEADER);
    (floor, slots.min(u64::try_from(ltls::MODULES_MAX).unwrap_or(0)))
}

/// The TLS generation now (DL-17: `ltls::ModuleIds::release` makes the next).
pub(super) fn tls_generation() -> u64 {
    GENERATION.load(Relaxed)
}

/// Take `gone`'s modules (`mods[..len]`, ID in word 3) out of [`THREAD_MODS`] at the end of the C
/// library's thread walk (its thread-list lock held, as [`publish_tls`]) and make `generation`
/// current: a thread whose dtv still names an old block repairs it in [`tls_slow`] (DL-17). The
/// threads' blocks are left as they are (their bytes belong to the surplus place, freed already).
pub(super) fn release_tls(walk: usize, gone: &mut NewTls, generation: u64) {
    gone.generation = generation;
    gone.release = true;
    #[allow(clippy::as_conversions)] // a function's address has no conversion other than `as`
    let visit = (tls_visit as *const ()).expose_provenance();
    let ctx = core::ptr::from_ref::<NewTls>(gone).expose_provenance();
    let arg = |v: usize| i64::from_ne_bytes(v.to_ne_bytes());
    if walk != 0 {
        let _ = crate::ffi::niji_machine_call3(walk, arg(visit), arg(ctx), 0);
    }
}

/// The place of the module whose image starts at `image` (its object's bias + `p_vaddr`): an
/// initially loaded one, or (for `dlinfo`) a `dlopen`ed one [`tls_visit`] has appended.
pub(super) fn initial_place(image: u64) -> Option<Placed> {
    (0..THREAD_MODS_LEN.load(Acquire)).filter_map(module_words).find(|w| w[0] == image).map(|w| Placed { module: w[3], tp_offset: i64::from_ne_bytes(w[4].to_ne_bytes()) })
}

/// Give `new`'s modules to every thread (DL-16d, plan/ph14/dl-16-dlopen.md §5 points 1 and 3):
/// the C library's `walk` calls [`tls_visit`] for each listed thread, then once with 0, all under
/// its thread-list lock. Called by `dlopen` (loader lock held) after relocation, before publication.
pub(super) fn publish_tls(walk: usize, new: &mut NewTls) {
    new.generation = GENERATION.load(Relaxed).wrapping_add(1);
    #[allow(clippy::as_conversions)] // a function's address has no conversion other than `as`
    let visit = (tls_visit as *const ()).expose_provenance();
    let ctx = core::ptr::from_ref::<NewTls>(new).expose_provenance();
    let arg = |v: usize| i64::from_ne_bytes(v.to_ne_bytes());
    let _ = crate::ffi::niji_machine_call3(walk, arg(visit), arg(ctx), 0);
}

/// One step of the C library's thread walk ([`publish_tls`]), its thread-list lock held. For a
/// thread `tp`: each new module's block gets its image and zeros, then the thread's dtv its slots,
/// the slot count and the new generation (eagerly; a thread whose vector is still behind repairs it
/// in [`tls_slow`]). For `tp` 0, the end of the walk: the modules join [`THREAD_MODS`] (so a thread
/// created from now on fills them itself) and the generation moves on.
extern "C" fn tls_visit(ctx: usize, tp: usize) -> usize {
    // UNSAFE(slice): the walk's context.
    #[allow(unsafe_code)]
    // SAFETY: [`publish_tls`] passes the address of a live `NewTls` on its stack and the walk calls
    // back before `publish_tls` returns.
    let Some(new) = (unsafe { core::ptr::with_exposed_provenance::<NewTls>(ctx).as_ref() }) else { return 0 };
    let mods = new.mods.get(..new.len).unwrap_or(&[]);
    if tp == 0 {
        // Entry `id - 1` holds module `id` (DL-17: a released ID's entry is all zero until reused).
        let mut n = THREAD_MODS_LEN.load(Relaxed);
        for m in mods {
            let Some(slot) = usize::try_from(m[3]).ok().and_then(|id| id.checked_sub(1)).and_then(|i| THREAD_MODS.get(i)) else { continue };
            let words = if new.release { [0; 5] } else { *m };
            slot.iter().zip(words).for_each(|(w, v)| w.store(v, Relaxed));
            if !new.release {
                n = n.max(usize::try_from(m[3]).unwrap_or(0));
            }
        }
        THREAD_MODS_LEN.store(n.min(ltls::MODULES_MAX), Release);
        GENERATION.store(new.generation, Release);
        return 0;
    }
    if new.release {
        return 0;
    }
    let b = crate::tb::listed(tp);
    // Only a block this loader made has a dtv of this layout (D-5.11 safety, design §7).
    if b.flags.load(Relaxed) & FLAG_OWN == 0 {
        return 0;
    }
    let dtv = dtv_words(b.dtv.load(Relaxed), usize::try_from(THREAD_DTV_WORDS.load(Relaxed)).unwrap_or(0));
    let mut count = None;
    for &[src, filesz, memsz, module, off] in mods {
        let block = u64::try_from(tp).unwrap_or(0).wrapping_add_signed(i64::from_ne_bytes(off.to_ne_bytes()));
        if init_block(block, [src, filesz, memsz], true).is_none() {
            continue;
        }
        if let Some(w) = dtv_slot(module).and_then(|i| dtv.get(i)) {
            w.store(block, Relaxed);
            count = Some(module);
        }
    }
    if let (Some(c), [g, n, ..]) = (count, dtv) {
        n.store(c.max(n.load(Relaxed)), Release);
        g.store(new.generation, Release);
    }
    0
}

/// `StartInfo::tls_init` (DL-15 `DL-TLS-THREADS`, design §7): the static area of a thread the C
/// library is about to start, below its thread pointer `tp`. Every block gets its image and zeros to
/// `p_memsz` (the C library may reuse an exited thread's mapping), the dtv goes in the lowest words
/// of the area (`tls_size` bytes below `tp`), and the thread block's dtv word points at it. Called
/// by the creating thread, concurrently with other creations: it reads only the recorded table.
pub(super) extern "C" fn thread_tls_init(tp: usize) -> usize {
    let size = usize::try_from(THREAD_TLS_SIZE.load(Acquire)).unwrap_or(0);
    let dtv_at = tp.wrapping_sub(size);
    if size == 0 || fill_static(tp, dtv_at, true).is_none() {
        refuse(b"ld.so: cannot set up a new thread's TLS\n");
    }
    let slot = tp.wrapping_add(usize::try_from(Native::DTV_SLOT).unwrap_or(0));
    // UNSAFE(slice): the dtv word of the new thread's block.
    #[allow(unsafe_code)]
    // SAFETY: the C library passes the thread pointer of a block it mapped read-write (`TCB_SIZE`
    // bytes from `tp`, the dtv word inside them) for a thread that does not run yet.
    unsafe { conv::bytes_at_mut(slot, 8) }.copy_from_slice(&dtv_at.to_ne_bytes());
    0
}

/// `StartInfo::tls_release` (DL-15, design §7): the thread at `tp` is ending. Its static blocks and
/// dtv lie in the C library's mapping and go with it, so nothing is left to release; the hook is the
/// place for the loader's per-thread state that does not (dynamic blocks, DL-19).
pub(super) extern "C" fn thread_tls_release(_tp: usize) -> usize {
    0
}

/// `__tls_get_addr` (spec §22.6, s22_6_c_4; exported by `ld.so` at `GLIBC_2.3`): the address of
/// `ti->offset` in module `ti->module`'s block for the calling thread. `ti` points at a `tls_index`
/// (two words: module ID, offset; x86-64 psABI §3.4.6). DL-12: static modules only; any other answer
/// is a failure (the dlopen family, DL-15/DL-16, adds the slow path).
// UNSAFE(export): the exported `__tls_get_addr` of the ld.so product.
#[unsafe(no_mangle)]
#[allow(unsafe_code)]
pub extern "C" fn __tls_get_addr(ti: usize) -> usize {
    // Only a block this loader's runtime made has its dtv word (D-5.11 safety, design §7; D-X.58):
    // in a process another loader started, never read the word of a block it made as a pointer.
    // The test is the process-level byte, never the block's own flags word (another library's data).
    if !STARTED_HERE.load(Acquire) {
        refuse(b"ld.so: __tls_get_addr called in a process this loader did not start\n");
    }
    let dtv = crate::tb::read::<crate::tb::Dtv>();
    if dtv == 0 {
        // A thread the runtime made without a vector (DL-12: threads get theirs with DL-15/DL-16).
        refuse(b"ld.so: __tls_get_addr: this thread has no TLS vector\n");
    }
    // UNSAFE(slice): the caller's `tls_index`.
    #[allow(unsafe_code)]
    // SAFETY: the psABI caller passes the address of a two-word `tls_index` in its GOT (⚑ ABI).
    let index = unsafe { conv::words_at(ti, 2) };
    let (module, offset) = (index.first().copied().unwrap_or(0), index.get(1).copied().unwrap_or(0));
    // This loader started the process (`STARTED_HERE`, checked above), so the thread block is its
    // runtime's and its non-zero dtv word (checked above) points at a vector this loader built with
    // `THREAD_DTV_WORDS` words; `ltls::lookup` reads a slot only after checking it against the count
    // in word 1.
    let words = dtv_words(dtv, usize::try_from(THREAD_DTV_WORDS.load(Relaxed)).unwrap_or(0));
    let word = |i: u64| -> u64 { usize::try_from(i).ok().and_then(|i| words.get(i)).map_or(0, |w| w.load(Relaxed)) };
    match ltls::lookup::<Native>(word, GENERATION.load(Relaxed), module, offset).and_then(|a| usize::try_from(a).ok()) {
        Some(a) => a,
        None => tls_slow(words, module, offset),
    }
}

/// `__tls_get_addr`'s slow path (DL-16d, plan §5 point 3): the calling thread's vector is behind
/// the generation (a module `dlopen`ed since it was filled) or lacks the slot. Every slot is filled
/// again from [`THREAD_MODS`] (static blocks: thread pointer + offset), then the count and the
/// generation read first; the address follows. A module the table does not hold aborts (s27_3_f_0).
#[cold]
#[inline(never)]
fn tls_slow(dtv: &[AtomicU64], module: u64, offset: u64) -> usize {
    let generation = GENERATION.load(Acquire);
    let n = THREAD_MODS_LEN.load(Acquire);
    let tp = u64::try_from(crate::tb::tp()).unwrap_or(0);
    let mut found = None;
    let mut count = 0u64;
    // A released ID's entry (module word 0, DL-17) is skipped.
    for w in (0..n).filter_map(module_words).filter(|w| w[3] != 0) {
        let block = tp.wrapping_add_signed(i64::from_ne_bytes(w[4].to_ne_bytes()));
        if let Some(s) = dtv_slot(w[3]).and_then(|i| dtv.get(i)) {
            s.store(block, Relaxed);
            count = count.max(w[3]);
        }
        if w[3] == module {
            found = Some(block);
        }
    }
    if let [g, c, ..] = dtv {
        c.store(count, Release);
        g.store(generation, Release);
    }
    match found.map(|b| b.wrapping_add(offset).wrapping_add(Native::DTP_BIAS)).and_then(|a| usize::try_from(a).ok()) {
        Some(a) => a,
        None => refuse(b"ld.so: __tls_get_addr: no TLS block for this module in this thread\n"),
    }
}
