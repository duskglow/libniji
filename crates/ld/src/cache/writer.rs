//! The loader cache writer (UTIL-2, plan/utilities.md §4; format: spec Appendix D.3.1 and D.3.2,
//! order: D.3.3).
//!
//! The writer produces exactly the new format (`glibc-ld.so.cache1.1`) with the tag 0 (generator
//! text) and tag 1 (glibc-hwcaps names) extensions: what libniji's reader ([`super::Cache`]) reads
//! and what other direct readers of the file expect.
//!
//! * **Order.** Entries are sorted by [`order`]: descending by key under [`super::compare`] (the
//!   comparison the reader's binary search uses, so writer order and reader search agree by
//!   construction); ties by configuration order (`Entry::order`), then a hwcaps entry before a
//!   plain one, then path bytes; the remaining fields only separate entries no reader can tell
//!   apart. The order is total, so the output depends only on the set of entries given, never on
//!   the order they arrive in (no directory-order or file-system dependence).
//! * **Untrusted input.** Keys, paths, hwcaps names and the generator text come from file names
//!   and ELF headers: each is validated and bounded ([`NAME_MAX`], [`PATH_MAX`], and every offset
//!   must fit the format's 32 bits) before anything is written. The number of entries, of distinct
//!   hwcaps names and the generator text's length have no cap of their own: the format's 32-bit
//!   offsets bound them ([`WriteError::TooLarge`]). A path is absolute, its components are non-empty and neither `.` nor
//!   `..`, and it ends in `/<key>`: the value is the soname link in the scanned directory
//!   (Appendix D.3.1). A hwcaps entry's path is `<dir>/glibc-hwcaps/<name>/<file>` with any valid
//!   file name (no links are made there); when `<file>` is not the key, the key gets its own
//!   string, otherwise a key is the tail of its path.
//! * **Allocation only past the common case.** The caller's entry slice is sorted in place and the
//!   file is written into the caller's buffer ([`encoded_len`] gives its size). The distinct hwcaps
//!   names are kept in a fixed inline list ([`NAMES_INLINE`]); a cache with more spills them to
//!   allocated storage, fallibly ([`WriteError::Oom`] when memory runs out). Nothing is mapped or
//!   executed.

use alloc::vec::Vec;
use core::cmp::Ordering;

use super::{compare, ENTRY, EXT_MAGIC, HEADER, HEADER_FLAGS, HWCAP_INDEX, MAGIC, TAG_GENERATOR, TAG_HWCAPS};

/// The longest key and hwcaps name, and the longest path component (bytes, without the NUL).
pub const NAME_MAX: usize = 255;
/// The longest path (bytes, without the NUL).
pub const PATH_MAX: usize = 4095;
/// Distinct hwcaps subdirectory names held without allocating (a system has a handful); more
/// spill to allocated storage.
pub const NAMES_INLINE: usize = 64;

/// The directory a hwcaps entry's subdirectory lives in.
const HWCAPS_DIR: &[u8] = b"/glibc-hwcaps/";

/// One library to record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry<'a> {
    /// Library type and ABI (Appendix D.3.1; `Arch::CACHE_FLAGS` for this architecture).
    pub flags: u32,
    /// The soname, or the file name of a library without one.
    pub key: &'a [u8],
    /// The absolute path of the soname link: `<dir>/<key>`, or `<dir>/glibc-hwcaps/<name>/<key>`.
    pub path: &'a [u8],
    /// The glibc-hwcaps subdirectory name of a hwcaps entry.
    pub hwcaps: Option<&'a [u8]>,
    /// The position of the entry's directory in the configuration (smaller = earlier); a hwcaps
    /// entry takes the position of the directory it is under.
    pub order: u32,
}

/// Why a cache was not written. An index names the entry in the caller's order (before sorting).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteError {
    /// The key is empty, longer than [`NAME_MAX`], `.` or `..`, or holds `/` or NUL.
    Key(usize),
    /// The path is not absolute and canonical, is longer than [`PATH_MAX`], or does not end in
    /// `/<key>` (`/glibc-hwcaps/<name>/<file>` for a hwcaps entry, `<file>` any valid name).
    Path(usize),
    /// The hwcaps name is not a valid subdirectory name (as for a key).
    Hwcaps(usize),
    /// Memory for the list of distinct hwcaps names ran out (only past [`NAMES_INLINE`] names).
    Oom,
    /// The generator text holds NUL.
    Generator,
    /// An offset or size would not fit the format's 32 bits.
    TooLarge,
    /// The output buffer is smaller than the file: the size it needs.
    Buffer(usize),
}

/// The writer's total order (module documentation): `Less` when `a` is written before `b`.
pub fn order(a: &Entry<'_>, b: &Entry<'_>) -> Ordering {
    compare(a.key, b.key)
        .reverse()
        .then(a.order.cmp(&b.order))
        .then(b.hwcaps.is_some().cmp(&a.hwcaps.is_some()))
        .then_with(|| a.path.cmp(b.path))
        .then_with(|| a.key.cmp(b.key))
        .then_with(|| a.hwcaps.cmp(&b.hwcaps))
        .then(a.flags.cmp(&b.flags))
}

/// A file or directory name: non-empty, bounded, not `.` or `..`, no `/` or NUL.
fn name_ok(s: &[u8]) -> bool {
    !s.is_empty() && s.len() <= NAME_MAX && s != b"." && s != b".." && !s.iter().any(|&c| c == 0 || c == b'/')
}

/// An absolute path whose components are all names.
fn path_ok(p: &[u8]) -> bool {
    p.len() <= PATH_MAX && p.strip_prefix(b"/").is_some_and(|rest| rest.split(|&c| c == b'/').all(name_ok))
}

/// Validate entry `i`.
fn check(i: usize, e: &Entry<'_>) -> Result<(), WriteError> {
    if !name_ok(e.key) {
        return Err(WriteError::Key(i));
    }
    if let Some(h) = e.hwcaps {
        if !name_ok(h) {
            return Err(WriteError::Hwcaps(i));
        }
    }
    let dir = match e.hwcaps {
        None => e.path.strip_suffix(e.key).and_then(|d| d.strip_suffix(b"/")),
        // ldconfig makes no soname links in a glibc-hwcaps subdirectory, so such an entry names the file itself
        // (any valid name; UTIL-LDCONFIG probe): only the directory part is fixed.
        Some(h) => e
            .path
            .iter()
            .rposition(|&c| c == b'/')
            .and_then(|k| Some((e.path.get(..k)?, e.path.get(k.checked_add(1)?..)?)))
            .and_then(|(d, f)| if name_ok(f) { Some(d) } else { None })
            .and_then(|d| d.strip_suffix(h))
            .and_then(|d| d.strip_suffix(HWCAPS_DIR)),
    };
    if dir.is_none() || !path_ok(e.path) {
        return Err(WriteError::Path(i));
    }
    Ok(())
}

/// Whether the entry's key is the last component of its path (then the key shares the path's
/// string); otherwise (a hwcaps entry naming a file other than the soname) it needs its own.
fn key_is_tail(e: &Entry<'_>) -> bool {
    e.path.strip_suffix(e.key).is_some_and(|d| d.ends_with(b"/"))
}

/// The distinct hwcaps names, sorted by bytes (deterministic indexes): inline while there are at
/// most [`NAMES_INLINE`], in `heap` (then the only list) beyond.
struct Names<'a> {
    names: [&'a [u8]; NAMES_INLINE],
    len: usize,
    heap: Vec<&'a [u8]>,
}

impl<'a> Names<'a> {
    fn collect(entries: &[Entry<'a>]) -> Result<Self, WriteError> {
        let mut n = Names { names: [&[]; NAMES_INLINE], len: 0, heap: Vec::new() };
        for h in entries.iter().filter_map(|e| e.hwcaps) {
            if !n.heap.is_empty() {
                // Spilled: gather, skipping a run of one name (a directory's entries arrive together); sorted and
                // deduplicated once at the end, so the cost stays O(n log n) whatever the order.
                if n.heap.last() != Some(&h) {
                    n.heap.try_reserve(1).map_err(|_| WriteError::Oom)?;
                    #[allow(clippy::disallowed_methods)] // capacity reserved on the line above, so this cannot allocate
                    n.heap.push(h);
                }
                continue;
            }
            let used = n.names.get(..n.len).unwrap_or(&[]);
            if let Err(at) = used.binary_search(&h) {
                if n.len >= NAMES_INLINE {
                    n.heap.try_reserve(NAMES_INLINE.saturating_mul(2)).map_err(|_| WriteError::Oom)?;
                    #[allow(clippy::disallowed_methods)] // capacity reserved above (twice the inline list), so no allocation
                    n.heap.extend_from_slice(&n.names);
                    #[allow(clippy::disallowed_methods)] // as above: one more than the inline list fits the reservation
                    n.heap.push(h);
                    continue;
                }
                if let Some(tail) = n.names.get_mut(at..=n.len) {
                    tail.rotate_right(1);
                }
                if let Some(slot) = n.names.get_mut(at) {
                    *slot = h;
                }
                n.len = n.len.saturating_add(1);
            }
        }
        if !n.heap.is_empty() {
            n.heap.sort_unstable();
            n.heap.dedup();
            n.len = n.heap.len();
        }
        Ok(n)
    }

    fn used(&self) -> &[&'a [u8]] {
        if self.heap.is_empty() { self.names.get(..self.len).unwrap_or(&[]) } else { &self.heap }
    }
}

/// Where each part of the file goes.
struct Layout {
    strings: usize,
    len_strings: usize,
    ext: usize,
    generator: usize,
    array: usize,
    total: usize,
}

fn add(a: usize, b: usize) -> Result<usize, WriteError> {
    a.checked_add(b).ok_or(WriteError::TooLarge)
}

fn mul(a: usize, b: usize) -> Result<usize, WriteError> {
    a.checked_mul(b).ok_or(WriteError::TooLarge)
}

fn up4(a: usize) -> Result<usize, WriteError> {
    a.checked_next_multiple_of(4).ok_or(WriteError::TooLarge)
}

fn layout(entries: &[Entry<'_>], names: &Names<'_>, generator: &[u8]) -> Result<Layout, WriteError> {
    let strings = add(HEADER, mul(ENTRY, entries.len())?)?;
    let mut len_strings = 0usize;
    for s in names.used().iter().copied().chain(entries.iter().map(|e| e.path)) {
        len_strings = add(len_strings, add(s.len(), 1)?)?;
    }
    for e in entries.iter().filter(|e| !key_is_tail(e)) {
        len_strings = add(len_strings, add(e.key.len(), 1)?)?;
    }
    let strings_end = add(strings, len_strings)?;
    let records = usize::from(!generator.is_empty()).saturating_add(usize::from(names.len != 0));
    let (ext, generator_at, array, total) = if records == 0 {
        (0, 0, 0, strings_end)
    } else {
        let ext = up4(strings_end)?;
        let generator_at = add(ext, add(8, mul(16, records)?)?)?;
        let generator_end = add(generator_at, generator.len())?;
        if names.len == 0 {
            (ext, generator_at, 0, generator_end)
        } else {
            let array = up4(generator_end)?;
            (ext, generator_at, array, add(array, mul(4, names.len)?)?)
        }
    };
    if u32::try_from(total).is_err() {
        return Err(WriteError::TooLarge);
    }
    Ok(Layout { strings, len_strings, ext, generator: generator_at, array, total })
}

/// Validate everything; the hwcaps names and the layout.
fn prepare<'a>(entries: &[Entry<'a>], generator: &[u8]) -> Result<(Names<'a>, Layout), WriteError> {
    if generator.contains(&0) {
        return Err(WriteError::Generator);
    }
    for (i, e) in entries.iter().enumerate() {
        check(i, e)?;
    }
    let names = Names::collect(entries)?;
    let layout = layout(entries, &names, generator)?;
    Ok((names, layout))
}

/// The size of the file [`write`] makes for `entries` and `generator` (validating them).
pub fn encoded_len(entries: &[Entry<'_>], generator: &[u8]) -> Result<usize, WriteError> {
    prepare(entries, generator).map(|(_, l)| l.total)
}

/// The output buffer, written at checked positions.
struct Out<'o> {
    buf: &'o mut [u8],
    need: usize,
}

impl Out<'_> {
    fn put(&mut self, at: usize, bytes: &[u8]) -> Result<(), WriteError> {
        let end = add(at, bytes.len())?;
        self.buf.get_mut(at..end).ok_or(WriteError::Buffer(self.need))?.copy_from_slice(bytes);
        Ok(())
    }

    fn u32(&mut self, at: usize, v: usize) -> Result<(), WriteError> {
        let v = u32::try_from(v).map_err(|_| WriteError::TooLarge)?;
        self.put(at, &v.to_le_bytes())
    }
}

/// Write the cache for `entries` (sorted in place by [`order`]) with the generator text
/// `generator` (no tag 0 record when it is empty) into `out`; the number of bytes written. Nothing
/// is written unless every input is valid and `out` is large enough.
pub fn write(entries: &mut [Entry<'_>], generator: &[u8], out: &mut [u8]) -> Result<usize, WriteError> {
    let (names, l) = prepare(entries, generator)?;
    if out.len() < l.total {
        return Err(WriteError::Buffer(l.total));
    }
    entries.sort_unstable_by(order);
    let mut o = Out { buf: out, need: l.total };
    o.buf.get_mut(..l.total).ok_or(WriteError::Buffer(l.total))?.fill(0);
    o.put(0, MAGIC)?;
    o.u32(20, entries.len())?;
    o.u32(24, l.len_strings)?;
    o.put(28, &[HEADER_FLAGS])?;
    o.u32(32, l.ext)?;
    // The string table: the hwcaps names, then each entry's path, followed by its key when the key
    // is not the tail of the path.
    let mut at = l.strings;
    for n in names.used() {
        o.put(at, n)?;
        at = add(at, add(n.len(), 1)?)?;
    }
    for (i, e) in entries.iter().enumerate() {
        let rec = add(HEADER, mul(ENTRY, i)?)?;
        let path_end = add(at, add(e.path.len(), 1)?)?;
        let key_at =
            if key_is_tail(e) { add(at, e.path.len().saturating_sub(e.key.len()))? } else { path_end };
        o.put(rec, &e.flags.to_le_bytes())?;
        o.u32(add(rec, 4)?, key_at)?;
        o.u32(add(rec, 8)?, at)?;
        let hwcap = match e.hwcaps {
            None => 0,
            Some(h) => {
                let index = names.used().binary_search(&h).map_err(|_| WriteError::Hwcaps(i))?;
                HWCAP_INDEX | u64::try_from(index).map_err(|_| WriteError::TooLarge)?
            }
        };
        o.put(add(rec, 16)?, &hwcap.to_le_bytes())?;
        o.put(at, e.path)?;
        at = path_end;
        if key_at == path_end {
            o.put(at, e.key)?;
            at = add(at, add(e.key.len(), 1)?)?;
        }
    }
    if l.ext != 0 {
        let mut records = 0usize;
        let mut record = |o: &mut Out<'_>, tag: u32, offset: usize, size: usize| -> Result<(), WriteError> {
            let rec = add(l.ext, add(8, mul(16, records)?)?)?;
            o.put(rec, &tag.to_le_bytes())?;
            o.u32(add(rec, 8)?, offset)?;
            o.u32(add(rec, 12)?, size)?;
            records = records.saturating_add(1);
            Ok(())
        };
        if !generator.is_empty() {
            record(&mut o, TAG_GENERATOR, l.generator, generator.len())?;
            o.put(l.generator, generator)?;
        }
        if names.len != 0 {
            record(&mut o, TAG_HWCAPS, l.array, mul(4, names.len)?)?;
            // Each name's offset again: they lead the string table, in this order.
            let mut off = l.strings;
            for (k, n) in names.used().iter().enumerate() {
                o.u32(add(l.array, mul(4, k)?)?, off)?;
                off = add(off, add(n.len(), 1)?)?;
            }
        }
        o.put(l.ext, &EXT_MAGIC.to_le_bytes())?;
        o.u32(add(l.ext, 4)?, records)?;
    }
    Ok(l.total)
}
