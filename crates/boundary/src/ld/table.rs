//! Tables that grow with the number of objects (DL-STARTUP-OBJECTS, GitHub issue #9): the loader's
//! start-time tables (load list, dependency links, search chains, init and fini order, TLS modules,
//! the records `dlopen` and the exit read) hold their first `N` entries inline, so a program with a
//! handful of objects maps nothing new, and spill to a private anonymous mapping beyond that,
//! doubling. The loader has no heap (its allocator is `mmap`); the only failure is the mapping's,
//! reported by the caller as an allocation failure. No count of objects is refused.

use core::marker::PhantomData;
use core::mem::{align_of, size_of, ManuallyDrop};
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

/// A table of `T`, `N` entries inline, grown on demand ([`Table::reserve`]); every entry, beyond
/// the ones written, holds the table's fill value. Dereferences to all its entries.
pub(super) struct Table<T: Copy, const N: usize> {
    inline: [T; N],
    /// The mapping holding the entries once they outgrow `inline` (0: none) and its entry count.
    heap: usize,
    cap: usize,
    fill: T,
}

impl<T: Copy, const N: usize> Table<T, N> {
    /// A mapping is page-aligned and holds at least one entry.
    const FITS: () = assert!(size_of::<T>() > 0 && align_of::<T>() <= 4096);

    /// `N` entries of `fill`, nothing mapped.
    pub(super) fn new(fill: T) -> Self {
        let () = Self::FITS;
        Self { inline: [fill; N], heap: 0, cap: 0, fill }
    }

    /// Room for at least `len` entries: the entries so far are kept, the new ones hold the fill.
    ///
    /// # Errors
    ///
    /// [`NoMemory`] when the mapping fails; the table is unchanged.
    pub(super) fn reserve(&mut self, len: usize) -> Result<(), NoMemory> {
        let have = self.len();
        if len <= have {
            return Ok(());
        }
        let cap = len.max(have.saturating_mul(2));
        let bytes = cap.checked_mul(size_of::<T>()).ok_or(NoMemory)?;
        let at = raw::mmap(0, bytes, map::PROT_READ | map::PROT_WRITE, MAP_PRIVATE_I32 | MAP_ANONYMOUS_I32, -1, 0).map_err(|_| NoMemory)?;
        let p = core::ptr::with_exposed_provenance_mut::<T>(at);
        for i in 0..cap {
            let v = self.get(i).copied().unwrap_or(self.fill);
            // UNSAFE(slice): one entry of a table's new mapping, written before the mapping is used.
            #[allow(unsafe_code)]
            // SAFETY: `at` is a fresh private read-write mapping of `cap` entries (page-aligned, so aligned for
            // `T`: FITS); `i < cap`; nothing else refers to it yet.
            unsafe {
                p.wrapping_add(i).write(v);
            }
        }
        self.release();
        (self.heap, self.cap) = (at, cap);
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
        if self.heap == 0 {
            return &self.inline;
        }
        // UNSAFE(slice): a table's entries in its own mapping.
        #[allow(unsafe_code)]
        // SAFETY: `heap` is the private mapping `reserve` made for this table, aligned for `T`, its `cap` entries
        // all written there; only this table refers to it, and it stays mapped until `release` (by `reserve`
        // after its copy, or drop), which needs `&mut self`, so not while this borrow lives.
        unsafe {
            core::slice::from_raw_parts(core::ptr::with_exposed_provenance::<T>(self.heap), self.cap)
        }
    }
}

impl<T: Copy, const N: usize> DerefMut for Table<T, N> {
    fn deref_mut(&mut self) -> &mut [T] {
        if self.heap == 0 {
            return &mut self.inline;
        }
        // UNSAFE(slice): a table's entries in its own mapping, borrowed uniquely.
        #[allow(unsafe_code)]
        // SAFETY: as for `deref`; the borrow of `self` is unique, so is the slice.
        unsafe {
            core::slice::from_raw_parts_mut(core::ptr::with_exposed_provenance_mut::<T>(self.heap), self.cap)
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
        if t.heap == 0 {
            t.reserve(N.saturating_add(1).max(len))?;
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
    /// Address space one chunk takes: [`INLINE`] full paths (touched only as far as written).
    const CHUNK: usize = INLINE * PATH_MAX;

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
