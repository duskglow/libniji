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
pub(super) const INLINE: usize = 64;

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
/// the paths arrive (none for a start that searches nothing), never moved, so a kept path stays
/// valid until [`PathStore::release`].
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

    /// A copy of `p` in the store.
    ///
    /// # Errors
    ///
    /// [`NoMemory`] when a new chunk is needed and cannot be mapped.
    pub(super) fn keep(&mut self, p: &[u8]) -> Result<&'static [u8], NoMemory> {
        if p.len() > self.free.len() {
            let len = Self::CHUNK.max(p.len());
            self.chunks.reserve(self.n.saturating_add(1))?;
            let at = raw::mmap(0, len, map::PROT_READ | map::PROT_WRITE, MAP_PRIVATE_I32 | MAP_ANONYMOUS_I32, -1, 0).map_err(|_| NoMemory)?;
            if let Some(c) = self.chunks.get_mut(self.n) {
                *c = (at, len);
            }
            self.n = self.n.saturating_add(1);
            // UNSAFE(slice): a fresh anonymous mapping, owned by the store.
            #[allow(unsafe_code)]
            // SAFETY: `len` bytes from `at` were just mapped private, readable and writable; nothing else refers
            // to them, and the store unmaps them only in `release`, after the caller's last use of a kept path.
            let fresh = unsafe { conv::bytes_at_mut(at, len) };
            self.free = fresh;
        }
        let (kept, rest) = core::mem::take(&mut self.free).split_at_mut(p.len());
        kept.copy_from_slice(p);
        self.free = rest;
        Ok(kept)
    }

    /// Unmap every chunk: no kept path is used after this.
    pub(super) fn release(&mut self) {
        for &(at, len) in self.chunks.get(..self.n).unwrap_or(&[]) {
            let _ = raw::munmap(at, len);
        }
        (self.free, self.n) = (&mut [], 0);
    }
}
