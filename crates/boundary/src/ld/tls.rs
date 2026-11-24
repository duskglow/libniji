//! Thread-local storage (design §7): the provisional thread block, the static TLS block and its
//! surplus for `dlopen`ed modules, the module table every new thread copies, the dtv and its
//! generation, and `__tls_get_addr`.

use core::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, AtomicUsize};
use core::sync::atomic::Ordering::{AcqRel, Acquire, Relaxed, Release};
use niji_ld::arch::{Arch, Native};
use niji_ld::tls::{self as ltls, Area, Module, Placed};
use niji_rt::tls::{ThreadBlock, FLAG_OWN};
use crate::{conv, raw};
use super::table::{Gens, Table, Wide, INLINE};
use super::dlopen;
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
    INITIAL_DTV.store(dtv_at, Relaxed);
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
/// The first `dlopen::MODS_HEAD` entries live with the link-map nodes (`dlopen::thread_mods_head`, on
/// the page a start already writes); this holds the rest. [`thread_mod`] reaches entry `i`.
static THREAD_MODS: [[AtomicU64; 5]; ltls::MODULES_MAX - dlopen::MODS_HEAD] = [const { [const { AtomicU64::new(0) }; 5] }; ltls::MODULES_MAX - dlopen::MODS_HEAD];
/// Entry `i` of the thread-local module table ([`THREAD_MODS`]), `None` past `MODULES_MAX`.
pub(super) fn thread_mod(i: usize) -> Option<&'static [AtomicU64; 5]> {
    dlopen::thread_mods_head().get(i).or_else(|| THREAD_MODS.get(i.wrapping_sub(dlopen::MODS_HEAD))).or_else(|| thread_mod_past(i))
}

/// Generations of the module table past its first [`ltls::MODULES_MAX`] entries (DL-PROCESS-OBJECTS
/// part 3, audit HL-3): never moved, each mapped by the `dlopen` that first takes an ID in it
/// ([`reserve_thread_mods`]) before the walk publishes the entry; read without a lock below
/// [`THREAD_MODS_LEN`] (Acquire), like the entries in place.
static THREAD_MODS_PAST: Gens<{ ltls::MODULES_MAX }, MODS_GENS> = Gens::new();
/// Generations past the first: `MODULES_MAX << MODS_GENS` is 2^32 entries.
const MODS_GENS: usize = 23;
/// One module table entry's bytes.
const MOD_BYTES: usize = core::mem::size_of::<[AtomicU64; 5]>();

/// [`thread_mod`] past the entries in place: `None` when its generation is not mapped.
#[cold]
#[inline(never)]
fn thread_mod_past(i: usize) -> Option<&'static [AtomicU64; 5]> {
    if i < ltls::MODULES_MAX {
        return None;
    }
    let at = THREAD_MODS_PAST.at(0, MOD_BYTES, i);
    if at == 0 {
        return None;
    }
    // UNSAFE(slice): one entry of a mapped generation of the module table.
    #[allow(unsafe_code)]
    // SAFETY: `at` is entry `i` of a generation `reserve_thread_mods` mapped read-write (page
    // aligned, entries of 8-byte words), never unmapped; every access goes through the atomics.
    Some(unsafe { &*core::ptr::with_exposed_provenance::<[AtomicU64; 5]>(at) })
}

/// Room in the module table for module `id` (entry `id - 1`): its generation mapped. Called by
/// `dlopen` (loader lock held) for an ID it takes, before the walk publishes it; `false` when the
/// mapping cannot be made.
pub(super) fn reserve_thread_mods(id: u64) -> bool {
    let Ok(len) = usize::try_from(id) else { return false };
    len <= ltls::MODULES_MAX || THREAD_MODS_PAST.reserve(MOD_BYTES, len)
}

/// The static surplus's place store (DL-PROCESS-OBJECTS part 3): the first [`ltls::MODULES_MAX`]
/// places in place in the loader, then a contiguous lock-held copy grown by `mremap`
/// ([`Places::grow`]). All-zero is the empty store (the loader's zeroed mapping holds it).
pub(super) struct Places {
    inline: [ltls::Place; ltls::MODULES_MAX],
    wide: Wide<ltls::Place>,
}

impl AsRef<[ltls::Place]> for Places {
    fn as_ref(&self) -> &[ltls::Place] {
        if self.wide.cap() == 0 { &self.inline } else { self.wide.all() }
    }
}

impl AsMut<[ltls::Place]> for Places {
    fn as_mut(&mut self) -> &mut [ltls::Place] {
        if self.wide.cap() == 0 { &mut self.inline } else { self.wide.all_mut() }
    }
}

impl Places {
    /// Room for more places, the ones made kept: the first growth copies the places in place.
    #[cold]
    #[inline(never)]
    pub(super) fn grow(&mut self) -> bool {
        let first = self.wide.cap() == 0;
        let len = self.as_ref().len();
        if !self.wide.reserve(len.saturating_mul(2)) {
            return false;
        }
        if first {
            if let Some(w) = self.wide.slice_mut(len) {
                w.copy_from_slice(&self.inline);
            }
        }
        true
    }
}
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
        let (Some(m), Some(p), Some(slot)) = (m, p, thread_mod(n)) else { continue };
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
    let slot = thread_mod(i)?;
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
        // while the object is loaded: a thread reaches its block only between its `dlopen` and
        // its `dlclose`.
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
        if module == 0 || ltls::dynamic_align(off).is_some() {
            // A released ID (DL-17) has no block until it is reused (entry `i` is ID `i + 1`); an
            // on-demand block (DL-19) is made at the thread's first access ([`tls_slow`]). Either way
            // the slot reads 0, as in a fresh mapping: a reused one holds an earlier thread's vector
            // here (the C library leaves this area to the loader). The slots past the count are
            // never read before a `dlopen` or `tls_slow` writes them.
            let id = if module == 0 { u64::try_from(i).ok()?.wrapping_add(1) } else { module };
            if let Some(w) = dtv_slot(id).and_then(|s| dtv.get(s)) {
                w.store(0, Relaxed);
                if module != 0 {
                    count = count.max(module);
                }
            }
            continue;
        }
        let block = tp64.wrapping_add_signed(i64::from_ne_bytes(off.to_ne_bytes()));
        init_block(block, [src, filesz, memsz], zero)?;
        // A module past the vector's slots (DL-PROCESS-OBJECTS part 3) is left to `tls_slow`, which
        // gives the thread a larger vector at its first access; the count stays within the slots.
        if let Some(w) = dtv_slot(module).and_then(|s| dtv.get(s)) {
            w.store(block, Relaxed);
            count = count.max(module);
        }
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
    /// One entry per module, any number (DL-DLOPEN-NEW-OBJECTS).
    pub(super) mods: Table<[u64; 5], INLINE>,
    generation: u64,
    /// The modules are unloaded ([`release_tls`], DL-17), not added.
    release: bool,
}

impl NewTls {
    /// No modules (nothing written, nothing mapped).
    #[inline(always)]
    pub(super) fn none() -> NewTls {
        NewTls { mods: Table::new([0; 5]), generation: 0, release: false }
    }
}

/// The static surplus `[lo, hi)` below every thread pointer and the thread pointer's alignment
/// (DL-16d); all 0 before [`record_thread_tls`].
pub(super) fn surplus() -> (u64, u64, u64) {
    (SURPLUS_LO.load(Relaxed), SURPLUS_HI.load(Relaxed), THREAD_TLS_ALIGN.load(Relaxed))
}

/// The IDs [`THREAD_MODS`] covers already: the floor of the IDs a `dlopen` may hand out
/// (`ltls::ModuleIds::take`). No ceiling (DL-PROCESS-OBJECTS part 3): a thread whose vector lacks a
/// module's slot gets a larger one at its first access ([`tls_slow`]).
pub(super) fn module_floor() -> u64 {
    u64::try_from(THREAD_MODS_LEN.load(Acquire)).unwrap_or(u64::MAX)
}

/// The initial thread's dtv (in its area; [`install_tls`]): with the vectors at `tp - THREAD_TLS_SIZE`
/// of the threads the C library makes, the vectors in place, which hold `THREAD_DTV_WORDS` words.
/// Any other vector a thread block names is one [`grow_dtv`] mapped.
static INITIAL_DTV: AtomicUsize = AtomicUsize::new(0);
/// The C library's thread walk, as the first [`publish_tls`] was given it (0: none yet).
static WALK: AtomicUsize = AtomicUsize::new(0);

/// Whether `dtv` is the vector in place of the thread at `tp`.
fn in_place(tp: usize, dtv: usize) -> bool {
    dtv == INITIAL_DTV.load(Relaxed) || dtv == tp.wrapping_sub(usize::try_from(THREAD_TLS_SIZE.load(Relaxed)).unwrap_or(0))
}

/// The words of the vector `dtv` of the thread at `tp`: `THREAD_DTV_WORDS` in place, else the
/// count [`grow_dtv`] wrote before it (the grown vector's mapping: the vector it replaced, its
/// words, then the vector).
fn dtv_room(tp: usize, dtv: usize) -> usize {
    let words = if in_place(tp, dtv) {
        THREAD_DTV_WORDS.load(Relaxed)
    } else {
        // UNSAFE(slice): the word before a grown vector.
        #[allow(unsafe_code)]
        // SAFETY: a vector not in place is one `grow_dtv` mapped (never unmapped while a thread
        // block names it, nor before a walk that read its address ends: `release_grown`), two
        // words after its mapping's start; the word before it was written before its address was
        // stored (Release) in the block this address was read from (Acquire).
        unsafe { (*core::ptr::with_exposed_provenance::<AtomicU64>(dtv.wrapping_sub(8))).load(Relaxed) }
    };
    usize::try_from(words).unwrap_or(0)
}

/// Give the calling thread (at `tp`, its vector `old` of `old_words` words) a vector with slots
/// for module `need` and the `n` modules loaded now, at least twice as many (DL-PROCESS-OBJECTS
/// part 3): a mapping of its own, `[old, words, vector...]`, named by the thread block from now on.
/// `old` stays mapped: a `dlopen`'s walk may still write it; the vectors a thread grew are unmapped
/// when it ends ([`thread_tls_release`]). No mapping: the process cannot go on (as a module
/// the table does not hold).
#[cold]
#[inline(never)]
fn grow_dtv(tp: usize, old: usize, old_words: usize, need: u64, n: usize) -> &'static [AtomicU64] {
    let fail = || -> ! { refuse(b"ld.so: __tls_get_addr: cannot allocate a larger TLS vector\n") };
    let header = usize::try_from(ltls::DTV_HEADER).unwrap_or(0);
    let need = usize::try_from(need).unwrap_or(usize::MAX).max(n);
    let mut slots = old_words.saturating_sub(header).max(1).saturating_mul(2);
    while slots < need {
        slots = slots.saturating_mul(2);
    }
    let words = slots.checked_add(header).unwrap_or_else(|| fail());
    let Some(bytes) = words.checked_add(2).and_then(|w| w.checked_mul(8)) else { fail() };
    let prot = i32::try_from(niji_kernel::uapi::PROT_READ | niji_kernel::uapi::PROT_WRITE).unwrap_or(0);
    let flags = i32::try_from(niji_kernel::uapi::MAP_PRIVATE | niji_kernel::uapi::MAP_ANONYMOUS).unwrap_or(0);
    let Ok(map) = raw::mmap(0, bytes, prot, flags, -1, 0) else { fail() };
    let head = dtv_words(map, 2);
    if let [prev, room] = head {
        prev.store(u64::try_from(old).unwrap_or(0), Relaxed);
        room.store(u64::try_from(words).unwrap_or(0), Relaxed);
    }
    let dtv = map.wrapping_add(16);
    let walk = WALK.load(Acquire);
    let mut grow = Grow { tp, old, old_words, new: dtv, words };
    if walk == 0 {
        // No `dlopen` has loaded TLS: no walk writes vectors and no slot holds an on-demand block.
        crate::tb::listed(tp).dtv.store(dtv, Release);
    } else {
        // Under the thread-list lock (DL-19): the slots move to the new vector, and the old
        // vector's are emptied, so a walk sees each on-demand block in exactly one vector.
        #[allow(clippy::as_conversions)] // a function's address has no conversion other than `as`
        let visit = (grow_visit as *const ()).expose_provenance();
        let ctx = core::ptr::from_mut::<Grow>(&mut grow).expose_provenance();
        let arg = |v: usize| i64::from_ne_bytes(v.to_ne_bytes());
        let _ = crate::ffi::niji_machine_call3(walk, arg(visit), arg(ctx), 0);
    }
    dtv_words(dtv, words)
}

/// A vector [`grow_dtv`] made, to be named by the thread block of the thread at `tp` in place of
/// `old` (`old_words` words).
struct Grow {
    tp: usize,
    old: usize,
    old_words: usize,
    new: usize,
    words: usize,
}

/// The step of the C library's thread walk [`grow_dtv`] runs (its thread-list lock held): at the
/// end (`tp` 0), the old vector's module slots move to the new one, which the thread block names
/// from then on.
extern "C" fn grow_visit(ctx: usize, tp: usize) -> usize {
    if tp != 0 {
        return 0;
    }
    // UNSAFE(slice): the walk's context.
    #[allow(unsafe_code)]
    // SAFETY: [`grow_dtv`] passes the address of a live `Grow` on its stack and the walk calls back
    // before it returns.
    let Some(g) = (unsafe { core::ptr::with_exposed_provenance::<Grow>(ctx).as_ref() }) else { return 0 };
    let header = usize::try_from(ltls::DTV_HEADER).unwrap_or(0);
    let (old, new) = (dtv_words(g.old, g.old_words), dtv_words(g.new, g.words));
    for (o, n) in old.iter().zip(new).skip(header) {
        n.store(o.swap(0, Relaxed), Relaxed);
    }
    crate::tb::listed(g.tp).dtv.store(g.new, Release);
    0
}

/// A no-op step of the C library's thread walk: [`release_grown`] calls the walk only for its lock.
extern "C" fn tls_noop(_ctx: usize, _tp: usize) -> usize {
    0
}

/// The vectors the thread at `tp` grew (the newest `at`), unmapped as it ends: its block names its
/// vector in place again, then an empty walk waits out any walk that read a grown vector's address
/// before (a walk holds the thread-list lock throughout), so nothing writes them any more.
#[cold]
#[inline(never)]
fn release_grown(tp: usize, at: usize) {
    let mut v = at;
    let mut home = at;
    while v != 0 && !in_place(tp, v) {
        home = dtv_words(v.wrapping_sub(16), 1).first().map_or(0, |w| usize::try_from(w.load(Relaxed)).unwrap_or(0));
        v = home;
    }
    crate::tb::listed(tp).dtv.store(home, Release);
    let walk = WALK.load(Acquire);
    if walk != 0 {
        #[allow(clippy::as_conversions)] // a function's address has no conversion other than `as`
        let noop = (tls_noop as *const ()).expose_provenance();
        let _ = crate::ffi::niji_machine_call3(walk, i64::from_ne_bytes(noop.to_ne_bytes()), 0, 0);
    }
    let mut v = at;
    while v != 0 && v != home {
        let map = v.wrapping_sub(16);
        let [prev, room] = [0usize, 1].map(|i| dtv_words(map, 2).get(i).map_or(0, |w| usize::try_from(w.load(Relaxed)).unwrap_or(0)));
        let _ = raw::munmap(map, room.saturating_add(2).saturating_mul(8));
        v = prev;
    }
}

/// The TLS generation now (DL-17: `ltls::ModuleIds::release` makes the next).
pub(super) fn tls_generation() -> u64 {
    GENERATION.load(Relaxed)
}

/// Take `gone`'s modules (`mods[..len]`, ID in word 3) out of [`THREAD_MODS`] at the end of the C
/// library's thread walk (its thread-list lock held, as [`publish_tls`]) and make `generation`
/// current: a thread whose dtv still names an old block repairs it in [`tls_slow`] (DL-17). A
/// static block's bytes belong to the surplus place, freed already; each thread's on-demand block
/// (DL-19) is unmapped by the walk's step for that thread ([`tls_visit`]).
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
/// initially loaded one, or (for `dlinfo`) a `dlopen`ed one [`tls_visit`] has appended. A module
/// with on-demand blocks (DL-19; its offset word is `ltls::DYNAMIC | align`) has `ltls::NO_STATIC`.
pub(super) fn initial_place(image: u64) -> Option<Placed> {
    let place = |w: [u64; 5]| {
        let tp_offset = if ltls::dynamic_align(w[4]).is_some() { ltls::NO_STATIC } else { i64::from_ne_bytes(w[4].to_ne_bytes()) };
        Placed { module: w[3], tp_offset }
    };
    (0..THREAD_MODS_LEN.load(Acquire)).filter_map(module_words).find(|w| w[0] == image).map(place)
}

/// Give `new`'s modules to every thread (DL-16d, plan/ph14/dl-16-dlopen.md §5 points 1 and 3):
/// the C library's `walk` calls [`tls_visit`] for each listed thread, then once with 0, all under
/// its thread-list lock. Called by `dlopen` (loader lock held) after relocation, before publication.
pub(super) fn publish_tls(walk: usize, new: &mut NewTls) {
    new.generation = GENERATION.load(Relaxed).wrapping_add(1);
    WALK.store(walk, Release);
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
    let mods: &[[u64; 5]] = &new.mods;
    if tp == 0 {
        // Entry `id - 1` holds module `id` (DL-17: a released ID's entry is all zero until reused).
        let mut n = THREAD_MODS_LEN.load(Relaxed);
        for m in mods {
            let Some(slot) = usize::try_from(m[3]).ok().and_then(|id| id.checked_sub(1)).and_then(thread_mod) else { continue };
            let words = if new.release { [0; 5] } else { *m };
            slot.iter().zip(words).for_each(|(w, v)| w.store(v, Relaxed));
            if !new.release {
                n = n.max(usize::try_from(m[3]).unwrap_or(0));
                if ltls::dynamic_align(m[4]).is_some() {
                    DYNAMIC_SEEN.store(true, Release);
                }
            }
        }
        THREAD_MODS_LEN.store(n, Release);
        GENERATION.store(new.generation, Release);
        return 0;
    }
    let b = crate::tb::listed(tp);
    // Only a block this loader made has a dtv of this layout (D-5.11 safety, design §7).
    if b.flags.load(Relaxed) & FLAG_OWN == 0 {
        return 0;
    }
    // The thread's own vector, in place or grown (DL-PROCESS-OBJECTS part 3): a slot it lacks is
    // skipped, and the thread grows its vector at its first access to that module.
    let at = b.dtv.load(Acquire);
    let dtv = dtv_words(at, dtv_room(tp, at));
    if new.release {
        // The thread's on-demand blocks of the modules going (DL-19), from their entries (still in
        // the table until the walk's end). The swap pairs with the thread's own at its exit
        // ([`release_blocks`]): one of the two frees each block.
        for m in mods {
            let Some(w) = usize::try_from(m[3]).ok().and_then(|id| id.checked_sub(1)).and_then(module_words) else { continue };
            if w[3] == m[3] && ltls::dynamic_align(w[4]).is_some() {
                if let Some(s) = dtv_slot(m[3]).and_then(|i| dtv.get(i)) {
                    block_free(s.swap(0, AcqRel), w[2]);
                }
            }
        }
        return 0;
    }
    let mut count = None;
    for &[src, filesz, memsz, module, off] in mods {
        let block = if ltls::dynamic_align(off).is_some() {
            // No block until the thread's first access (DL-19).
            0
        } else {
            let block = u64::try_from(tp).unwrap_or(0).wrapping_add_signed(i64::from_ne_bytes(off.to_ne_bytes()));
            if init_block(block, [src, filesz, memsz], true).is_none() {
                continue;
            }
            block
        };
        if let Some(w) = dtv_slot(module).and_then(|i| dtv.get(i)) {
            w.store(block, Relaxed);
            count = Some(count.map_or(module, |c: u64| c.max(module)));
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
/// `p_memsz`, and every dtv word up to the count it sets is written (the C library may reuse an
/// exited thread's mapping, and it does not clear this area; the rest of it, the free surplus, is
/// written by the `dlopen` that places a block there, [`tls_visit`], before anything reads it), the dtv goes in the lowest words
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

/// `StartInfo::tls_release` (DL-15, design §7): the thread at `tp` is ending (still listed). Its
/// static blocks and dtv lie in the C library's mapping and go with it; the loader's own per-thread
/// state does not: its on-demand blocks (DL-19) and the vectors it grew.
pub(super) extern "C" fn thread_tls_release(tp: usize) -> usize {
    if DYNAMIC_SEEN.load(Acquire) {
        release_blocks(tp);
    }
    // The vectors the thread grew (DL-PROCESS-OBJECTS part 3) are the loader's.
    let at = crate::tb::listed(tp).dtv.load(Relaxed);
    if at != 0 && !in_place(tp, at) {
        release_grown(tp, at);
    }
    0
}

/// Whether a module with on-demand blocks (DL-19) was ever published: only then does a thread's
/// exit look for blocks to free. Set by the walk that publishes the first one, before any thread
/// can reach it.
static DYNAMIC_SEEN: AtomicBool = AtomicBool::new(false);

/// The page size on-demand blocks are mapped in.
// ARCH(x86_64): 4 KiB pages; another architecture takes its page size from the auxiliary vector.
const PAGE: u64 = 4096;

/// The bytes a block of `memsz` bytes is mapped in (at least one page: a slot holding 0 means none).
fn block_len(memsz: u64) -> Option<usize> {
    usize::try_from(memsz.max(1).checked_next_multiple_of(PAGE)?).ok()
}

/// A new on-demand block (DL-19) for module entry `[image, p_filesz, p_memsz]` with alignment
/// `align`: a private zero mapping holding its image, aligned as the module asks. The calling
/// thread's own (only it stores the address in its vector). No memory ends the process as the
/// reference does (probe-settled: the text, status 127): the access has no error path.
#[cold]
#[inline(never)]
fn block_alloc(image: [u64; 3], align: u64) -> usize {
    let fail = || -> ! { refuse(b"cannot allocate memory for thread-local data: ABORT\n") };
    let ([_, _, memsz], Some(len), Ok(align)) = (image, block_len(image[2]), usize::try_from(align)) else { fail() };
    let page = usize::try_from(PAGE).unwrap_or(0);
    let extra = if align > page { align } else { 0 };
    let prot = i32::try_from(niji_kernel::uapi::PROT_READ | niji_kernel::uapi::PROT_WRITE).unwrap_or(0);
    let flags = i32::try_from(niji_kernel::uapi::MAP_PRIVATE | niji_kernel::uapi::MAP_ANONYMOUS).unwrap_or(0);
    let Some(map_len) = len.checked_add(extra) else { fail() };
    let Ok(map) = raw::mmap(0, map_len, prot, flags, -1, 0) else { fail() };
    let block = map.checked_next_multiple_of(align.max(1)).unwrap_or_else(|| fail());
    if extra != 0 {
        // Keep exactly the block's pages: the free only knows `p_memsz`.
        let head = block.wrapping_sub(map);
        if head != 0 {
            let _ = raw::munmap(map, head);
        }
        let tail = extra.wrapping_sub(head);
        if tail != 0 {
            let _ = raw::munmap(block.wrapping_add(len), tail);
        }
    }
    // The mapping is zero already: only the image is written.
    if block_image(block, image[0], image[1].min(memsz)).is_none() {
        fail()
    }
    block
}

/// Copy `len` bytes of a module's TLS image from `src` to the new on-demand block at `block`
/// (DL-19), a byte at a time through atomics. This runs under the TLS descriptor trampoline
/// (tlsdesc.S), which keeps only the x87/SSE state, so it must not reach `memcpy`: the copy
/// kernels may use AVX and end in `vzeroupper`, clearing the caller's upper ymm halves. Atomic
/// accesses are never turned into a library call. Cold: once per thread and module.
#[cold]
#[inline(never)]
fn block_image(block: usize, src: u64, len: u64) -> Option<()> {
    let (src, len) = (usize::try_from(src).ok()?, usize::try_from(len).ok()?);
    if len == 0 {
        return Some(());
    }
    if block == 0 || src == 0 {
        return None;
    }
    // UNSAFE(slice): a new on-demand block, as atomic bytes.
    #[allow(unsafe_code)]
    // SAFETY: [`block_alloc`] just mapped `block` read-write with at least `p_memsz` >= `len`
    // bytes (page-rounded), no other thread knows it yet; `AtomicU8` has `u8`'s layout.
    let to = unsafe { core::slice::from_raw_parts(core::ptr::with_exposed_provenance::<AtomicU8>(block), len) };
    // UNSAFE(slice): the module's TLS initialisation image in its loaded object, as atomic bytes.
    #[allow(unsafe_code)]
    // SAFETY: `ltls::image_in_load` / `program_tls` checked that [p_vaddr, p_vaddr + p_filesz) lies
    // in the file part of one of the object's PT_LOADs, mapped readable while the object is loaded
    // (a thread makes a block only between its `dlopen` and `dlclose`); only read.
    let from = unsafe { core::slice::from_raw_parts(core::ptr::with_exposed_provenance::<AtomicU8>(src), len) };
    for (d, s) in to.iter().zip(from) {
        d.store(s.load(Relaxed), Relaxed);
    }
    Some(())
}

/// Free the on-demand block at `block` of a module of `p_memsz` `memsz` ([`block_alloc`]).
fn block_free(block: u64, memsz: u64) {
    if let (Ok(at), Some(len)) = (usize::try_from(block), block_len(memsz)) {
        if at != 0 {
            let _ = raw::munmap(at, len);
        }
    }
}

/// Free the on-demand blocks of the ending thread at `tp` (DL-19). Each slot is taken with a swap:
/// a `dlclose` walking the threads now ([`tls_visit`]) takes it the same way, so exactly one of them
/// frees each block. A module whose table entry is already cleared was taken by that walk.
#[cold]
#[inline(never)]
fn release_blocks(tp: usize) {
    let at = crate::tb::listed(tp).dtv.load(Acquire);
    let dtv = dtv_words(at, dtv_room(tp, at));
    for w in (0..THREAD_MODS_LEN.load(Acquire)).filter_map(module_words) {
        if w[3] == 0 || ltls::dynamic_align(w[4]).is_none() {
            continue;
        }
        if let Some(s) = dtv_slot(w[3]).and_then(|i| dtv.get(i)) {
            block_free(s.swap(0, AcqRel), w[2]);
        }
    }
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
    // A grown vector (DL-PROCESS-OBJECTS part 3) has more words than the fast path reads: a current
    // one answers here, else it is the vector to fill.
    let me = crate::tb::tp();
    let at = crate::tb::read::<crate::tb::Dtv>();
    let mut dtv = dtv;
    if !in_place(me, at) {
        dtv = dtv_words(at, dtv_room(me, at));
        let word = |i: u64| -> u64 { usize::try_from(i).ok().and_then(|i| dtv.get(i)).map_or(0, |w| w.load(Relaxed)) };
        if let Some(a) = ltls::lookup::<Native>(word, GENERATION.load(Relaxed), module, offset).and_then(|a| usize::try_from(a).ok()) {
            return a;
        }
    }
    let generation = GENERATION.load(Acquire);
    let n = THREAD_MODS_LEN.load(Acquire);
    if dtv_slot(module).is_some_and(|s| s >= dtv.len()) && usize::try_from(module).is_ok_and(|m| m <= n) {
        dtv = grow_dtv(me, at, dtv.len(), module, n);
    }
    let tp = u64::try_from(me).unwrap_or(0);
    let mut found = None;
    let mut count = 0u64;
    // A released ID's entry (module word 0, DL-17) is skipped.
    for w in (0..n).filter_map(module_words).filter(|w| w[3] != 0) {
        let slot = dtv_slot(w[3]).and_then(|i| dtv.get(i));
        let block = if let Some(align) = ltls::dynamic_align(w[4]) {
            // An on-demand block (DL-19): the slot is the thread's own (0 until made). The module
            // asked for gets its block now.
            let mut block = slot.map_or(0, |s| s.load(Relaxed));
            if block == 0 && w[3] == module {
                let Some(s) = slot else { refuse(b"ld.so: __tls_get_addr: no TLS block for this module in this thread\n") };
                block = u64::try_from(block_alloc([w[0], w[1], w[2]], align)).unwrap_or(0);
                s.store(block, Release);
            }
            block
        } else {
            let block = tp.wrapping_add_signed(i64::from_ne_bytes(w[4].to_ne_bytes()));
            if let Some(s) = slot {
                s.store(block, Relaxed);
            }
            block
        };
        if slot.is_some() {
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

/// `_dl_get_tls_static_info(size_t *size, size_t *align)` (`GLIBC_PRIVATE` in versions.tsv, here
/// `NIJI_PRIVATE`, R14-12): the static TLS every thread has, for a runtime that finds it by name
/// (sanitizers' `dlsym`). `*size`: the bytes from the lowest static block (the surplus included,
/// `SURPLUS_HI` below the thread pointer) to the end of the initial thread block above it
/// ([`INITIAL_TAIL`]); `*align`: the thread pointer's alignment. A null or misaligned pointer is not
/// written. Both 0 in a process this loader did not start (nothing laid out). Off the start path
/// (D-27.65).
// UNSAFE(export): an exported function of the ld.so product.
#[unsafe(no_mangle)]
#[allow(unsafe_code)]
#[cold]
#[inline(never)]
pub extern "C" fn _dl_get_tls_static_info(size: usize, align: usize) {
    let below = SURPLUS_HI.load(Relaxed);
    let total = if below == 0 { 0 } else { below.saturating_add(INITIAL_TAIL) };
    if let Some(w) = crate::tb::c_word(size) {
        w.store(usize::try_from(total).unwrap_or(usize::MAX), Relaxed);
    }
    if let Some(w) = crate::tb::c_word(align) {
        w.store(usize::try_from(THREAD_TLS_ALIGN.load(Relaxed)).unwrap_or(0), Relaxed);
    }
}

/// The calling thread's address of `offset` in the block of module `p` (`dlsym` of an `STT_TLS`
/// symbol, `dlinfo`'s `RTLD_DI_TLS_DATA`): a static block's at its offset from the thread pointer;
/// an on-demand block's (DL-19) through `__tls_get_addr`, which makes it now when `make`, else the
/// block the thread has (0 when it has none yet: the reference's documented `dlinfo` answer).
pub(super) fn thread_address(p: Placed, offset: u64, make: bool) -> usize {
    let tp = crate::tb::tp();
    if p.tp_offset != ltls::NO_STATIC {
        let at = tp.wrapping_add_signed(isize::try_from(p.tp_offset).unwrap_or(0));
        return at.wrapping_add(usize::try_from(offset).unwrap_or(0));
    }
    if make {
        let index = [p.module, offset];
        return __tls_get_addr(core::ptr::from_ref(&index).expose_provenance());
    }
    let at = crate::tb::read::<crate::tb::Dtv>();
    let dtv = dtv_words(at, dtv_room(tp, at));
    let count = dtv.get(1).map_or(0, |w| w.load(Relaxed));
    let block = dtv_slot(p.module).filter(|_| p.module <= count).and_then(|i| dtv.get(i)).map_or(0, |w| w.load(Relaxed));
    if block == 0 { 0 } else { usize::try_from(block.wrapping_add(offset)).unwrap_or(0) }
}

/// The records of the on-demand TLS descriptor function (DL-19, tlsdesc.S
/// `niji_machine_ld_tlsdesc_ondemand`): four words each, `[resolver, module ID, offset, address of
/// GENERATION]` (words 1 and 2 a `tls_index`; the fast path compares the thread's vector against
/// word 3's generation as `__tls_get_addr` does), in pages the loader maps, never unmapped: a
/// descriptor's argument points at one. Word 0 of a page links the next page. A free record is
/// `[0, 0, next free, _]`. Changed only with the loader lock held; the descriptor function reads a
/// record a relocation wrote before its object was published.
static DESC_PAGES: AtomicUsize = AtomicUsize::new(0);
/// The first free record (0: none).
static DESC_FREE: AtomicUsize = AtomicUsize::new(0);
/// Bytes of one record.
const DESC_BYTES: usize = 32;
/// Records in one page after its link word.
const DESC_PER_PAGE: usize = (4096 - 8) / DESC_BYTES;

/// The four words of the record at `at` (or the link word of a page at `at`, as its first).
fn desc_words(at: usize) -> &'static [AtomicU64] {
    if at == 0 || at % 8 != 0 {
        return &[];
    }
    // UNSAFE(slice): a record of the dynamic TLS descriptor function, as atomic words.
    #[allow(unsafe_code)]
    // SAFETY: `at` is a page [`desc_record`] mapped read-write (never unmapped) or a record in one
    // (8 + 32 k bytes in, k < DESC_PER_PAGE: four words inside the page); every access goes
    // through these atomics.
    unsafe {
        core::slice::from_raw_parts(core::ptr::with_exposed_provenance::<AtomicU64>(at), 4)
    }
}

/// A record for offset `offset` in module `module`'s block: the argument of a dynamic TLS
/// descriptor. Loader lock held (relocation). `None` when no page can be mapped.
pub(super) fn desc_record(module: u64, offset: u64) -> Option<usize> {
    if DESC_FREE.load(Relaxed) == 0 {
        let prot = i32::try_from(niji_kernel::uapi::PROT_READ | niji_kernel::uapi::PROT_WRITE).unwrap_or(0);
        let flags = i32::try_from(niji_kernel::uapi::MAP_PRIVATE | niji_kernel::uapi::MAP_ANONYMOUS).unwrap_or(0);
        let page = raw::mmap(0, 4096, prot, flags, -1, 0).ok()?;
        desc_words(page).first()?.store(u64::try_from(DESC_PAGES.load(Relaxed)).ok()?, Relaxed);
        DESC_PAGES.store(page, Relaxed);
        for k in (0..DESC_PER_PAGE).rev() {
            let at = page.wrapping_add(8).wrapping_add(k.wrapping_mul(DESC_BYTES));
            desc_words(at).get(2)?.store(u64::try_from(DESC_FREE.load(Relaxed)).ok()?, Relaxed);
            DESC_FREE.store(at, Relaxed);
        }
    }
    let at = DESC_FREE.load(Relaxed);
    let [f, m, o, g] = desc_words(at) else { return None };
    DESC_FREE.store(usize::try_from(o.load(Relaxed)).ok()?, Relaxed);
    #[allow(clippy::as_conversions)] // a function's address has no conversion other than `as`
    let resolver = (tlsdesc_resolve as *const ()).expose_provenance();
    o.store(offset, Relaxed);
    m.store(module, Relaxed);
    g.store(u64::try_from(GENERATION.as_ptr().expose_provenance()).ok()?, Relaxed);
    f.store(u64::try_from(resolver).ok()?, Release);
    Some(at)
}

/// Free the records of module `module` (its object is unloaded, or its `dlopen` failed). Loader
/// lock held. No descriptor that names them can run any more: their objects are gone too.
pub(super) fn desc_release(module: u64) {
    let mut page = DESC_PAGES.load(Relaxed);
    while page != 0 {
        for k in 0..DESC_PER_PAGE {
            let at = page.wrapping_add(8).wrapping_add(k.wrapping_mul(DESC_BYTES));
            let [f, m, o, _] = desc_words(at) else { continue };
            if f.load(Relaxed) != 0 && m.load(Relaxed) == module {
                f.store(0, Relaxed);
                m.store(0, Relaxed);
                o.store(u64::try_from(DESC_FREE.load(Relaxed)).unwrap_or(0), Relaxed);
                DESC_FREE.store(at, Relaxed);
            }
        }
        page = desc_words(page).first().map_or(0, |w| usize::try_from(w.load(Relaxed)).unwrap_or(0));
    }
}

/// The resolver of a dynamic TLS descriptor's record (tlsdesc.S): the variable's address in the
/// calling thread, as `__tls_get_addr` gives it for the record's `tls_index` (words 1 and 2).
extern "C" fn tlsdesc_resolve(record: usize) -> usize {
    __tls_get_addr(record.wrapping_add(8))
}

/// Bytes of the static surplus set by `glibc.rtld.optional_static_tls` (0: not named; never 0
/// once named, the fixed part is added; D-27.94).
static SURPLUS_ASKED: AtomicU64 = AtomicU64::new(0);

/// The static surplus every thread's area has (D-27.94): `ltls::SURPLUS` (1712 bytes), or
/// `ltls::SURPLUS_FIXED` plus the `glibc.rtld.optional_static_tls` value when that is named.
pub(super) fn surplus_bytes() -> u64 {
    match SURPLUS_ASKED.load(Relaxed) {
        0 => ltls::SURPLUS,
        asked => asked,
    }
}

/// Record the start's `glibc.rtld.optional_static_tls` (read only when `GLIBC_TUNABLES` names it):
/// the surplus becomes the value plus `ltls::SURPLUS_FIXED`, lower or higher than the default.
#[cold]
#[inline(never)]
pub(super) fn ask_surplus(bytes: u64) {
    SURPLUS_ASKED.store(bytes.saturating_add(ltls::SURPLUS_FIXED), Relaxed);
}
