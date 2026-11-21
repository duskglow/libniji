//! Tables that grow with the number of objects (DL-STARTUP-OBJECTS, GitHub issue #9): the loader's
//! start-time tables (load list, dependency links, search chains, init and fini order, TLS modules,
//! the records `dlopen` and the exit read) hold their first `N` entries inline, so a program with a
//! handful of objects maps nothing new, and spill to a private anonymous mapping beyond that,
//! doubling. The loader has no heap (its allocator is `mmap`); the only failure is the mapping's,
//! reported by the caller as an allocation failure. No count of objects is refused.

use core::marker::PhantomData;
use core::mem::{align_of, size_of, ManuallyDrop, MaybeUninit};
use core::ops::{Deref, DerefMut};
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering::{Acquire, Relaxed, Release};

use niji_ld::map;

use niji_ld::search::PATH_MAX;

use super::{MAP_ANONYMOUS_I32, MAP_PRIVATE_I32};
use crate::{conv, raw};

/// Entries a start-time table holds inline: the object count up to which a start maps no table.
/// DYN-START-PARITY: 16, not 64. The start's tables live in one stack frame, and every page of that
/// frame is touched when it is set up, so 64 entries per table (the lookup views alone are 296 bytes
/// each) cost a dynamic start about a dozen page faults it did not need.
pub(super) const INLINE: usize = 16;

/// A table could not grow: no address space (or a size that does not fit a word).
#[derive(Clone, Copy, Debug)]
pub(super) struct NoMemory;

/// A table of `T`: its first `N` entries inline, never written until reserved, then a mapping
/// ([`Table::reserve`], which sets every new entry to the table's fill value). Dereferences to its
/// `len` entries, the most reserved so far.
pub(super) struct Table<T: Copy, const N: usize> {
    inline: [MaybeUninit<T>; N],
    /// Entries reserved (each holds a value), in `inline` or in the mapping.
    len: usize,
    /// The mapping holding the entries once they outgrow `inline` (0: none) and its entry count.
    heap: usize,
    cap: usize,
    fill: T,
}

impl<T: Copy, const N: usize> Table<T, N> {
    /// A mapping is page-aligned and holds at least one entry.
    const FITS: () = assert!(size_of::<T>() > 0 && align_of::<T>() <= 4096);

    /// No entries; nothing written, nothing mapped.
    #[inline(always)]
    pub(super) fn new(fill: T) -> Self {
        let () = Self::FITS;
        Self { inline: [const { MaybeUninit::uninit() }; N], len: 0, heap: 0, cap: 0, fill }
    }

    /// The first entry's address and the entries the storage holds.
    fn storage(&mut self) -> (*mut T, usize) {
        if self.heap == 0 {
            (self.inline.as_mut_ptr().cast::<T>(), N)
        } else {
            (core::ptr::with_exposed_provenance_mut::<T>(self.heap), self.cap)
        }
    }

    /// At least `len` entries: the entries so far are kept, each new one holds the fill value.
    ///
    /// # Errors
    ///
    /// [`NoMemory`] when a mapping is needed and fails; the table is unchanged.
    pub(super) fn reserve(&mut self, len: usize) -> Result<(), NoMemory> {
        if len <= self.len {
            return Ok(());
        }
        let (mut p, room) = self.storage();
        if len > room {
            let cap = len.max(room.saturating_mul(2));
            let bytes = cap.checked_mul(size_of::<T>()).ok_or(NoMemory)?;
            let at = raw::mmap(0, bytes, map::PROT_READ | map::PROT_WRITE, MAP_PRIVATE_I32 | MAP_ANONYMOUS_I32, -1, 0).map_err(|_| NoMemory)?;
            let to = core::ptr::with_exposed_provenance_mut::<T>(at);
            // UNSAFE(slice): the table's entries copied into its new mapping.
            #[allow(unsafe_code)]
            // SAFETY: `p` holds `self.len` written entries (aligned, in the old storage); `to` is a fresh private
            // read-write mapping of `cap` >= `len` > `self.len` entries, page-aligned (so aligned for `T`: FITS),
            // which nothing else refers to; the two do not overlap.
            unsafe {
                core::ptr::copy_nonoverlapping(p, to, self.len);
            }
            self.release();
            (self.heap, self.cap, p) = (at, cap, to);
        }
        for i in self.len..len {
            // UNSAFE(slice): one new entry of the table, written before it is reserved.
            #[allow(unsafe_code)]
            // SAFETY: `i < len`, at most the storage's entry count (inline `N`, or the mapping's `cap`, grown above),
            // `p` its aligned first entry; the table is borrowed uniquely.
            unsafe {
                p.wrapping_add(i).write(self.fill);
            }
        }
        self.len = len;
        Ok(())
    }

    /// Append `v` (one more entry, growing as [`Table::reserve`] does).
    ///
    /// # Errors
    ///
    /// [`NoMemory`] when the table must grow and cannot; the table is unchanged.
    pub(super) fn push(&mut self, v: T) -> Result<(), NoMemory> {
        let n = self.len;
        self.reserve(n.checked_add(1).ok_or(NoMemory)?)?;
        if let Some(slot) = self.get_mut(n) {
            *slot = v;
        }
        Ok(())
    }

    /// No entries; the storage is kept, so as many pushes as the table held before need no memory.
    pub(super) fn clear(&mut self) {
        self.len = 0;
    }

    /// Unmap the table's mapping, if it has one (its entries are `Copy`: nothing to drop).
    fn release(&mut self) {
        if self.heap != 0 {
            let _ = raw::munmap(self.heap, self.cap.saturating_mul(size_of::<T>()));
            (self.heap, self.cap) = (0, 0);
        }
    }
}

impl<T: Copy, const N: usize> Deref for Table<T, N> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        let p = if self.heap == 0 { self.inline.as_ptr().cast::<T>() } else { core::ptr::with_exposed_provenance::<T>(self.heap) };
        // UNSAFE(slice): a table's reserved entries.
        #[allow(unsafe_code)]
        // SAFETY: `reserve` wrote the first `len` entries of the current storage (inline, or the mapping it made,
        // aligned for `T`, which only this table refers to and which stays mapped until `release`, called only
        // through `&mut self`, so not while this borrow lives).
        unsafe {
            core::slice::from_raw_parts(p, self.len)
        }
    }
}

impl<T: Copy, const N: usize> DerefMut for Table<T, N> {
    fn deref_mut(&mut self) -> &mut [T] {
        let len = self.len;
        let (p, _) = self.storage();
        // UNSAFE(slice): a table's reserved entries, borrowed uniquely.
        #[allow(unsafe_code)]
        // SAFETY: as for `deref`; the borrow of `self` is unique, so is the slice.
        unsafe {
            core::slice::from_raw_parts_mut(p, len)
        }
    }
}

impl<T: Copy, const N: usize> Drop for Table<T, N> {
    fn drop(&mut self) {
        self.release();
    }
}

/// A table handed over for the process's lifetime: published once at start (before the program
/// runs), then read by `dlopen` and the exit, never written again.
pub(super) struct Kept<T> {
    at: AtomicUsize,
    len: AtomicUsize,
    of: PhantomData<T>,
}

impl<T: Copy> Kept<T> {
    /// Nothing kept.
    pub(super) const fn new() -> Self {
        Self { at: AtomicUsize::new(0), len: AtomicUsize::new(0), of: PhantomData }
    }

    /// Keep the first `len` entries of `t` (mapped for this: an inline table is moved to a mapping)
    /// for the process's lifetime.
    ///
    /// # Errors
    ///
    /// [`NoMemory`] when `t` has no mapping and none can be made.
    pub(super) fn keep<const N: usize>(&self, mut t: Table<T, N>, len: usize) -> Result<(), NoMemory> {
        // A kept table lives in a mapping (an inline one would go with the caller's frame).
        t.reserve(len)?;
        if t.heap == 0 {
            t.reserve(N.saturating_add(1))?;
        }
        let t = ManuallyDrop::new(t);
        self.at.store(t.heap, Relaxed);
        self.len.store(len.min(t.cap), Release);
        Ok(())
    }

    /// The kept entries (none: nothing kept).
    pub(super) fn get(&self) -> &'static [T] {
        let len = self.len.load(Acquire);
        let at = self.at.load(Relaxed);
        if len == 0 || at == 0 {
            return &[];
        }
        // UNSAFE(slice): a kept table's entries, read only.
        #[allow(unsafe_code)]
        // SAFETY: `keep` published `at` (a mapping a `Table<T, _>` made, aligned for `T`, `len` <= its entry
        // count, every entry written) before `len` (Release, read here with Acquire); the table was consumed
        // without drop, so the mapping is never unmapped and no reference writes it again.
        unsafe {
            core::slice::from_raw_parts(core::ptr::with_exposed_provenance::<T>(at), len)
        }
    }
}

/// The paths objects were found by, kept for their `$ORIGIN`: chunks of address space mapped as
/// the paths arrive (none for a start that searches nothing), never moved or released: a kept path
/// stays valid for the process (the start-time link maps' names, DL-23).
pub(super) struct PathStore {
    free: &'static mut [u8],
    chunks: Table<(usize, usize), 4>,
    n: usize,
}

impl Default for PathStore {
    fn default() -> Self {
        Self { free: &mut [], chunks: Table::new((0, 0)), n: 0 }
    }
}

impl PathStore {
    /// Address space one chunk takes (touched only as far as written): four full paths, or a few
    /// hundred ordinary ones; a longer path gets a chunk of its own size.
    const CHUNK: usize = 4 * PATH_MAX;

    /// A copy of `p` in the store, a NUL after it (DL-23: a kept path is also a link map's name).
    ///
    /// # Errors
    ///
    /// [`NoMemory`] when a new chunk is needed and cannot be mapped.
    pub(super) fn keep(&mut self, p: &[u8]) -> Result<&'static [u8], NoMemory> {
        let with_nul = p.len().saturating_add(1);
        if with_nul > self.free.len() {
            let len = Self::CHUNK.max(with_nul);
            self.chunks.reserve(self.n.saturating_add(1))?;
            let at = raw::mmap(0, len, map::PROT_READ | map::PROT_WRITE, MAP_PRIVATE_I32 | MAP_ANONYMOUS_I32, -1, 0).map_err(|_| NoMemory)?;
            if let Some(c) = self.chunks.get_mut(self.n) {
                *c = (at, len);
            }
            self.n = self.n.saturating_add(1);
            // UNSAFE(slice): a fresh anonymous mapping, owned by the store.
            #[allow(unsafe_code)]
            // SAFETY: `len` bytes from `at` were just mapped private, readable and writable; nothing else refers
            // to them, and the store never unmaps them (a kept path lives as long as the process).
            let fresh = unsafe { conv::bytes_at_mut(at, len) };
            self.free = fresh;
        }
        let (kept, rest) = core::mem::take(&mut self.free).split_at_mut(with_nul);
        let (kept, nul) = kept.split_at_mut(p.len());
        kept.copy_from_slice(p);
        nul.fill(0);
        self.free = rest;
        Ok(kept)
    }
}

/// The entries an [`Arena`] holds: plain integers, for which every bit pattern (a fresh mapping's
/// zeros included) is a valid value.
pub(super) trait Plain: Copy {}
impl Plain for u8 {}
impl Plain for u16 {}
impl Plain for u32 {}

/// An arena of spans that grows by generations (DL-PROCESS-OBJECTS, audit HL-5): generation 0 is the
/// `N` entries in place (in the loader's mapping); generation `g >= 1` holds `N << (g - 1)` entries
/// in a private anonymous mapping of its own, made at its first use and never moved or unmapped, so
/// an address handed out (a path's `l_name`, `dlpi_name`) stays valid for readers that hold no lock
/// ([`Arena::addr_in`]). Offsets are virtual: generation `g` covers `[N << (g - 1), N << g)`, and a
/// span never crosses a generation's end. Offsets fit 32 bits (`N << G <= 2^32`). All-zero is the
/// empty arena (the zeroed loader holds it). Only the lock holder writes; it writes only at and past
/// the top, which no reader reads (a span is published after it is written; the top is lowered only
/// over spans no reader can reach any more).
#[repr(C)]
pub(super) struct Arena<T: Plain, const N: usize, const G: usize> {
    /// Generation `g`'s address at `g - 1` (0: not mapped yet), stored with Release before any span
    /// in it is published. First, so [`Arena::addr_in`] finds it by address.
    gens: [AtomicUsize; G],
    /// The first offset past every span written.
    top: usize,
    /// Where the span being written began ([`Arena::open`]): [`Arena::push`] stays in its generation.
    open_at: usize,
    /// Generation 0, after the words above ([`Arena::INLINE_AT`]).
    inline: [T; N],
}

// `INLINE_AT` is the `#[repr(C)]` offset of generation 0 (checked for both entry sizes in use).
const _: () = assert!(core::mem::offset_of!(Arena<u8, 4096, 4>, inline) == Arena::<u8, 4096, 4>::INLINE_AT);
const _: () = assert!(core::mem::offset_of!(Arena<u16, 4096, 3>, inline) == Arena::<u16, 4096, 3>::INLINE_AT);

impl<T: Plain, const N: usize, const G: usize> Arena<T, N, G> {
    /// Generation 0's offset in the arena: the `G` directory words, the top and the span start.
    const INLINE_AT: usize = G.wrapping_add(2).wrapping_mul(size_of::<usize>());
    /// Power-of-two generations whose offsets fit 32 bits, entries no more aligned than a word.
    const FITS: () = assert!(N.is_power_of_two() && G >= 1 && G < 32 && N <= (1usize << 32) >> G && size_of::<T>() > 0 && align_of::<T>() <= align_of::<usize>());

    /// The generation and index of offset `off`.
    fn gen_of(off: usize) -> (usize, usize) {
        let q = off.checked_div(N).unwrap_or(0);
        if q == 0 {
            return (0, off);
        }
        let g = usize::try_from(usize::BITS.wrapping_sub(q.leading_zeros())).unwrap_or(usize::MAX);
        (g, off.wrapping_sub(Self::gen_start(g)))
    }

    /// The first offset of generation `g` (for `g >= 1`, also its entry count).
    fn gen_start(g: usize) -> usize {
        if g == 0 {
            return 0;
        }
        u32::try_from(g.wrapping_sub(1)).ok().and_then(|s| N.checked_shl(s)).unwrap_or(usize::MAX)
    }

    /// Entries of generation `g`.
    fn gen_len(g: usize) -> usize {
        if g == 0 { N } else { Self::gen_start(g) }
    }

    /// Generation `g >= 1`'s first entry's address (0: not mapped).
    fn base(&self, g: usize) -> usize {
        self.gens.get(g.wrapping_sub(1)).map_or(0, |w| w.load(Relaxed))
    }

    /// The first offset past every span written.
    pub(super) fn top(&self) -> usize {
        self.top
    }

    /// Forget the spans at and past `top` (an attempt unwound, freed slots at the table's end): later
    /// spans write that room again; the generations stay mapped.
    pub(super) fn set_top(&mut self, top: usize) {
        (self.top, self.open_at) = (top, top);
    }

    /// Begin a span of at most `most` entries ([`Arena::push`]): at the top when the rest of its
    /// generation has the room, else at the start of the first later generation that does, mapped
    /// now. Its offset, or `None` (nothing changed) when that mapping cannot be made or the offsets
    /// would pass 32 bits.
    pub(super) fn open(&mut self, most: usize) -> Option<usize> {
        let () = Self::FITS;
        self.open_at = self.top;
        let mut s = self.top;
        loop {
            let (g, i) = Self::gen_of(s);
            if g > G {
                return None;
            }
            if i.checked_add(most)? <= Self::gen_len(g) {
                if g > 0 && most > 0 && self.base(g) == 0 {
                    self.map(g)?;
                }
                (self.top, self.open_at) = (s, s);
                return Some(s);
            }
            s = Self::gen_start(g.wrapping_add(1));
        }
    }

    /// Map generation `g` (`1..=G`) and publish its address.
    fn map(&self, g: usize) -> Option<()> {
        let word = self.gens.get(g.wrapping_sub(1))?;
        let bytes = Self::gen_len(g).checked_mul(size_of::<T>())?.checked_next_multiple_of(4096)?;
        let at = raw::mmap(0, bytes, map::PROT_READ | map::PROT_WRITE, MAP_PRIVATE_I32 | MAP_ANONYMOUS_I32, -1, 0).ok()?;
        word.store(at, Release);
        Some(())
    }

    /// Append `v` to the span [`Arena::open`] began; `false` (nothing written) past the room it
    /// found.
    pub(super) fn push(&mut self, v: T) -> bool {
        let (g, i) = Self::gen_of(self.top);
        if g != Self::gen_of(self.open_at).0 {
            return false;
        }
        if g == 0 {
            let Some(slot) = self.inline.get_mut(i) else { return false };
            *slot = v;
        } else {
            let at = self.base(g);
            if at == 0 || g > G {
                return false;
            }
            let dst = core::ptr::with_exposed_provenance_mut::<T>(at).wrapping_add(i);
            // UNSAFE(slice): one entry of a mapped generation, at the top.
            #[allow(unsafe_code)]
            // SAFETY: `at` is generation `g`'s private read-write mapping of `gen_len(g)` entries or
            // more (made by `map`, page aligned, never unmapped) and `i < gen_len(g)` (`gen_of`), so
            // `dst` is an aligned entry inside it. It is at the top, which no reader reads, and
            // `&mut self` excludes every other writer and every slice `get` handed out.
            unsafe {
                dst.write(v);
            }
        }
        self.top = self.top.wrapping_add(1);
        true
    }

    /// Write `s` and then `end` as one span: its offset, or `None` (nothing written) when no
    /// generation can hold it.
    pub(super) fn append(&mut self, s: &[T], end: T) -> Option<usize> {
        let n = s.len().checked_add(1)?;
        let off = self.open(n)?;
        let (g, i) = Self::gen_of(off);
        if g == 0 {
            let (body, tail) = self.inline.get_mut(i..i.checked_add(n)?)?.split_at_mut(s.len());
            body.copy_from_slice(s);
            tail.fill(end);
        } else {
            let dst = core::ptr::with_exposed_provenance_mut::<T>(self.base(g)).wrapping_add(i);
            // UNSAFE(slice): the room `open` found in a mapped generation, at the top.
            #[allow(unsafe_code)]
            // SAFETY: `open` mapped generation `g` and found `n` entries of room from its entry `i`, so
            // `dst` and the `n` entries from it are aligned and inside the mapping (page aligned,
            // never unmapped). They are at and past the top, which no reader reads; `copy` allows `s`
            // anywhere, even in this arena.
            unsafe {
                core::ptr::copy(s.as_ptr(), dst, s.len());
            }
            // UNSAFE(slice): the last entry of that room.
            #[allow(unsafe_code)]
            // SAFETY: entry `s.len() < n` of the room above: aligned, inside the mapping, read by no
            // reader.
            unsafe {
                dst.wrapping_add(s.len()).write(end);
            }
        }
        self.top = off.wrapping_add(n);
        Some(off)
    }

    /// The `len` entries at `off` (a written span), or `None` when they are not all inside one
    /// mapped generation.
    pub(super) fn get(&self, off: usize, len: usize) -> Option<&[T]> {
        let (g, i) = Self::gen_of(off);
        if g == 0 {
            return self.inline.get(i..i.checked_add(len)?);
        }
        let at = self.base(g);
        if at == 0 || g > G || i.checked_add(len)? > Self::gen_len(g) {
            return None;
        }
        // UNSAFE(slice): a span inside one mapped generation.
        #[allow(unsafe_code)]
        // SAFETY: `at` is generation `g`'s mapping of `gen_len(g)` entries (made by `map`, page
        // aligned, never unmapped) and the `len` entries from `i` lie inside it; every bit pattern is
        // a valid `T` (`Plain`). The borrow of `self` keeps the only writer (`&mut self`) away while
        // the slice lives.
        Some(unsafe { core::slice::from_raw_parts(core::ptr::with_exposed_provenance::<T>(at).wrapping_add(i), len) })
    }

    /// The address of entry `off` (a span's first entry, for its readers), 0 when its generation is
    /// not mapped.
    pub(super) fn addr(&self, off: usize) -> usize {
        let (g, i) = Self::gen_of(off);
        let at = if g == 0 { self.inline.as_ptr().expose_provenance() } else { self.base(g) };
        if at == 0 { 0 } else { at.wrapping_add(i.wrapping_mul(size_of::<T>())) }
    }

    /// [`Arena::addr`] of the arena at address `arena`, read without a lock and without a reference
    /// to it (a reader that may run while the writer holds `&mut`): generation 0's place, else the
    /// generation's directory word, loaded with Acquire. 0 when that generation is not mapped.
    pub(super) fn addr_in(arena: usize, off: usize) -> usize {
        let (g, i) = Self::gen_of(off);
        let at = if g == 0 {
            arena.wrapping_add(Self::INLINE_AT)
        } else if g <= G {
            let w = arena.wrapping_add(g.wrapping_sub(1).wrapping_mul(size_of::<usize>()));
            // UNSAFE(slice): one directory word of an arena, by address.
            #[allow(unsafe_code)]
            // SAFETY: `arena` is a live `Arena`'s address (a field of the loader's mapping, never
            // unmapped) and `g - 1 < G`, so `w` is the aligned address of `gens[g - 1]` (the first
            // field, `#[repr(C)]`); an atomic may be loaded while the writer stores it.
            unsafe { (*core::ptr::with_exposed_provenance::<AtomicUsize>(w)).load(Acquire) }
        } else {
            0
        };
        if at == 0 { 0 } else { at.wrapping_add(i.wrapping_mul(size_of::<T>())) }
    }
}

/// Entries for which all-zero (a fresh mapping) and every value written are valid: what [`Grown`],
/// [`Wide`] and [`Spill`] hand out before or after a write.
pub(super) trait ZeroValid: Copy {}
impl ZeroValid for bool {}
impl ZeroValid for u8 {}
impl ZeroValid for u32 {}
impl ZeroValid for u64 {}
impl ZeroValid for usize {}
impl<A: ZeroValid, B: ZeroValid> ZeroValid for (A, B) {}
impl<T: ZeroValid, const N: usize> ZeroValid for [T; N] {}
impl<T: Copy> ZeroValid for MaybeUninit<T> {}
impl ZeroValid for niji_ld::init::ExitWalk {}

/// A private read-write anonymous mapping of `bytes` (rounded up to pages); 0 when it cannot be made.
fn map_zeroed(bytes: usize) -> usize {
    let Some(bytes) = bytes.checked_next_multiple_of(4096) else { return 0 };
    raw::mmap(0, bytes, map::PROT_READ | map::PROT_WRITE, MAP_PRIVATE_I32 | MAP_ANONYMOUS_I32, -1, 0).unwrap_or(0)
}

/// The directory of a table of fixed-size entries that grows by generations (DL-PROCESS-OBJECTS
/// part 2, audit HL-3), whose generation 0 (`N` entries) lives elsewhere: `ld.so`'s data (the
/// link-map nodes), a static (the readers' table) or a [`Grown`]'s place. Generation `g >= 1` holds
/// `N << (g - 1)` entries in a private anonymous mapping made by [`Gens::reserve`], never moved or
/// unmapped, with [`Arena`]'s generation arithmetic: an entry's address (a handle, a link map)
/// stays valid for the process, and a reader that holds no lock decodes an index past generation 0
/// with one directory load (Acquire). The directory word is stored (Release) before any entry in
/// the generation is published. Indices fit 32 bits (`N << G <= 2^32`). All-zero is the empty
/// directory.
#[repr(C)]
pub(super) struct Gens<const N: usize, const G: usize> {
    gens: [AtomicUsize; G],
}

impl<const N: usize, const G: usize> Gens<N, G> {
    /// Power-of-two generations whose indices fit 32 bits.
    const FITS: () = assert!(N.is_power_of_two() && G >= 1 && G < 32 && N <= (1usize << 32) >> G);

    pub(super) const fn new() -> Self {
        Gens { gens: [const { AtomicUsize::new(0) }; G] }
    }

    /// Entries generations `0..=G` hold together: one more index than any.
    pub(super) const MOST: usize = N << G;

    /// Entry `i`'s address: `zero` (generation 0's first entry) plus its offset there, else from
    /// generation `g`'s address `base(g)`; 0 when that generation is not mapped or `i` is past every
    /// generation.
    fn locate(i: usize, size: usize, zero: usize, base: impl FnOnce(usize) -> usize) -> usize {
        let () = Self::FITS;
        let (g, j) = Arena::<u8, N, G>::gen_of(i);
        let at = if g == 0 {
            zero
        } else if g <= G {
            base(g)
        } else {
            0
        };
        if at == 0 { 0 } else { at.wrapping_add(j.wrapping_mul(size)) }
    }

    /// Generation `g >= 1`'s address in the directory at address `dir`, loaded without a reference
    /// (Acquire); 0 when not mapped.
    fn base_in(dir: usize, g: usize) -> usize {
        if g == 0 || g > G {
            return 0;
        }
        let w = dir.wrapping_add(g.wrapping_sub(1).wrapping_mul(size_of::<usize>()));
        // UNSAFE(slice): one directory word, by address.
        #[allow(unsafe_code)]
        // SAFETY: `dir` is a live `Gens`'s address (a field of the loader's mapping, never unmapped,
        // or a reference's) and `g - 1 < G`, so `w` is the aligned address of `gens[g - 1]`
        // (`#[repr(C)]`, the only field); an atomic may be loaded while the writer stores it.
        unsafe { (*core::ptr::with_exposed_provenance::<AtomicUsize>(w)).load(Acquire) }
    }

    /// Entry `i`'s address for the directory at address `dir` (a reader that may run while the
    /// writer holds `&mut` of what holds the directory), entries of `size` bytes, generation 0 at
    /// `zero`; 0 when not mapped.
    pub(super) fn at_in(dir: usize, zero: usize, size: usize, i: usize) -> usize {
        Self::locate(i, size, zero, |g| Self::base_in(dir, g))
    }

    /// [`Gens::at_in`] through a reference.
    pub(super) fn at(&self, zero: usize, size: usize, i: usize) -> usize {
        Self::at_in(core::ptr::from_ref(self).expose_provenance(), zero, size, i)
    }

    /// The index of the entry whose first byte is `addr` (entries of `size` bytes, generation 0 at
    /// `zero`), for the directory at `dir`: one range test for generation 0, then one per mapped
    /// generation (mapped in order, so the first unmapped one ends the search).
    pub(super) fn index_in(dir: usize, zero: usize, size: usize, addr: usize) -> Option<usize> {
        let () = Self::FITS;
        for g in 0..=G {
            let base = if g == 0 { zero } else { Self::base_in(dir, g) };
            if base == 0 {
                return None;
            }
            let len = Arena::<u8, N, G>::gen_len(g);
            if let Some(off) = addr.checked_sub(base).filter(|&o| o < len.saturating_mul(size)) {
                return (off.checked_rem(size)? == 0).then(|| Arena::<u8, N, G>::gen_start(g).wrapping_add(off / size));
            }
        }
        None
    }

    /// [`Gens::index_in`] through a reference.
    pub(super) fn index(&self, zero: usize, size: usize, addr: usize) -> Option<usize> {
        Self::index_in(core::ptr::from_ref(self).expose_provenance(), zero, size, addr)
    }

    /// Map every generation entries `0..len` need (entries of `size` bytes), publishing each
    /// address before it returns; `false` when a mapping cannot be made or `len` passes every
    /// generation (the ones made stay, as all generations do).
    pub(super) fn reserve(&self, size: usize, len: usize) -> bool {
        let () = Self::FITS;
        for g in 1..=G {
            if Arena::<u8, N, G>::gen_start(g) >= len {
                return true;
            }
            let Some(word) = self.gens.get(g.wrapping_sub(1)) else { return false };
            if word.load(Relaxed) == 0 {
                let at = Arena::<u8, N, G>::gen_len(g).checked_mul(size).map_or(0, map_zeroed);
                if at == 0 {
                    return false;
                }
                word.store(at, Release);
            }
        }
        Arena::<u8, N, G>::gen_start(G.wrapping_add(1)) >= len
    }
}

/// A table of `T` that grows by never-moved generations ([`Gens`]) with its generation 0 in place
/// (`N` entries, after the directory): a process within `N` entries uses the same bytes as a fixed
/// array of `N`. Only the lock holder writes; a reader that holds no lock reads an entry published
/// to it by address ([`Grown::addr_in`]).
#[repr(C)]
pub(super) struct Grown<T: ZeroValid, const N: usize, const G: usize> {
    gens: Gens<N, G>,
    inline: [T; N],
}

impl<T: ZeroValid, const N: usize, const G: usize> Grown<T, N, G> {
    /// Generation 0's offset (after the `G` directory words; entries no more aligned than a word).
    pub(super) const INLINE_AT: usize = G.wrapping_mul(size_of::<usize>());
    const ALIGNED: () = assert!(align_of::<T>() <= align_of::<usize>() && size_of::<T>() > 0);

    fn zero(&self) -> usize {
        self.inline.as_ptr().expose_provenance()
    }

    /// Entry `i`'s address (0: its generation is not mapped).
    pub(super) fn addr(&self, i: usize) -> usize {
        let () = Self::ALIGNED;
        self.gens.at(self.zero(), size_of::<T>(), i)
    }

    /// Entry `i`'s address in the table at address `at`, read without a reference to it.
    pub(super) fn addr_in(at: usize, i: usize) -> usize {
        let () = Self::ALIGNED;
        Gens::<N, G>::at_in(at, at.wrapping_add(Self::INLINE_AT), size_of::<T>(), i)
    }

    /// Room for entries `0..len`; `false` when a generation cannot be mapped.
    pub(super) fn reserve(&self, len: usize) -> bool {
        len <= N || self.gens.reserve(size_of::<T>(), len)
    }

    pub(super) fn get(&self, i: usize) -> Option<&T> {
        if i < N {
            return self.inline.get(i);
        }
        let at = self.addr(i);
        if at == 0 {
            return None;
        }
        // UNSAFE(slice): one entry of a mapped generation.
        #[allow(unsafe_code)]
        // SAFETY: `at` is entry `i` of a generation `reserve` mapped (read-write, page aligned, never
        // unmapped), aligned for `T` (`ALIGNED`); all-zero and every value written are valid `T`s
        // (`ZeroValid`); the borrow of `self` keeps the only writer (`&mut self`) away.
        Some(unsafe { &*core::ptr::with_exposed_provenance::<T>(at) })
    }

    pub(super) fn get_mut(&mut self, i: usize) -> Option<&mut T> {
        if i < N {
            return self.inline.get_mut(i);
        }
        let at = self.addr(i);
        if at == 0 {
            return None;
        }
        // UNSAFE(slice): one entry of a mapped generation, for its one writer.
        #[allow(unsafe_code)]
        // SAFETY: as in `get`; `&mut self` excludes every other reference made through the table, and
        // readers without a lock read only entries published to them, which the writer does not
        // change until their slot is reused after a grace period.
        Some(unsafe { &mut *core::ptr::with_exposed_provenance_mut::<T>(at) })
    }

    /// Generation 0 (the fixed array it replaced).
    pub(super) fn inline(&self) -> &[T; N] {
        &self.inline
    }
}

/// Grow the mapping at `at` (`old` bytes, 0: none) to `new` bytes, moving it (`mremap`) when it must:
/// its new address, 0 when that cannot be done (the old mapping is kept).
fn grow_map(at: usize, old: usize, new: usize) -> usize {
    let Some(new) = new.checked_next_multiple_of(4096) else { return 0 };
    if at == 0 {
        return map_zeroed(new);
    }
    raw::mremap(at, old, new).unwrap_or(0)
}

/// A contiguous lock-held buffer of `T` in a mapping of its own, moved and grown by `mremap`
/// (DL-PROCESS-OBJECTS part 2): scratch that the `dlopen` growing the object count reserves, so the
/// calls that must not allocate (`dlclose`, the exit) find it sized. All-zero is the empty buffer.
pub(super) struct Wide<T: ZeroValid> {
    at: usize,
    cap: usize,
    _t: PhantomData<T>,
}

impl<T: ZeroValid> Wide<T> {
    pub(super) const fn new() -> Self {
        Wide { at: 0, cap: 0, _t: PhantomData }
    }

    /// Room for `len` entries (zero when never written); `false` when the mapping cannot grow.
    pub(super) fn reserve(&mut self, len: usize) -> bool {
        if len <= self.cap {
            return true;
        }
        let size = size_of::<T>().max(1);
        let want = len.max(self.cap.saturating_mul(2));
        let Some(bytes) = want.checked_mul(size) else { return false };
        let at = grow_map(self.at, self.cap.saturating_mul(size).next_multiple_of(4096), bytes);
        if at == 0 {
            return false;
        }
        (self.at, self.cap) = (at, bytes.next_multiple_of(4096) / size);
        true
    }

    /// Entries `0..len`, `None` past the room reserved.
    pub(super) fn slice_mut(&mut self, len: usize) -> Option<&mut [T]> {
        if len > self.cap || self.at == 0 {
            return if len == 0 { Some(&mut []) } else { None };
        }
        // UNSAFE(slice): the reserved prefix of the buffer's own mapping.
        #[allow(unsafe_code)]
        // SAFETY: `at` is a private read-write mapping of `cap >= len` entries (page aligned, so
        // aligned for `T`), made or moved by `reserve` only, which `&mut self` keeps from running
        // while the slice lives; all-zero and every value written are valid `T`s (`ZeroValid`).
        Some(unsafe { core::slice::from_raw_parts_mut(core::ptr::with_exposed_provenance_mut::<T>(self.at), len) })
    }
}

/// Contiguous lock-held copies of records (not `ZeroValid`: they hold references) in a mapping of
/// their own, moved and grown by `mremap`: `0..len` are written. The relocator's and the lookups'
/// slices of the objects past generation 0 ([`Grown`] holds the published ones).
pub(super) struct Records<T: Copy> {
    at: usize,
    cap: usize,
    len: usize,
    _t: PhantomData<T>,
}

impl<T: Copy> Records<T> {
    /// Room for `len` records; `false` when the mapping cannot grow.
    pub(super) fn reserve(&mut self, len: usize) -> bool {
        if len <= self.cap {
            return true;
        }
        let size = size_of::<T>().max(1);
        let want = len.max(self.cap.saturating_mul(2));
        let Some(bytes) = want.checked_mul(size) else { return false };
        let at = grow_map(self.at, self.cap.saturating_mul(size).next_multiple_of(4096), bytes);
        if at == 0 {
            return false;
        }
        (self.at, self.cap) = (at, bytes.next_multiple_of(4096) / size);
        true
    }

    /// Records written (`0..len`).
    pub(super) fn len(&self) -> usize {
        self.len
    }

    /// Write record `i` (at most one past the written ones, inside the room reserved); `false`
    /// (nothing written) otherwise.
    pub(super) fn set(&mut self, i: usize, v: T) -> bool {
        if i > self.len || i >= self.cap || self.at == 0 {
            return false;
        }
        // UNSAFE(slice): one record of the buffer's own mapping.
        #[allow(unsafe_code)]
        // SAFETY: `i < cap`: inside the private read-write mapping `reserve` made (page aligned, so
        // aligned for `T`); `&mut self` excludes every slice `slice` handed out; a plain write of a
        // `Copy` value, so the old bytes need not be a valid `T`.
        unsafe {
            core::ptr::with_exposed_provenance_mut::<T>(self.at).wrapping_add(i).write(v);
        }
        if i == self.len {
            self.len = self.len.wrapping_add(1);
        }
        true
    }

    /// Records `0..k`, `None` unless all are written.
    pub(super) fn slice(&self, k: usize) -> Option<&[T]> {
        if k > self.len || self.at == 0 {
            return if k == 0 { Some(&[]) } else { None };
        }
        // UNSAFE(slice): the written prefix of the buffer's own mapping.
        #[allow(unsafe_code)]
        // SAFETY: records `0..len` were written by `set` as valid `T`s, inside the mapping (`len <=
        // cap`); the borrow of `self` keeps `reserve` (the only mover) and `set` away.
        Some(unsafe { core::slice::from_raw_parts(core::ptr::with_exposed_provenance::<T>(self.at), k) })
    }
}

/// A lock-held list with its first `N` entries in place (the fixed array it replaced: a process
/// within `N` uses the same bytes) that moves, whole, to a [`Wide`] buffer the first time it needs
/// more (the in-place entries are then no longer read). All-zero is the empty list.
pub(super) struct Spill<T: ZeroValid, const N: usize> {
    inline: [T; N],
    wide: Wide<T>,
}

impl<T: ZeroValid, const N: usize> Spill<T, N> {
    /// Room for entries `0..len`; `false` when the buffer cannot be made or grow.
    pub(super) fn reserve(&mut self, len: usize) -> bool {
        if len <= N && self.wide.cap == 0 {
            return true;
        }
        let first = self.wide.cap == 0;
        if !self.wide.reserve(len.max(N.saturating_mul(2))) {
            return false;
        }
        if first {
            if let Some(w) = self.wide.slice_mut(N) {
                w.copy_from_slice(&self.inline);
            }
        }
        true
    }

    /// Entries `0..len` (`None` past the room reserved).
    pub(super) fn slice(&self, len: usize) -> Option<&[T]> {
        if self.wide.cap == 0 {
            return self.inline.get(..len);
        }
        if len > self.wide.cap {
            return None;
        }
        // UNSAFE(slice): the reserved prefix of the list's buffer.
        #[allow(unsafe_code)]
        // SAFETY: as `Wide::slice_mut`, shared: the borrow of `self` keeps `reserve` away.
        Some(unsafe { core::slice::from_raw_parts(core::ptr::with_exposed_provenance::<T>(self.wide.at), len) })
    }

    pub(super) fn slice_mut(&mut self, len: usize) -> Option<&mut [T]> {
        if self.wide.cap == 0 {
            return self.inline.get_mut(..len);
        }
        self.wide.slice_mut(len)
    }

    pub(super) fn get(&self, i: usize) -> Option<&T> {
        self.slice(i.checked_add(1)?)?.last()
    }

    pub(super) fn get_mut(&mut self, i: usize) -> Option<&mut T> {
        self.slice_mut(i.checked_add(1)?)?.last_mut()
    }
}
