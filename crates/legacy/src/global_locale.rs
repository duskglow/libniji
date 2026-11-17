//! The process-global locale of `setlocale`, the interned locale values, and the mapped locale
//! files (spec §7.1–7.3; D-7.1, D-7.7, D-7.8).
//!
//! **Interned locales.** Every locale the process selects or creates is kept, deduplicated, for
//! the rest of the process: a handle (index + 1; 0 is `"C"`) names it, so a thread that read a
//! handle never sees its locale freed, and `setlocale` stays safe while other threads use the
//! global locale (D-7.1). Distinct locales are few (one per combination of categories a
//! process uses), but there is no limit on them but memory: the static table holds the common
//! case ([`INLINE`] entries) and further entries spill to allocated chunks that are never moved
//! ([`SPILL`]). Names are interned the same way.
//!
//! **Files.** Locale files are mapped once, by path, and never unmapped (D-7.7). Charmaps
//! (D-6.8) are read from `/usr/share/i18n/charmaps`, parsed once, and kept.
//!
//! **Invalidation** (D-X.13). [`invalidate`] advances a generation. The next use of a mapped file
//! in a new generation compares the file's identity (`stat`) with the one recorded when it was
//! mapped: unchanged, the mapping is used again; changed, the file is mapped again (the old
//! mapping stays, for the locales that point into it) unless its bytes are the ones mapped
//! already (a reinstall, a regenerated locale), when the new mapping is released and the old one
//! kept, so repeated updates that change nothing grow nothing; gone, it is missing. A charmap is
//! parsed again only when its file was mapped again. Then the global locale is loaded again by
//! its names. The only failure besides a missing file is memory exhaustion (`ENOMEM`).
//!
//! The values live at addresses this module leaked; the POSIX layer turns an address back into
//! a reference ([`Objects`]) and maps files ([`Mapper`]).

use core::sync::atomic::{fence, AtomicU64, AtomicUsize, Ordering::AcqRel, Ordering::Acquire, Ordering::Relaxed, Ordering::Release};

use niji::charmap::{self, inflate, Charmap};
use niji::iconv::{self, Codec};
use niji::text::CharmapRef;
use niji::error::{Errno, KResult};
use niji::locale::file::Cat;
use niji::locale::view::LocaleView;
use niji::locale::{self, name, Data, Locale, Source};
use niji::mem::{TryBox, TryVec};
use niji_kernel::sys::Sys;
use niji_rt::sync::RawMutex;

/// Entries in the static table: interned locales, interned names and mapped files each count
/// their own entries in it (a process that loads one locale uses a few dozen). Entries past it
/// spill to allocated chunks.
pub const INLINE: usize = 1024;
/// The entries of the first spill chunk; chunk `k` holds `SPILL << k`, from entry
/// `INLINE + SPILL * (2^k - 1)` on. A chunk is allocated (fallibly, under [`LOCK`]) when its first
/// entry is about to be written, and its address published before that entry's count, so a
/// reader that holds an index finds its entry without the lock; chunks are never moved or freed.
pub const SPILL: usize = 64;
/// Spill chunk addresses (0: not allocated). Chunk 40 alone would hold `64 << 40` entries,
/// more than an address space: an index past the last chunk is memory exhaustion.
static SPILL_AT: [AtomicUsize; 40] = [const { AtomicUsize::new(0) }; 40];
/// The mappings made in all (each distinct file once, and a file replaced by different bytes and
/// mapped again after an invalidation once more): [`mappings`].
static MAPS: AtomicUsize = AtomicUsize::new(0);
/// Whether the last failed `setlocale` failed for want of memory rather than for a missing locale
/// ([`last_failure`]). Written under [`LOCK`].
static FAILED_NOMEM: AtomicUsize = AtomicUsize::new(0);

/// Entry `i` of the three tables: the `i`th interned locale, the `i`th interned name and the
/// `i`th mapped file (each table counts on its own). The tables are interleaved, one record per
/// index, so a process that loads one locale touches one page of them, not one per column
/// (separate 8 KiB columns cost one page fault each; task PAGEFAULTS). Every field is atomic and
/// all-zero words are a valid value of each (the POSIX layer reads a spill chunk through
/// [`Objects::slots_at`]).
pub struct Slot {
    interned: AtomicUsize,
    name_addr: AtomicUsize,
    name_len: AtomicUsize,
    file_name: AtomicUsize,
    file_addr: AtomicUsize,
    file_len: AtomicUsize,
    /// The generation the file was last checked in.
    file_gen: AtomicUsize,
    /// The file's identity ([`Identity`]).
    file_id: [AtomicU64; 7],
}
impl Slot {
    const fn new() -> Slot {
        Slot {
            interned: AtomicUsize::new(0),
            name_addr: AtomicUsize::new(0),
            name_len: AtomicUsize::new(0),
            file_name: AtomicUsize::new(0),
            file_addr: AtomicUsize::new(0),
            file_len: AtomicUsize::new(0),
            file_gen: AtomicUsize::new(0),
            file_id: [const { AtomicU64::new(0) }; 7],
        }
    }
}
/// The table, page-aligned (STARTUP-DATA-PAGE-ALIGN-3): start-up writes its first records, whose page
/// then stays the same wherever the objects ahead of it in `.lbss` end (unaligned, the head straddled
/// two pages). The alignment costs at most 4 KiB of zero-filled address space, no file bytes.
#[repr(C, align(4096))]
struct Slots([Slot; INLINE]);
static SLOTS: Slots = Slots([const { Slot::new() }; INLINE]);
static COUNT: AtomicUsize = AtomicUsize::new(0);
static NAME_COUNT: AtomicUsize = AtomicUsize::new(0);
static FILE_COUNT: AtomicUsize = AtomicUsize::new(0);
/// The most charmaps a process loads.
const CHARMAPS: usize = 32;
static CHARMAP_ADDR: [AtomicUsize; CHARMAPS] = [const { AtomicUsize::new(0) }; CHARMAPS];
static CHARMAP_COUNT: AtomicUsize = AtomicUsize::new(0);
/// Each charmap's file (its interned path's slot), the address of the mapping it was parsed
/// from, and the generation it was last checked in.
static CHARMAP_PATH: [AtomicUsize; CHARMAPS] = [const { AtomicUsize::new(0) }; CHARMAPS];
static CHARMAP_SRC: [AtomicUsize; CHARMAPS] = [const { AtomicUsize::new(0) }; CHARMAPS];
static CHARMAP_GEN: [AtomicUsize; CHARMAPS] = [const { AtomicUsize::new(0) }; CHARMAPS];
/// The invalidation generation (D-X.13).
static GENERATION: AtomicUsize = AtomicUsize::new(0);

/// A file's identity: device, inode, size, modification and change times (seconds and
/// nanoseconds).
pub type Identity = [u64; 7];
/// The installed charmaps (D-6.8).
const CHARMAP_DIR: &[u8] = b"/usr/share/i18n/charmaps/";
/// The largest charmap text inflated (the largest installed is a few MiB).
const CHARMAP_LIMIT: usize = 32 << 20;
static GLOBAL: AtomicUsize = AtomicUsize::new(0);
/// Serializes interning, mapping and `setlocale`/`newlocale` (loading may read files).
static LOCK: RawMutex = RawMutex::new();

/// Address-to-reference conversions for values this module leaked (the POSIX layer).
pub trait Objects {
    /// The locale view at `addr`, an address [`intern`] recorded (D-7.13).
    fn view_at(&self, addr: usize) -> &'static LocaleView;
    /// The `len` bytes at `addr`: a name this module recorded, or a mapping [`Mapper::map`]
    /// returned.
    fn bytes_at(&self, addr: usize, len: usize) -> &'static [u8];
    /// The charmap at `addr`, an address [`charmap`] recorded.
    fn charmap_at(&self, addr: usize) -> &'static Charmap;
    /// The `len` table entries at `addr`: a spill chunk this module leaked and recorded.
    fn slots_at(&self, addr: usize, len: usize) -> &'static [Slot];
}

/// The file system access locale loading needs (the POSIX layer).
pub trait Mapper: Objects {
    /// Map the file at `path` read-only, for the rest of the process: its address, length and
    /// identity, or the error (`ENOMEM` when the mapping did not fit; anything else is a file
    /// that cannot be read).
    fn map(&self, path: &[u8]) -> KResult<(usize, usize, Identity)>;
    /// Unmap a mapping [`Mapper::map`] just returned and nothing has used (it could not be
    /// recorded, or it holds the bytes of the mapping already in use).
    fn unmap(&self, addr: usize, len: usize);
    /// The identity of the file at `path` now (`None`: it cannot be examined).
    fn identity(&self, path: &[u8]) -> Option<Identity>;
    /// `LOCPATH`, unless unset or in secure mode.
    fn locpath(&self) -> Option<&[u8]>;
    /// Read the first bytes of the file at `path` into `buf` (without mapping it): the count.
    fn read_prefix(&self, path: &[u8], buf: &mut [u8]) -> Option<usize>;
    /// Call `f` with the name of each entry of the directory `path`.
    fn list_dir(&self, path: &[u8], f: &mut dyn FnMut(&[u8]));
}

fn addr_of<T: ?Sized>(r: &'static T) -> usize {
    core::ptr::from_ref(r).cast::<u8>().expose_provenance()
}

fn load(a: &[AtomicUsize], i: usize) -> usize {
    a.get(i).map_or(0, |x| x.load(Acquire))
}

/// Entry `i` (`None`: in a spill chunk not allocated, so never published). The static table
/// answers inline; the spill chunks are out of line, so the common case pays no more than the
/// static table's bounds check.
#[inline(always)]
fn slot(objs: &impl Objects, i: usize) -> Option<&'static Slot> {
    match SLOTS.0.get(i) {
        Some(s) => Some(s),
        None => spilled(objs, i),
    }
}

/// Spill chunk `k` and entry `i`'s place in it (`i` at least [`INLINE`]).
fn spill_place(i: usize) -> Option<(usize, usize)> {
    let j = i.checked_sub(INLINE)?.checked_add(SPILL)?;
    let k = usize::try_from(j.ilog2().checked_sub(SPILL.ilog2())?).ok()?;
    Some((k, j.checked_sub(SPILL.checked_shl(u32::try_from(k).ok()?)?)?))
}

/// The length of spill chunk `k`.
fn spill_len(k: usize) -> Option<usize> {
    SPILL.checked_mul(1usize.checked_shl(u32::try_from(k).ok()?)?)
}

/// [`slot`] past the static table.
#[cold]
#[inline(never)]
fn spilled(objs: &impl Objects, i: usize) -> Option<&'static Slot> {
    let (k, off) = spill_place(i)?;
    let addr = load(&SPILL_AT, k);
    if addr == 0 {
        return None;
    }
    objs.slots_at(addr, spill_len(k)?).get(off)
}

/// Entry `i`, about to be written and then published by its table's count: its spill chunk is
/// allocated first if it has none. `ENOMEM` when it cannot be. The caller holds [`LOCK`].
fn slot_for_write(objs: &impl Objects, i: usize) -> KResult<&'static Slot> {
    if let Some(s) = slot(objs, i) {
        return Ok(s);
    }
    let (k, off) = spill_place(i).ok_or(Errno::ENOMEM)?;
    let at = SPILL_AT.get(k).ok_or(Errno::ENOMEM)?;
    let len = spill_len(k).ok_or(Errno::ENOMEM)?;
    let mut v = TryVec::with_capacity(len)?;
    for _ in 0..len {
        v.push(Slot::new())?;
    }
    let chunk: &'static [Slot] = v.leak();
    at.store(addr_of(chunk), Release);
    chunk.get(off).ok_or(Errno::ENOMEM)
}

/// Field `f` of slot `i` (0 for an entry never published).
fn field(objs: &impl Objects, f: fn(&'static Slot) -> &'static AtomicUsize, i: usize) -> usize {
    slot(objs, i).map_or(0, |s| f(s).load(Acquire))
}

/// The mappings made so far (each distinct file once; a file replaced by other bytes, once
/// more after the invalidation that finds it).
pub fn mappings() -> usize {
    MAPS.load(Acquire)
}

/// The cause of the last failed `setlocale`: `ENOMEM` when memory ran out, else `ENOENT` (a
/// locale's data is not available).
pub fn last_failure() -> Errno {
    if FAILED_NOMEM.load(Acquire) != 0 { Errno::ENOMEM } else { Errno::ENOENT }
}

/// The view of the `"C"` locale, as a static for [`view`].
static C: LocaleView = LocaleView::c();

/// The interned locale named by `handle` (0: `"C"`).
pub fn get(objs: &impl Objects, handle: usize) -> &'static Locale {
    view(objs, handle).locale()
}

/// The view of the interned locale named by `handle` (0, and a value that names none: `"C"`'s;
/// D-7.13).
pub fn view(objs: &impl Objects, handle: usize) -> &'static LocaleView {
    let addr = handle.checked_sub(1).map_or(0, |i| field(objs, |s| &s.interned, i));
    if addr == 0 { &C } else { objs.view_at(addr) }
}

/// The handle of `l`, interning it if new: its view is built here, once, before the handle is
/// published (D-7.13). The caller holds [`LOCK`].
fn intern_locked(objs: &impl Objects, l: Locale) -> KResult<usize> {
    if l == Locale::C {
        return Ok(0);
    }
    let n = COUNT.load(Acquire);
    if let Some(i) = (0..n).find(|&i| *objs.view_at(field(objs, |s| &s.interned, i)).locale() == l) {
        return Ok(i.saturating_add(1));
    }
    let slot = &slot_for_write(objs, n)?.interned;
    // A category file an earlier view already tabulated is not tabulated again (LV-3).
    let reuse = |cat: Cat, d: &Data| {
        (0..n).find_map(|i| {
            let earlier = objs.view_at(field(objs, |s| &s.interned, i)).locale();
            if earlier.data(cat).same(d) { earlier.table(cat) } else { None }
        })
    };
    let leaked: &'static LocaleView = TryBox::new(LocaleView::new(l, n.saturating_add(1), &reuse)?)?.leak();
    slot.store(addr_of(leaked), Release);
    COUNT.store(n.saturating_add(1), Release);
    Ok(n.saturating_add(1))
}

/// The handle of `l`, interning it if new.
pub fn intern<S: Sys>(sys: &S, objs: &impl Objects, l: Locale) -> KResult<usize> {
    let _g = LOCK.lock(sys);
    intern_locked(objs, l)
}

/// The interned copy of `name` (deduplicated), `'static` and followed by a NUL, and its slot.
/// The caller holds [`LOCK`].
fn intern_name_locked(objs: &impl Objects, name: &[u8]) -> KResult<(usize, &'static [u8])> {
    let n = NAME_COUNT.load(Acquire);
    for i in 0..n {
        let stored = objs.bytes_at(field(objs, |s| &s.name_addr, i), field(objs, |s| &s.name_len, i));
        if stored == name {
            return Ok((i, stored));
        }
    }
    let e = slot_for_write(objs, n)?;
    let (addr, len) = (&e.name_addr, &e.name_len);
    let mut v = TryVec::with_capacity(name.len().saturating_add(1))?;
    v.extend_from_slice(name)?;
    v.push(0)?;
    let leaked: &'static [u8] = v.leak();
    addr.store(addr_of(leaked), Release);
    len.store(name.len(), Release);
    NAME_COUNT.store(n.saturating_add(1), Release);
    Ok((n, leaked.get(..name.len()).unwrap_or(&[])))
}

/// `name` interned, followed by a NUL in memory (for `_NL_LOCALE_NAME`, whose result is a C
/// string).
pub fn name_z<S: Sys>(sys: &S, objs: &impl Objects, name: &[u8]) -> KResult<&'static [u8]> {
    let _g = LOCK.lock(sys);
    intern_name_locked(objs, name).map(|(_, s)| s)
}

/// The global locale's handle.
pub fn global_handle() -> usize {
    GLOBAL.load(Acquire)
}

/// The global locale.
pub fn global(objs: &impl Objects) -> &'static Locale {
    get(objs, global_handle())
}

/// A locale [`Source`] over the mapper, through the file cache. Used with [`LOCK`] held.
struct Files<'a, M: Mapper> {
    m: &'a M,
    /// `m.locpath()`, read at the first need and kept for the selection (SETLOCALE-3: read
    /// once per category before). `None` until then. A plain `Cell`, not a `OnceCell`
    /// (HOTTEXT-TIME-CLUSTER-STABLE-2): `OnceCell::get_or_init` runs its first fill in an
    /// out-of-line cold generic with no hot-text key, which `setlocale(LC_ALL, "")` touches in cold
    /// text, where its distance from other cold code (and so its page-fault count) followed the
    /// codegen-unit partition of unrelated code.
    locpath: core::cell::Cell<Option<Option<&'a [u8]>>>,
    /// Whether a file could not be mapped or recorded for want of memory: a load that then
    /// finds nothing fails with `ENOMEM`, not as a missing locale.
    nomem: core::cell::Cell<bool>,
}

impl<'a, M: Mapper> Files<'a, M> {
    fn new(m: &'a M) -> Self {
        Files { m, locpath: core::cell::Cell::new(None), nomem: core::cell::Cell::new(false) }
    }

    /// `r`'s value; an error is noted when it is `ENOMEM`.
    fn recorded<T>(&self, r: KResult<T>) -> Option<T> {
        r.map_err(|e| {
            if e == Errno::ENOMEM {
                self.nomem.set(true);
            }
        })
        .ok()
    }

    /// The file at `path` mapped ([`Mapper::map`]); a failure for want of memory is noted.
    fn mapped(&self, path: &[u8]) -> Option<(usize, usize, Identity)> {
        self.recorded(self.m.map(path))
    }

    /// The error a load that found nothing reports: `ENOMEM` when memory ran out on the way,
    /// else `ENOENT`.
    fn missing(&self) -> Errno {
        if self.nomem.get() { Errno::ENOMEM } else { Errno::ENOENT }
    }
}

/// Record file slot `i`: the mapping, its identity, checked in `generation`. The caller holds
/// [`LOCK`].
fn record_file(objs: &impl Objects, i: usize, addr: usize, len: usize, id: &Identity, generation: usize) {
    if let Some(e) = slot(objs, i) {
        let (a, l, w, g) = (&e.file_addr, &e.file_len, &e.file_id, &e.file_gen);
        a.store(addr, Release);
        l.store(len, Release);
        for (slot, v) in w.iter().zip(id) {
            slot.store(*v, Release);
        }
        g.store(generation, Release);
    }
}

fn file_id(objs: &impl Objects, i: usize) -> Identity {
    let mut id = [0u64; 7];
    if let Some(w) = slot(objs, i).map(|e| &e.file_id) {
        for (v, slot) in id.iter_mut().zip(w) {
            *v = slot.load(Acquire);
        }
    }
    id
}

/// `niji_locale_invalidate`'s first half (D-X.13): every mapped file and parsed charmap is
/// checked against its file before its next use. The new generation.
pub fn invalidate() -> usize {
    GENERATION.fetch_add(1, Release).wrapping_add(1)
}

impl<M: Mapper> Source for Files<'_, M> {
    fn file(&self, path: &[u8]) -> Option<&'static [u8]> {
        let generation = GENERATION.load(Acquire);
        let n = FILE_COUNT.load(Acquire);
        for i in 0..n {
            let name = field(self.m, |s| &s.file_name, i);
            if self.m.bytes_at(field(self.m, |s| &s.name_addr, name), field(self.m, |s| &s.name_len, name)) != path {
                continue;
            }
            let current = self.m.bytes_at(field(self.m, |s| &s.file_addr, i), field(self.m, |s| &s.file_len, i));
            if field(self.m, |s| &s.file_gen, i) == generation {
                return Some(current);
            }
            // The first use since an invalidation: the same file is used again, a changed one
            // mapped again (the old mapping stays for the locales that point into it).
            if self.m.identity(path) == Some(file_id(self.m, i)) {
                if let Some(g) = slot(self.m, i).map(|e| &e.file_gen) {
                    g.store(generation, Release);
                }
                return Some(current);
            }
            let (addr, len, id) = self.mapped(path)?;
            if self.m.bytes_at(addr, len) == current {
                // Replaced by the same bytes: the mapping in use stays and the new one goes, so
                // an update that changes nothing grows nothing (and the locale built on the
                // file stays the same interned locale).
                if len != 0 {
                    self.m.unmap(addr, len);
                }
                let (old_addr, old_len) = (field(self.m, |s| &s.file_addr, i), field(self.m, |s| &s.file_len, i));
                record_file(self.m, i, old_addr, old_len, &id, generation);
                return Some(current);
            }
            MAPS.fetch_add(1, Release);
            record_file(self.m, i, addr, len, &id, generation);
            return Some(self.m.bytes_at(addr, len));
        }
        // A new file: only when it can be recorded (a load that cannot record it fails, for want
        // of memory, instead of mapping again at every call).
        let fname = &self.recorded(slot_for_write(self.m, n))?.file_name;
        let (addr, len, id) = self.mapped(path)?;
        let Some((slot, _)) = self.recorded(intern_name_locked(self.m, path)) else {
            if len != 0 {
                self.m.unmap(addr, len);
            }
            return None;
        };
        MAPS.fetch_add(1, Release);
        fname.store(slot, Release);
        record_file(self.m, n, addr, len, &id, generation);
        FILE_COUNT.store(n.saturating_add(1), Release);
        Some(self.m.bytes_at(addr, len))
    }

    fn locpath(&self) -> Option<&[u8]> {
        if let Some(p) = self.locpath.get() {
            return p;
        }
        let p = self.m.locpath();
        self.locpath.set(Some(p));
        p
    }

    fn charmap(&self, name: &[u8]) -> Option<&'static Charmap> {
        let n = CHARMAP_COUNT.load(Acquire);
        if let Some(i) = (0..n).find(|&i| names(self.m.charmap_at(load(&CHARMAP_ADDR, i)), name)) {
            return self.current_charmap(i);
        }
        let slot = CHARMAP_ADDR.get(n)?;
        let parsed = match self.read_charmap(name) {
            Some(cm) => cm,
            None => self.read_charmap(&self.find_charmap(name)?)?,
        };
        // The file found for an alias may be one already loaded under its own name.
        if let Some(i) = (0..n).find(|&i| self.m.charmap_at(load(&CHARMAP_ADDR, i)).name() == parsed.0.name()) {
            return self.current_charmap(i);
        }
        let cm = TryBox::new(parsed.0).ok()?.leak();
        slot.store(addr_of(cm), Release);
        record_charmap(n, parsed.1, parsed.2);
        CHARMAP_COUNT.store(n.saturating_add(1), Release);
        Some(cm)
    }
}

/// Record charmap slot `i`'s file (the interned path's slot) and the mapping it was parsed from,
/// checked in the current generation. The caller holds [`LOCK`].
fn record_charmap(i: usize, path: usize, src: usize) {
    if let (Some(p), Some(s), Some(g)) = (CHARMAP_PATH.get(i), CHARMAP_SRC.get(i), CHARMAP_GEN.get(i)) {
        p.store(path, Release);
        s.store(src, Release);
        g.store(GENERATION.load(Acquire), Release);
    }
}

/// Whether `name` names `cm`: its name or an alias, compared like codeset names (case and
/// punctuation aside: `eucjp` is `EUC-JP`).
fn names(cm: &Charmap, name: &[u8]) -> bool {
    core::iter::once(cm.name()).chain(cm.aliases()).any(|n| name::same_codeset(n, name))
}

/// A charmap file name (without `.gz`).
#[derive(Clone, Copy)]
struct Stem([u8; 128], usize);

impl core::ops::Deref for Stem {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        self.0.get(..self.1).unwrap_or(&[])
    }
}

impl<M: Mapper> Files<'_, M> {
    /// The charmap file whose name or header names `name`, by a scan of the directory: first a
    /// file name that matches, then the header names (`<code_set_name>`, aliases).
    fn find_charmap(&self, name: &[u8]) -> Option<Stem> {
        let mut entries: TryVec<Stem> = TryVec::new();
        self.m.list_dir(CHARMAP_DIR.strip_suffix(b"/").unwrap_or(CHARMAP_DIR), &mut |e| {
            let stem = e.strip_suffix(b".gz").unwrap_or(e);
            if stem.first() == Some(&b'.') || stem.is_empty() {
                return;
            }
            let mut s = Stem([0; 128], stem.len());
            if let Some(dst) = s.0.get_mut(..stem.len()) {
                dst.copy_from_slice(stem);
                let _ = entries.push(s);
            }
        });
        if let Some(i) = entries.iter().position(|s| name::same_codeset(s, name)) {
            return entries.get(i).copied();
        }
        let mut raw = [0u8; 4096];
        let mut path = [0u8; 256];
        for &s in entries.iter() {
            let mut found = false;
            for ext in [&b".gz"[..], b""] {
                let total = CHARMAP_DIR.len().saturating_add(s.len());
                let Some(p) = path.get_mut(..total.saturating_add(ext.len())) else { continue };
                let (dir, rest) = p.split_at_mut(CHARMAP_DIR.len());
                dir.copy_from_slice(CHARMAP_DIR);
                let (stem, tail) = rest.split_at_mut(s.len());
                stem.copy_from_slice(&s);
                tail.copy_from_slice(ext);
                let Some(n) = self.m.read_prefix(p, &mut raw) else { continue };
                let data = raw.get(..n).unwrap_or(&[]);
                let mut check = |t: &[u8]| charmap::header_names(t, &mut |h| found |= name::same_codeset(h, name));
                if ext.is_empty() {
                    check(data);
                } else if let Ok(t) = inflate::gunzip_prefix(data, 8192) {
                    check(&t);
                }
                break;
            }
            if found {
                return Some(s);
            }
        }
        None
    }

    /// Parse the charmap file for `name` (`<name>.gz`, else `<name>`): the charmap, its path's
    /// interned slot, and the address of the mapping it was parsed from. A name holding `/`, or
    /// `.` or `..`, is not a file name.
    fn read_charmap(&self, name: &[u8]) -> Option<(Charmap, usize, usize)> {
        if !name::component(name) {
            return None;
        }
        let mut buf = [0u8; 256];
        let total = CHARMAP_DIR.len().checked_add(name.len())?;
        buf.get_mut(..CHARMAP_DIR.len())?.copy_from_slice(CHARMAP_DIR);
        buf.get_mut(CHARMAP_DIR.len()..total)?.copy_from_slice(name);
        buf.get_mut(total..total.checked_add(3)?)?.copy_from_slice(b".gz");
        for path in [buf.get(..total.checked_add(3)?)?, buf.get(..total)?] {
            if let Some(raw) = self.file(path) {
                let cm = parse_charmap(path, raw)?;
                let (slot, _) = self.recorded(intern_name_locked(self.m, path))?;
                return Some((cm, slot, raw.as_ptr().expose_provenance()));
            }
        }
        None
    }

    /// Charmap slot `i`, checked against its file in the current generation: parsed again (in
    /// the same slot; the old charmap stays for its users) only when the file was mapped again.
    fn current_charmap(&self, i: usize) -> Option<&'static Charmap> {
        let cm = self.m.charmap_at(load(&CHARMAP_ADDR, i));
        let generation = GENERATION.load(Acquire);
        if load(&CHARMAP_GEN, i) == generation {
            return Some(cm);
        }
        let slot = load(&CHARMAP_PATH, i);
        let path = self.m.bytes_at(field(self.m, |s| &s.name_addr, slot), field(self.m, |s| &s.name_len, slot));
        let raw = self.file(path)?;
        let src = raw.as_ptr().expose_provenance();
        if src != load(&CHARMAP_SRC, i) {
            let fresh: &'static Charmap = TryBox::new(parse_charmap(path, raw)?).ok()?.leak();
            CHARMAP_ADDR.get(i)?.store(addr_of(fresh), Release);
            record_charmap(i, slot, src);
            return Some(fresh);
        }
        record_charmap(i, slot, src);
        Some(cm)
    }
}

/// Parse the charmap text `raw` read from `path` (inflated first for a `.gz` file).
fn parse_charmap(path: &[u8], raw: &[u8]) -> Option<Charmap> {
    if path.ends_with(b".gz") {
        let text = inflate::gunzip(raw, CHARMAP_LIMIT).ok()?;
        return Charmap::parse(&text).ok();
    }
    Charmap::parse(raw).ok()
}

/// The codec `name` (without suffixes) denotes for `iconv_open`: built in, else an installed
/// charmap found by its name or an alias (D-6.8, D-6.9).
pub fn codec<S: Sys, M: Mapper>(sys: &S, m: &M, name: &[u8]) -> Option<Codec> {
    if let Some(c) = iconv::builtin(name) {
        return Some(c);
    }
    let _g = LOCK.lock(sys);
    Files::new(m).charmap(name).map(|c| Codec::Charmap(CharmapRef(c)))
}

/// The name `setlocale(cat, "")` takes for `cat`: `LC_ALL`, then `LC_<cat>`, then `LANG`
/// (empty values ignored), else `"C"`.
pub fn env_name<'e>(cat: Cat, getenv: &dyn Fn(&[u8]) -> Option<&'e [u8]>) -> &'e [u8] {
    let nonempty = |k: &[u8]| getenv(k).filter(|v| !v.is_empty());
    nonempty(b"LC_ALL").or_else(|| nonempty(cat.name())).or_else(|| nonempty(b"LANG")).unwrap_or(b"C")
}

/// The variables `setlocale(cat, "")` reads, by number: [`ENV_LC_ALL`], [`env_key`] of a
/// category, [`ENV_LANG`] (`LC_ALL`, the categories in [`Cat::ALL`] order, `LANG`). The caller's
/// lookup is then an index, not a name compare: by name it cost ~150 instructions per variable,
/// fourteen per `setlocale(LC_ALL, "")` (SETLOCALE-CTYPE-ENV-2).
pub const ENV_LC_ALL: usize = 0;
/// `LANG`'s number (see [`ENV_LC_ALL`]).
pub const ENV_LANG: usize = 13;

/// `LC_<cat>`'s number (see [`ENV_LC_ALL`]).
#[inline]
pub const fn env_key(c: Cat) -> usize {
    c.index().saturating_add(1)
}

/// What `setlocale` is asked.
pub enum Request<'a> {
    /// Query the current name.
    Query,
    /// Take each category's name from the environment (values live while the call runs): the
    /// value of variable number `k` (see [`ENV_LC_ALL`]).
    Environment(&'a dyn Fn(usize) -> Option<&'static [u8]>),
    /// Use this name: for `LC_ALL`, a single name or a composite name.
    Name(&'a [u8]),
}

/// `base` with the categories in `cats` replaced by the locales `names` gives them (indexed
/// like [`Cat::ALL`]): `ENOENT` when a locale is not available, `ENOMEM` when memory ran out.
/// The caller holds [`LOCK`].
fn replace<M: Mapper>(m: &M, base: Locale, cats: &[Cat], names: &[&[u8]; 12]) -> KResult<Locale> {
    let src = Files::new(m);
    let mut l = base;
    // The alias and the archive search are shared by the categories of one name.
    let mut cache = locale::FindCache::default();
    for &c in cats {
        let (_, stored) = intern_name_locked(m, names.get(c.index()).copied().ok_or(Errno::ENOENT)?)?;
        let data = locale::find_cached(&src, c, stored, &mut cache).ok_or_else(|| src.missing())?;
        let reported = locale::builtin(stored).map_or(stored, |(_, r)| r);
        l = l.with(c, reported, data).ok_or(Errno::ENOENT)?;
    }
    locale::attach_charmap(&src, l).ok_or_else(|| src.missing())
}

/// [`replace`] for `setlocale` (§7.2, s7_2_c_4): a category whose current name is the name
/// asked for keeps its data, without a lookup (no file is read, `LOCPATH` is not consulted
/// again), as the reference does (SETLOCALE-4 probe). `current` itself when every category asked
/// keeps its name. The caller holds [`LOCK`]. `current` by reference: one copy of the locale
/// fewer on the first `setlocale` of a process (SETLOCALE-4 trace).
fn replace_changed<M: Mapper>(m: &M, current: &Locale, cats: &[Cat], names: &[&[u8]; 12]) -> KResult<Locale> {
    let mut changed = [Cat::Ctype; 12];
    let mut n = 0;
    for &c in cats {
        if names.get(c.index()).copied().ok_or(Errno::ENOENT)? != current.name(c) {
            *changed.get_mut(n).ok_or(Errno::ENOENT)? = c;
            n = n.saturating_add(1);
        }
    }
    if n == cats.len() {
        // Nothing kept (the common first call): the caller's list as it is.
        return replace(m, *current, cats, names);
    }
    match changed.get(..n).ok_or(Errno::ENOENT)? {
        [] => Ok(*current),
        some => replace(m, *current, some, names),
    }
}

/// Note why a `setlocale` failed ([`last_failure`]): `None`. The caller holds [`LOCK`].
#[cold]
#[inline(never)]
fn failed<T>(e: Errno) -> Option<T> {
    FAILED_NOMEM.store(usize::from(e == Errno::ENOMEM), Release);
    None
}

/// The query cache (WL-HOT-LS): for the global locale `QUERY_HANDLE`, each category's answer
/// to `setlocale(cat, NULL)` (index 12: `LC_ALL`'s composite name) as its interned name's
/// slot + 1 (0: not cached yet). Programs query once per output line (`ls -l`: gnulib's
/// `hard_locale (LC_TIME)`), and the locked path costs ~1,200 instructions against ~100.
/// Written only with [`LOCK`] held; a handle change runs between two `QUERY_SEQ` increments
/// (odd while it runs), so a lock-free reader that sees the same even sequence number before
/// and after its reads saw one handle's slots. A name for a handle never changes (the handle
/// names one interned locale) and name slots are never reused, so a hit is always right.
static QUERY_SEQ: AtomicUsize = AtomicUsize::new(0);
static QUERY_HANDLE: AtomicUsize = AtomicUsize::new(0);
static QUERY_SLOT: [AtomicUsize; 13] = [const { AtomicUsize::new(0) }; 13];

/// The query cache's index for `cat` (`None`: `LC_ALL`).
fn query_index(cat: Option<Cat>) -> usize {
    cat.map_or(12, Cat::index)
}

/// The cached answer to `setlocale(cat, NULL)` for the current global locale, without the lock.
fn query_cached(objs: &impl Objects, cat: Option<Cat>) -> Option<&'static [u8]> {
    let seq = QUERY_SEQ.load(Acquire);
    if seq & 1 != 0 {
        return None;
    }
    let handle = QUERY_HANDLE.load(Acquire);
    let slot = load(&QUERY_SLOT, query_index(cat)).checked_sub(1)?;
    fence(Acquire);
    if QUERY_SEQ.load(Relaxed) != seq || handle != GLOBAL.load(Acquire) {
        return None;
    }
    Some(objs.bytes_at(field(objs, |s| &s.name_addr, slot), field(objs, |s| &s.name_len, slot)))
}

/// [`query_cached`] for the global locale `handle` only: `None` once the global locale is another.
fn query_cached_for(objs: &impl Objects, cat: Option<Cat>, handle: usize) -> Option<&'static [u8]> {
    let seq = QUERY_SEQ.load(Acquire);
    if seq & 1 != 0 || QUERY_HANDLE.load(Acquire) != handle {
        return None;
    }
    let slot = load(&QUERY_SLOT, query_index(cat)).checked_sub(1)?;
    fence(Acquire);
    if QUERY_SEQ.load(Relaxed) != seq || handle != GLOBAL.load(Acquire) {
        return None;
    }
    Some(objs.bytes_at(field(objs, |s| &s.name_addr, slot), field(objs, |s| &s.name_len, slot)))
}

/// `setlocale(cat, "")` when every category asked already has the name the environment selects
/// for it (§7.2, s7_2_c_4: such a category keeps its data, nothing is looked up): the answer
/// from the query cache, without the lock and without copying or comparing whole locales. The
/// environment is read on every call (`getenv`; it may have changed since the last one); only
/// the selection is skipped. `None`: a name differs, or the cache has no answer for this global
/// locale; [`setlocale_locked`] then decides (SETLOCALE-CTYPE-ENV).
pub fn setlocale_env_kept(objs: &impl Objects, cat: Option<Cat>, getenv: &dyn Fn(usize) -> Option<&'static [u8]>) -> Option<&'static [u8]> {
    let handle = GLOBAL.load(Acquire);
    // Interned locales never change, so their names may be read without the lock.
    let current = get(objs, handle);
    let one = [cat.unwrap_or(Cat::Ctype)];
    let cats: &[Cat] = if cat.is_some() { &one } else { &Cat::ALL };
    // The selection of `setlocale_locked`: `LC_ALL`, then `LC_<cat>`, then `LANG`, else "C".
    let nonempty = |k: usize| getenv(k).filter(|v| !v.is_empty());
    let all = nonempty(ENV_LC_ALL);
    let mut lang: Option<&'static [u8]> = None;
    for &c in cats {
        let n = match all.or_else(|| nonempty(env_key(c))) {
            Some(n) => n,
            None => *lang.get_or_insert_with(|| nonempty(ENV_LANG).unwrap_or(b"C")),
        };
        let s = current.name(c);
        // "POSIX" selects the built-in locale that reports itself as "C" (D-X.80).
        if !(same_name(s, n) || same_name(s, locale::reported(n))) {
            return None;
        }
    }
    query_cached_for(objs, cat, handle)
}

/// Record `slot` (an interned name's) as the answer for `cat` in the global locale `handle`.
/// The caller holds [`LOCK`].
fn query_record(handle: usize, cat: Option<Cat>, slot: usize) {
    if QUERY_HANDLE.load(Acquire) != handle {
        QUERY_SEQ.fetch_add(1, AcqRel);
        for s in &QUERY_SLOT {
            s.store(0, Release);
        }
        QUERY_HANDLE.store(handle, Release);
        QUERY_SEQ.fetch_add(1, AcqRel);
    }
    if let Some(s) = QUERY_SLOT.get(query_index(cat)) {
        s.store(slot.saturating_add(1), Release);
    }
}

/// `a == b` for two locale names, compared here eight bytes at a time rather than through a
/// library call: names are short, and the call made the fast path of [`setlocale`] save and
/// restore six registers and keep a frame, so its miss could not jump straight to
/// [`setlocale_locked`] (SETLOCALE-C-ROWS).
#[inline(always)]
fn same_name(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let word = |c: &[u8]| <[u8; 8]>::try_from(c).ok().map(u64::from_ne_bytes);
    let (mut a8, mut b8) = (a.chunks_exact(8), b.chunks_exact(8));
    for (x, y) in a8.by_ref().zip(b8.by_ref()) {
        if word(x) != word(y) {
            return false;
        }
    }
    a8.remainder().iter().zip(b8.remainder()).all(|(x, y)| x == y)
}

/// `setlocale(cat, …)` (`cat` `None` is `LC_ALL`). Returns the category's name or the
/// composite name (`'static`, followed by a NUL), or `None` if a name is unknown, in which
/// case nothing changes. A query is answered from the query cache when it can be.
///
/// Only the lock-free answers are here; the rest is [`setlocale_locked`], kept out of line so
/// that a call answered here does not set up that function's frame (several kilobytes of stack
/// buffers, six saved registers): SETLOCALE-C-ROWS.
pub fn setlocale<S: Sys, M: Mapper>(sys: &S, m: &M, cat: Option<Cat>, req: Request<'_>) -> Option<&'static [u8]> {
    match req {
        Request::Query => {
            if let Some(s) = query_cached(m, cat) {
                return Some(s);
            }
        }
        // The name the query would answer (for `LC_ALL`: the single or composite name) names
        // every category asked as it is: nothing to look up (s7_2_c_4), no lock. `"POSIX"` is
        // compared as the `"C"` it reports: otherwise every repeated setlocale(cat, "POSIX")
        // missed here and rebuilt, compared and copied whole locales under the lock (~2,300
        // instructions, partly memmove whose count follows stack alignment).
        Request::Name(n) => {
            // The name as given first: "C" and every file name match there at no extra cost.
            if let Some(s) = query_cached(m, cat).filter(|&s| same_name(s, n) || same_name(s, locale::reported(n))) {
                return Some(s);
            }
        }
        Request::Environment(_) => {}
    }
    setlocale_locked(sys, m, cat, &req)
}

/// [`setlocale`] when the query cache did not answer, under [`LOCK`]; also called directly for a
/// [`Request::Environment`], which the cache never answers. A caller that calls this itself puts
/// its stack buffers right below its own frame, where they were when this was part of
/// `setlocale`: `setlocale(cat, "")` copies whole locales there and its memmove count follows
/// their alignment (+6 to +8 per call when reached through [`setlocale`]; SETLOCALE-C-ROWS).
#[inline(never)]
pub fn setlocale_locked<S: Sys, M: Mapper>(sys: &S, m: &M, cat: Option<Cat>, req: &Request<'_>) -> Option<&'static [u8]> {
    let _g = LOCK.lock(sys);
    match select_locked(m, cat, req) {
        Ok(s) => Some(s),
        Err(e) => failed(e),
    }
}

/// [`setlocale_locked`]'s work, with the cause of a failure: `ENOENT` (a locale is not
/// available) or `ENOMEM`. The caller holds [`LOCK`].
#[inline(always)]
fn select_locked<M: Mapper>(m: &M, cat: Option<Cat>, req: &Request<'_>) -> KResult<&'static [u8]> {
    let current = *global(m);
    let one = [cat.unwrap_or(Cat::Ctype)];
    let cats: &[Cat] = if cat.is_some() { &one } else { &Cat::ALL };
    let next = match *req {
        Request::Query => current,
        Request::Name(n) => {
            let names = match (cat, name::parse_composite(n)) {
                (None, Some(all)) => all,
                _ => [n; 12],
            };
            replace_changed(m, &current, cats, &names)?
        }
        Request::Environment(getenv) => {
            // `env_name` for the categories asked, with `LC_ALL` and `LANG` each looked up once
            // (SETLOCALE-START: three environment scans per category before).
            let nonempty = |k: usize| getenv(k).filter(|v| !v.is_empty());
            let all = nonempty(ENV_LC_ALL);
            let mut lang: Option<Option<&'static [u8]>> = None;
            let mut names: [&[u8]; 12] = [b"C"; 12];
            for &c in cats {
                let n = match all.or_else(|| nonempty(env_key(c))) {
                    Some(n) => n,
                    None => lang.get_or_insert_with(|| nonempty(ENV_LANG)).unwrap_or(b"C"),
                };
                *names.get_mut(c.index()).ok_or(Errno::ENOENT)? = n;
            }
            replace_changed(m, &current, cats, &names)?
        }
    };
    if next != current {
        GLOBAL.store(intern_locked(m, next)?, Release);
    } else if let Some(s) = query_cached(m, cat) {
        return Ok(s);
    }
    let (slot, s) = match cat {
        // Interned for its NUL: built-in names are plain literals.
        Some(c) => intern_name_locked(m, next.name(c))?,
        None => {
            let mut buf = [0u8; 12 * (name::MAX_NAME + 20)];
            let len = name::composite(next.names(), &mut buf).ok_or(Errno::ENOENT)?;
            intern_name_locked(m, buf.get(..len).ok_or(Errno::ENOENT)?)?
        }
    };
    query_record(GLOBAL.load(Acquire), cat, slot);
    Ok(s)
}

/// `niji_locale_invalidate` (D-X.13): advance the generation, then load the global locale
/// again by its names (the files are checked, and read again where they changed). `ENOENT` when
/// a name no longer resolves, `ENOMEM` when memory ran out; the global locale is then unchanged.
pub fn invalidate_and_reload<S: Sys, M: Mapper>(sys: &S, m: &M) -> KResult<()> {
    let _g = LOCK.lock(sys);
    invalidate();
    let current = *global(m);
    let mut names: [&[u8]; 12] = [b"C"; 12];
    for c in Cat::ALL {
        *names.get_mut(c.index()).ok_or(Errno::EINVAL)? = current.name(c);
    }
    let next = replace(m, Locale::C, &Cat::ALL, &names)?;
    if next != current {
        GLOBAL.store(intern_locked(m, next)?, Release);
    }
    Ok(())
}

/// `newlocale`: `base` with the categories `cats` taken from the locale `name` (a composite
/// name when `cats` is every category). The new locale's handle, or `ENOENT` (not available) or
/// `ENOMEM`.
pub fn newlocale<S: Sys, M: Mapper>(sys: &S, m: &M, cats: &[Cat], name: &[u8], base: Locale) -> KResult<usize> {
    let _g = LOCK.lock(sys);
    let names = match name::parse_composite(name) {
        Some(all) if cats.len() == Cat::ALL.len() => all,
        _ => [name; 12],
    };
    let l = replace(m, base, cats, &names)?;
    intern_locked(m, l)
}

/// `newlocale(mask, "", base)`: `base` with each category in `cats` taken from the locale the
/// environment names for it ([`env_name`], as `setlocale(cat, "")` reads it). The new locale's
/// handle, or `ENOENT` when a name does not resolve, `ENOMEM` when memory ran out.
pub fn newlocale_env<S: Sys, M: Mapper>(sys: &S, m: &M, cats: &[Cat], getenv: &dyn Fn(&[u8]) -> Option<&'static [u8]>, base: Locale) -> KResult<usize> {
    let mut names: [&[u8]; 12] = [b"C"; 12];
    for &c in cats {
        *names.get_mut(c.index()).ok_or(Errno::EINVAL)? = env_name(c, getenv);
    }
    let _g = LOCK.lock(sys);
    let l = replace(m, base, cats, &names)?;
    intern_locked(m, l)
}
