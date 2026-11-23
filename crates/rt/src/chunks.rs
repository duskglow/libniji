//! A table that grows with use in chunks that never move (LIMITS-RUNTIME-SMALL; D-1.2, D-2.6,
//! D-16.8): the library's registries that have no limit but memory (exit handlers, added
//! severities, `getaddrinfo_a`'s queue and notification groups).
//!
//! Entries are `w` words each. The first chunk is one page, each later one twice the one before,
//! and a chunk is mapped only when an entry in it is first needed, so the memory mapped is never
//! more than about twice what is used (plus one page). An entry stays where it is for the life of
//! the process, so readers need no lock: a chunk's address is published (release) after the chunk
//! is mapped and read (acquire) before any of its words. Growth ([`Chunks::reserve`]) is
//! serialised by the owner. The directory holds [`DIR`] chunk addresses; together those chunks
//! span 2^60 bytes, more than any x86-64 address space (2^57 with five-level paging), so the
//! directory's length is no limit: running out of memory is the only failure.

use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering::{Acquire, Relaxed, Release}};

use crate::heap::{Pages, PAGE};

/// Chunk addresses in the directory: chunk `k` is `PAGE << k` bytes.
pub const DIR: usize = 48;

/// A table of `w`-word entries in chunks that never move.
pub struct Chunks {
    dir: [AtomicUsize; DIR],
}

impl Default for Chunks {
    fn default() -> Chunks {
        Chunks::new()
    }
}

impl Chunks {
    /// An empty table (nothing mapped).
    pub const fn new() -> Chunks {
        Chunks { dir: [const { AtomicUsize::new(0) }; DIR] }
    }

    /// The chunk holding entry `i` of `w` words: its number, the entry's offset in it, and the
    /// chunk's first entry and number of entries. `None` for `w` outside 1 to 512 or beyond the
    /// directory.
    pub fn place(i: usize, w: usize) -> Option<(usize, usize, usize, usize)> {
        let per_page = PAGE.checked_div(w.checked_mul(8)?).filter(|&b| b > 0)?;
        // Chunk k holds entries [b(2^k - 1), b(2^(k+1) - 1)): k is the base-2 log of i / b + 1.
        let q = i.checked_div(per_page)?.checked_add(1)?;
        let k = usize::try_from(q.ilog2()).ok().filter(|&k| k < DIR)?;
        let first = per_page.checked_mul(1usize.checked_shl(u32::try_from(k).ok()?)?.checked_sub(1)?)?;
        let count = per_page.checked_shl(u32::try_from(k).ok()?)?;
        Some((k, i.checked_sub(first)?, first, count))
    }

    /// Entry `i`'s `w` words; `None` while its chunk is not mapped. Needs no lock.
    pub fn get<'a, P: Pages>(&self, pages: &'a P, i: usize, w: usize) -> Option<&'a [AtomicU64]> {
        let (k, off, _, _) = Self::place(i, w)?;
        match self.dir.get(k)?.load(Acquire) {
            0 => None,
            base => Some(pages.words(base.checked_add(off.checked_mul(w)?.checked_mul(8)?)?, w)),
        }
    }

    /// Make entry `i` (of `w` words) present, mapping its chunk if it is not yet; false without
    /// memory (nothing changes then). Callers serialise growth.
    pub fn reserve<P: Pages>(&self, pages: &P, i: usize, w: usize) -> bool {
        let Some((k, _, _, _)) = Self::place(i, w) else { return false };
        let Some(slot) = self.dir.get(k) else { return false };
        if slot.load(Relaxed) != 0 {
            return true;
        }
        let Some(len) = PAGE.checked_shl(u32::try_from(k).unwrap_or(u32::MAX)) else { return false };
        match pages.map(len, PAGE, None) {
            Some(base) => {
                slot.store(base, Release);
                true
            }
            None => false,
        }
    }

    /// Bytes mapped so far.
    pub fn mapped(&self) -> usize {
        self.dir.iter().enumerate().filter(|(_, c)| c.load(Relaxed) != 0).map(|(k, _)| PAGE.checked_shl(u32::try_from(k).unwrap_or(u32::MAX)).unwrap_or(0)).fold(0, usize::saturating_add)
    }

    /// Unmap every chunk, for an owner that goes away (a table in a test); no entry may be used
    /// afterwards.
    pub fn unmap_all<P: Pages>(&self, pages: &P) {
        for (k, c) in self.dir.iter().enumerate() {
            let base = c.swap(0, Relaxed);
            if base != 0 {
                pages.unmap(base, PAGE.checked_shl(u32::try_from(k).unwrap_or(u32::MAX)).unwrap_or(0));
            }
        }
    }
}
