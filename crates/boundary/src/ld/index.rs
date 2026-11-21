//! The loader's address index (DL-FIND-OBJECT-FAST, D-NEW in
//! `records/pending/DL-FIND-OBJECT-FAST-1.toml`): which published object holds an address, for
//! `_dl_find_object` ([`super::readers`]) and `dladdr` (`super::dlopen::dladdr`), in a binary search
//! instead of a walk over every object (the unwinders call `_dl_find_object` once per frame).
//!
//! * **Entries.** One per `PT_LOAD` segment of a published object, sorted by segment start:
//!   `[segment start, segment end, object low, object high, PT_GNU_EH_FRAME, object index]`, the
//!   object's range being its lowest to highest `PT_LOAD` address (what `_dl_find_object` reports
//!   and `dladdr` matches). Segments of one object that overlap are merged into one entry.
//! * **Flags.** [`OVERLAP`]: segments of two objects overlap (`_dl_find_object` falls back to the
//!   table walk); [`NESTED`]: the ranges of two objects overlap (`dladdr`, whose object is the first
//!   in chain order whose range holds the address, falls back to the chain walk). Neither happens
//!   when every object is mapped by its own reservation; both keep the walk's result exact when a
//!   malformed or unusual layout does it.
//! * **The start-time objects** (never unloaded) get an immutable index built by the first reader
//!   or `dlopen` in a private mapping and published by one compare-and-swap ([`START`]); racing
//!   builders build the same index and the losers unmap theirs. No lock, nothing at start-up.
//! * **After the first `dlopen`** the index of every published object lives in two copies (a
//!   latch: Boehm, "Can seqlocks get along with programming language memory models?", MSPC 2012,
//!   for the fences). Only the writer changes them, under the loader's lock ([`added`],
//!   [`removed`]): [`SEQ`] odd sends readers to copy 1 while copy 0 is rewritten, even sends them to
//!   copy 0 while copy 1 is rewritten. A reader picks the copy by [`SEQ`], searches it with atomic
//!   loads, and keeps the answer only when [`SEQ`] did not move meanwhile; so it never acts on a
//!   torn entry, and a reader that interrupts the writer (a signal handler) always reads a
//!   complete copy. After [`TRIES`] moved attempts it walks the table instead.
//! * **Unloading.** `dlclose` removes an object's entries when it unpublishes it, before the grace
//!   period whose end lets its slot be reused (DL-17): a reader that entered later cannot find it,
//!   and one that found it earlier is still counted, so the object stays mapped and its index
//!   means the same object while that reader runs.
//! * **Memory.** A copy that must grow moves to a new mapping twice its size; the old one stays
//!   mapped (a reader may still be searching it) and is never reused: at most twice the largest
//!   index per copy. A mapping that cannot be made disables the index for the process (the walks
//!   answer, as before).

use super::readers;
use crate::raw;
use core::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use core::sync::atomic::{fence, AtomicUsize};
use niji_ld::map;

/// Words per entry and per region header (`[entries, flags, capacity]`).
const W: usize = 6;
const H: usize = 3;
const PT_LOAD: u64 = 1;
const PT_GNU_EH_FRAME: u64 = 0x6474_e550;
/// Segments of two objects overlap.
const OVERLAP: usize = 1;
/// The ranges of two objects overlap.
const NESTED: usize = 2;
/// Validated attempts before a reader walks instead.
const TRIES: usize = 8;
/// Entries a copy starts with.
const MIN_CAP: usize = 64;

/// The start-time objects' index (a region address; 0: not built yet).
static START: AtomicUsize = AtomicUsize::new(0);
/// The latch's sequence (0: no `dlopen` has published yet; the start index answers).
static SEQ: AtomicUsize = AtomicUsize::new(0);
/// The two copies' regions.
static COPY: [AtomicUsize; 2] = [const { AtomicUsize::new(0) }; 2];
/// Nonzero once a mapping could not be made: the walks answer from then on.
static DISABLED: AtomicUsize = AtomicUsize::new(0);

/// The object holding an address: its range, `PT_GNU_EH_FRAME` and index.
#[derive(Clone, Copy)]
pub(super) struct Hit {
    pub(super) lo: usize,
    pub(super) hi: usize,
    pub(super) eh: usize,
    pub(super) i: usize,
}

/// A lookup's answer: the object, none, or the index cannot tell (the caller walks).
pub(super) enum Found {
    Hit(Hit),
    Miss,
    Unknown,
}

/// The words of the region at `at` (its header and capacity's entries).
fn region(at: usize) -> &'static [AtomicUsize] {
    // UNSAFE(slice): the header of an index region this module mapped.
    #[allow(unsafe_code)]
    // SAFETY: `at` is a region `alloc` mapped (read-write, never unmapped once published, see the
    // module documentation) whose header (H atomic words) was written before the Release store
    // that made it reachable; atomics are valid for any bit pattern.
    let head = unsafe { core::slice::from_raw_parts(core::ptr::with_exposed_provenance::<AtomicUsize>(at), H) };
    let cap = head.get(2).map_or(0, |w| w.load(Relaxed));
    // UNSAFE(slice): the whole region (header and `cap` entries).
    #[allow(unsafe_code)]
    // SAFETY: as above; `alloc` mapped H + cap * W words and wrote `cap`, which never changes.
    unsafe { core::slice::from_raw_parts(core::ptr::with_exposed_provenance::<AtomicUsize>(at), H.wrapping_add(cap.wrapping_mul(W))) }
}

/// Word `k` of entry `e`.
#[inline(always)]
fn word(r: &[AtomicUsize], e: usize, k: usize) -> usize {
    r.get(H.wrapping_add(e.wrapping_mul(W)).wrapping_add(k)).map_or(0, |w| w.load(Relaxed))
}

fn set(r: &[AtomicUsize], e: usize, k: usize, v: usize) {
    if let Some(w) = r.get(H.wrapping_add(e.wrapping_mul(W)).wrapping_add(k)) {
        w.store(v, Relaxed);
    }
}

fn entry(r: &[AtomicUsize], e: usize) -> [usize; W] {
    core::array::from_fn(|k| word(r, e, k))
}

fn put(r: &[AtomicUsize], e: usize, v: [usize; W]) {
    for (k, x) in v.into_iter().enumerate() {
        set(r, e, k, x);
    }
}

fn head(r: &[AtomicUsize], k: usize) -> usize {
    r.get(k).map_or(0, |w| w.load(Relaxed))
}

fn set_head(r: &[AtomicUsize], k: usize, v: usize) {
    if let Some(w) = r.get(k) {
        w.store(v, Relaxed);
    }
}

/// The object whose segment (`whole`: whose range) holds `a`. Called inside a read section.
pub(super) fn find(a: usize, whole: bool) -> Found {
    for _ in 0..TRIES {
        let s = SEQ.load(Acquire);
        if DISABLED.load(Relaxed) != 0 {
            return Found::Unknown;
        }
        let at = if s == 0 {
            match start() {
                Some(at) => at,
                None => return Found::Unknown,
            }
        } else {
            COPY.get(s & 1).map_or(0, |c| c.load(Acquire))
        };
        if at == 0 {
            return Found::Unknown;
        }
        let r = region(at);
        let answer = search(r, a, whole);
        if s == 0 {
            // The start index never changes once published.
            return answer;
        }
        fence(Acquire);
        if SEQ.load(Relaxed) == s {
            return answer;
        }
    }
    Found::Unknown
}

/// [`find`] in one region.
#[inline(always)]
fn search(r: &[AtomicUsize], a: usize, whole: bool) -> Found {
    let flags = head(r, 1);
    if flags & if whole { NESTED } else { OVERLAP } != 0 {
        return Found::Unknown;
    }
    // The last entry whose segment starts at or below `a`.
    let (mut lo, mut hi) = (0usize, head(r, 0).min(head(r, 2)));
    while lo < hi {
        let mid = lo.wrapping_add(hi.wrapping_sub(lo) / 2);
        if word(r, mid, 0) <= a {
            lo = mid.wrapping_add(1);
        } else {
            hi = mid;
        }
    }
    let Some(e) = lo.checked_sub(1) else { return Found::Miss };
    let [seg_lo, seg_hi, o_lo, o_hi, eh, i] = entry(r, e);
    let holds = if whole { o_lo <= a && a < o_hi } else { seg_lo <= a && a < seg_hi };
    if holds {
        Found::Hit(Hit { lo: o_lo, hi: o_hi, eh, i })
    } else {
        Found::Miss
    }
}

/// The start-time objects' index, built at the first call.
fn start() -> Option<usize> {
    let at = START.load(Acquire);
    if at != 0 {
        return Some(at);
    }
    build_start()
}

/// Build the start-time index in a private region and publish it unless another thread did.
#[cold]
#[inline(never)]
fn build_start() -> Option<usize> {
    let n = readers::start_count();
    let m = (0..n).map(segments).fold(0usize, usize::wrapping_add);
    let at = alloc(m.wrapping_mul(2))?;
    let r = region(at);
    let tail = head(r, 2).wrapping_sub(m);
    let filled = fill(r, tail, 0..n);
    sort(r, tail, filled);
    let mut out = Out::new(r);
    for e in tail..tail.wrapping_add(filled) {
        out.push(entry(r, e));
    }
    out.finish();
    match START.compare_exchange(0, at, Release, Acquire) {
        Ok(_) => Some(at),
        Err(other) => {
            // Never published: no reader can be reading it.
            let _ = raw::munmap(at, bytes(head(r, 2)));
            Some(other)
        }
    }
}

/// The number of `PT_LOAD` entries object `i` gives (0: not live).
fn segments(i: usize) -> usize {
    readers::live(i).map_or(0, |(_, phdr, phnum)| readers::headers(phdr, phnum).filter(|h| h.0 == PT_LOAD).count())
}

/// Write the entries of the objects `objs` (unsorted) at entries `from..` of `r`: how many.
fn fill(r: &[AtomicUsize], from: usize, objs: core::ops::Range<usize>) -> usize {
    let mut k = from;
    for i in objs {
        let Some((bias, phdr, phnum)) = readers::live(i) else { continue };
        let at = |v: u64| bias.wrapping_add(usize::try_from(v).unwrap_or(0));
        let loads = || readers::headers(phdr, phnum).filter(|h| h.0 == PT_LOAD);
        let lo = loads().map(|h| at(h.1)).min().unwrap_or(0);
        let hi = loads().map(|h| at(h.1).wrapping_add(usize::try_from(h.2).unwrap_or(0))).max().unwrap_or(0);
        let eh = readers::headers(phdr, phnum).find(|h| h.0 == PT_GNU_EH_FRAME).map_or(0, |h| at(h.1));
        for h in loads() {
            let s = at(h.1);
            put(r, k, [s, s.wrapping_add(usize::try_from(h.2).unwrap_or(0)), lo, hi, eh, i]);
            k = k.wrapping_add(1);
        }
    }
    k.wrapping_sub(from)
}

/// Sort entries `from..from + n` of `r` by segment start (insertion sort: one `dlopen`'s set).
fn sort(r: &[AtomicUsize], from: usize, n: usize) {
    for j in 1..n {
        let e = entry(r, from.wrapping_add(j));
        let mut k = j;
        while k > 0 && word(r, from.wrapping_add(k).wrapping_sub(1), 0) > e[0] {
            put(r, from.wrapping_add(k), entry(r, from.wrapping_add(k).wrapping_sub(1)));
            k = k.wrapping_sub(1);
        }
        put(r, from.wrapping_add(k), e);
    }
}

/// The sorted output of a build: merges a segment overlapping the previous one of the same
/// object and sets the flags.
struct Out<'a> {
    r: &'a [AtomicUsize],
    n: usize,
    flags: usize,
    /// The highest object end seen so far, and its object.
    reach: usize,
    reach_i: usize,
}

impl<'a> Out<'a> {
    fn new(r: &'a [AtomicUsize]) -> Self {
        Self { r, n: 0, flags: 0, reach: 0, reach_i: usize::MAX }
    }

    fn push(&mut self, e: [usize; W]) {
        let [seg_lo, seg_hi, _, o_hi, _, i] = e;
        if let Some(last) = self.n.checked_sub(1) {
            let l_hi = word(self.r, last, 1);
            if seg_lo < l_hi {
                if word(self.r, last, 5) == i {
                    if seg_hi > l_hi {
                        set(self.r, last, 1, seg_hi);
                    }
                    return;
                }
                self.flags |= OVERLAP | NESTED;
            }
        }
        // A segment that starts inside another object's range: the ranges overlap.
        if seg_lo < self.reach && i != self.reach_i {
            self.flags |= NESTED;
        }
        if o_hi > self.reach {
            self.reach = o_hi;
            self.reach_i = i;
        }
        put(self.r, self.n, e);
        self.n = self.n.wrapping_add(1);
    }

    fn finish(self) {
        set_head(self.r, 0, self.n);
        set_head(self.r, 1, self.flags);
    }
}

/// Bytes of a region of `cap` entries.
fn bytes(cap: usize) -> usize {
    H.wrapping_add(cap.wrapping_mul(W)).wrapping_mul(size_of::<usize>())
}

/// Map a region of at least `cap` entries (its capacity written): its address.
fn alloc(cap: usize) -> Option<usize> {
    let cap = cap.max(1);
    let at = raw::mmap(0, bytes(cap), map::PROT_READ | map::PROT_WRITE, super::MAP_PRIVATE_I32 | super::MAP_ANONYMOUS_I32, -1, 0).ok()?;
    // UNSAFE(slice): the header of the region just mapped (private to this thread until published).
    #[allow(unsafe_code)]
    // SAFETY: a fresh read-write anonymous mapping of at least H words, zero-filled.
    let head = unsafe { core::slice::from_raw_parts(core::ptr::with_exposed_provenance::<AtomicUsize>(at), H) };
    set_head(head, 2, cap);
    Some(at)
}

/// The writer's copy `c` with room for `cap` entries: its region (a new, larger one when needed).
fn room(c: usize, cap: usize) -> Option<&'static [AtomicUsize]> {
    let slot = COPY.get(c)?;
    let at = slot.load(Relaxed);
    let have = if at == 0 { 0 } else { head(region(at), 2) };
    if at != 0 && have >= cap {
        return Some(region(at));
    }
    let new = alloc(cap.max(have.wrapping_mul(2)).max(MIN_CAP))?;
    slot.store(new, Release);
    Some(region(new))
}

/// The writer's update: copy 0 becomes `src`'s entries (less those of object `gone`) and the
/// entries of objects `objs`; then copy 1 the same. Under the loader's lock.
fn update(gone: Option<usize>, objs: core::ops::Range<usize>) {
    if DISABLED.load(Relaxed) != 0 {
        return;
    }
    if update_with(gone, objs).is_none() {
        DISABLED.store(1, Relaxed);
        SEQ.fetch_add(2, Release);
    }
}

fn update_with(gone: Option<usize>, objs: core::ops::Range<usize>) -> Option<()> {
    let s = SEQ.load(Relaxed);
    let first = s == 0;
    let src = if first { region(start()?) } else { region(COPY.get(1)?.load(Relaxed)) };
    let n_src = head(src, 0);
    let m = objs.clone().map(segments).fold(0usize, usize::wrapping_add);
    if !first {
        // Readers to copy 1 (the old state) before copy 0 changes or moves; a reader still on
        // copy 0 sees `SEQ` move.
        SEQ.store(s.wrapping_add(1), Relaxed);
        fence(Release);
    }
    // The new entries go at the end of copy 0's room and are merged forward into its start.
    let dst = room(0, n_src.wrapping_add(m.wrapping_mul(2)))?;
    let tail = head(dst, 2).wrapping_sub(m);
    let filled = fill(dst, tail, objs);
    sort(dst, tail, filled);
    let mut out = Out::new(dst);
    let (mut a, mut b) = (0usize, 0usize);
    loop {
        let from_src = a < n_src && (b >= filled || word(src, a, 0) <= word(dst, tail.wrapping_add(b), 0));
        if from_src {
            let e = entry(src, a);
            a = a.wrapping_add(1);
            if Some(e[5]) != gone {
                out.push(e);
            }
        } else if b < filled {
            out.push(entry(dst, tail.wrapping_add(b)));
            b = b.wrapping_add(1);
        } else {
            break;
        }
    }
    let len = out.n;
    out.finish();
    // Readers to copy 0 (the new state) before copy 1 changes or moves (the first update: no
    // reader uses either copy before `SEQ` is 2).
    if !first {
        SEQ.store(s.wrapping_add(2), Release);
        fence(Release);
    }
    let Some(other) = room(1, len) else {
        if first {
            SEQ.store(2, Release);
        }
        return None;
    };
    let words = H.wrapping_add(len.wrapping_mul(W));
    for (k, (w, v)) in other.iter().zip(dst).enumerate().take(words) {
        // The capacity word stays the region's own.
        if k != 2 {
            w.store(v.load(Relaxed), Relaxed);
        }
    }
    if first {
        SEQ.store(2, Release);
    }
    Some(())
}

/// A `dlopen` published objects `base..len` (`readers::publish`, under the loader's lock).
pub(super) fn added(base: usize, len: usize) {
    update(None, base..len);
}

/// `dlclose` unpublished object `k` (`readers::unpublish`, under the loader's lock).
pub(super) fn removed(k: usize) {
    update(Some(k), 0..0);
}
