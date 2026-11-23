//! The locale archive (spec Appendix D.1.6, D-31.25): read, change and write the file
//! `localedef` keeps every compiled locale in.
//!
//! Layout written: the 56-byte header (14 little-endian words: magic, serial, then offset, used
//! and size of the name table, the string table, the locale-record table and the sum table),
//! the name table (12-byte entries: hash, name offset, record offset), the names (NUL-ended),
//! the records (a reference count and 13 (offset, length) slots, one per category number) and
//! the category images, 16-byte aligned, each distinct image stored once. The name table has a
//! prime size of at least twice the names plus one, grown until every name's hash lands in a
//! slot of its own, so a reader that probes the table finds each name at its first probe and an
//! absent one at an empty slot. The sum table is empty.
//!
//! An archive is untrusted data: [`Archive::parse`] checks every offset and length and refuses
//! a damaged file as a whole.

use crate::TryVec;
use niji::error::Errno;

/// The archive's magic number (D.1.6).
pub const MAGIC: u32 = 0xde02_0109;
/// Category slots per record (category numbers 0…12; 6, `LC_ALL`, is unused).
pub const SLOTS: usize = 13;
/// The header's bytes.
const HEADER: usize = 56;
/// A name table entry's bytes.
const ENTRY: usize = 12;
/// A record's bytes: the reference count and the slots.
const RECORD: usize = 4 + SLOTS * 8;

/// A locale's category images by category number (empty: none).
pub type Images = [TryVec<u8>; SLOTS];

/// One locale: its names (each looked up alone) and its images.
#[derive(Debug, Default)]
pub struct Locale {
    /// The names, in the order added.
    pub names: TryVec<TryVec<u8>>,
    /// The category images.
    pub images: Images,
}

/// An archive in memory.
#[derive(Debug, Default)]
pub struct Archive {
    /// The locales, in the order added.
    pub locales: TryVec<Locale>,
}

/// Why an archive cannot be used.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArchiveError {
    /// The file is not an archive, or an offset or length in it is out of bounds.
    Damaged,
    /// Memory ran out.
    NoMemory,
}

impl From<Errno> for ArchiveError {
    fn from(_: Errno) -> ArchiveError {
        ArchiveError::NoMemory
    }
}

fn word(d: &[u8], at: usize) -> Result<u32, ArchiveError> {
    let b = d.get(at..at.checked_add(4).ok_or(ArchiveError::Damaged)?).ok_or(ArchiveError::Damaged)?;
    let mut w = [0u8; 4];
    w.copy_from_slice(b);
    Ok(u32::from_le_bytes(w))
}

fn index(d: &[u8], at: usize) -> Result<usize, ArchiveError> {
    usize::try_from(word(d, at)?).map_err(|_| ArchiveError::Damaged)
}

/// The hash of a name in the name table: start at the length, then for each byte rotate left by
/// 9 bits and add the byte.
pub fn name_hash(name: &[u8]) -> u32 {
    let mut h = u32::try_from(name.len()).unwrap_or(u32::MAX);
    for &c in name {
        h = h.rotate_left(9).wrapping_add(u32::from(c));
    }
    h
}

fn copy(b: &[u8]) -> Result<TryVec<u8>, ArchiveError> {
    TryVec::from_slice(b).map_err(Into::into)
}

impl Archive {
    /// Read an archive. Every used name table entry (name offset and record offset not 0) is a
    /// name; names sharing a record are one locale.
    pub fn parse(d: &[u8]) -> Result<Archive, ArchiveError> {
        if word(d, 0)? != MAGIC {
            return Err(ArchiveError::Damaged);
        }
        let (table, size) = (index(d, 8)?, index(d, 16)?);
        let mut a = Archive::default();
        let mut records: TryVec<usize> = TryVec::new();
        for k in 0..size {
            let e = k.checked_mul(ENTRY).and_then(|o| o.checked_add(table)).ok_or(ArchiveError::Damaged)?;
            let (name_at, rec) = (index(d, e.checked_add(4).ok_or(ArchiveError::Damaged)?)?, index(d, e.checked_add(8).ok_or(ArchiveError::Damaged)?)?);
            if name_at == 0 || rec == 0 {
                continue;
            }
            let tail = d.get(name_at..).ok_or(ArchiveError::Damaged)?;
            let name = tail.get(..tail.iter().position(|&b| b == 0).ok_or(ArchiveError::Damaged)?).unwrap_or_default();
            let at = match records.iter().position(|&r| r == rec) {
                Some(i) => i,
                None => {
                    let mut images: Images = Default::default();
                    for (slot, img) in images.iter_mut().enumerate() {
                        let s = rec.checked_add(slot_at(slot)).ok_or(ArchiveError::Damaged)?;
                        let (off, len) = (index(d, s)?, index(d, s.checked_add(4).ok_or(ArchiveError::Damaged)?)?);
                        if len != 0 {
                            *img = copy(d.get(off..off.checked_add(len).ok_or(ArchiveError::Damaged)?).ok_or(ArchiveError::Damaged)?)?;
                        }
                    }
                    a.locales.push(Locale { names: TryVec::new(), images })?;
                    records.push(rec)?;
                    records.len().saturating_sub(1)
                }
            };
            if let Some(l) = a.locales.get_mut(at) {
                l.names.push(copy(name)?)?;
            }
        }
        Ok(a)
    }

    /// Every name, sorted bytewise.
    pub fn names(&self) -> Result<TryVec<&[u8]>, ArchiveError> {
        let mut v = TryVec::new();
        for l in self.locales.iter() {
            for n in l.names.iter() {
                v.push(n.as_slice())?;
            }
        }
        v.sort_unstable();
        Ok(v)
    }

    /// Whether `name` is in the archive.
    pub fn contains(&self, name: &[u8]) -> bool {
        self.locales.iter().any(|l| l.names.iter().any(|n| n.as_slice() == name))
    }

    /// Remove the name `name`; a locale left without names goes. Whether it was there.
    pub fn remove(&mut self, name: &[u8]) -> bool {
        let mut found = false;
        for l in self.locales.iter_mut() {
            if let Some(i) = l.names.iter().position(|n| n.as_slice() == name) {
                l.names.remove(i);
                found = true;
            }
        }
        let mut i = self.locales.len();
        while i > 0 {
            i = i.saturating_sub(1);
            if self.locales.get(i).is_some_and(|l| l.names.is_empty()) {
                self.locales.remove(i);
            }
        }
        found
    }

    /// Add a locale under `names` (each replacing any locale of that name).
    pub fn add(&mut self, names: &[&[u8]], images: Images) -> Result<(), ArchiveError> {
        for n in names {
            self.remove(n);
        }
        let mut l = Locale { names: TryVec::new(), images };
        for n in names {
            if !l.names.iter().any(|m| m.as_slice() == *n) {
                l.names.push(copy(n)?)?;
            }
        }
        self.locales.push(l).map_err(Into::into)
    }

    /// The archive's file.
    pub fn write(&self) -> Result<TryVec<u8>, ArchiveError> {
        let count: usize = self.locales.iter().map(|l| l.names.len()).sum();
        let size = table_size(self, count);
        let table = HEADER;
        let strings = table.checked_add(size.checked_mul(ENTRY).ok_or(ArchiveError::NoMemory)?).ok_or(ArchiveError::NoMemory)?;
        let names_len: usize = self.locales.iter().flat_map(|l| l.names.iter()).map(|n| n.len().saturating_add(1)).sum();
        let recs = align(strings.saturating_add(names_len), 4);
        let images_at = align(recs.saturating_add(self.locales.len().saturating_mul(RECORD)), 16);
        let mut out = TryVec::new();
        out.resize(images_at, 0)?;
        // The images, each distinct one once: (offset, length) per locale and slot.
        let mut slots: TryVec<[(u32, u32); SLOTS]> = TryVec::new();
        let mut stored: TryVec<(usize, usize)> = TryVec::new();
        for l in self.locales.iter() {
            let mut s = [(0u32, 0u32); SLOTS];
            for (slot, img) in l.images.iter().enumerate() {
                if img.is_empty() {
                    continue;
                }
                let same = stored.iter().find(|&&(o, n)| n == img.len() && out.get(o..o.saturating_add(n)) == Some(img.as_slice()));
                let off = match same {
                    Some(&(o, _)) => o,
                    None => {
                        let o = align(out.len(), 16);
                        out.resize(o, 0)?;
                        out.extend_from_slice(img)?;
                        stored.push((o, img.len()))?;
                        o
                    }
                };
                if let Some(x) = s.get_mut(slot) {
                    *x = (u32w(off)?, u32w(img.len())?);
                }
            }
            slots.push(s)?;
        }
        let file_len = out.len();
        let mut put = |at: usize, v: u32| {
            if let Some(b) = out.get_mut(at..at.saturating_add(4)) {
                b.copy_from_slice(&v.to_le_bytes());
            }
        };
        let mut name_at = strings;
        let mut name_slots: TryVec<(usize, usize)> = TryVec::new();
        for (r, l) in self.locales.iter().enumerate() {
            let rec = recs.saturating_add(r.saturating_mul(RECORD));
            put(rec, u32w(l.names.len())?);
            for (slot, &(o, n)) in slots.get(r).into_iter().flatten().enumerate() {
                put(rec.saturating_add(slot_at(slot)), o);
                put(rec.saturating_add(slot_at(slot)).saturating_add(4), n);
            }
            for n in l.names.iter() {
                let h = name_hash(n);
                let k = usize::try_from(h).unwrap_or(0).checked_rem(size).unwrap_or(0);
                let e = table.saturating_add(k.saturating_mul(ENTRY));
                put(e, h);
                put(e.saturating_add(4), u32w(name_at)?);
                put(e.saturating_add(8), u32w(rec)?);
                name_slots.push((name_at, n.len()))?;
                name_at = name_at.saturating_add(n.len()).saturating_add(1);
            }
        }
        let header = [
            MAGIC,
            0,
            u32w(table)?,
            u32w(count)?,
            u32w(size)?,
            u32w(strings)?,
            u32w(names_len)?,
            u32w(names_len)?,
            u32w(recs)?,
            u32w(self.locales.len())?,
            u32w(self.locales.len())?,
            u32w(file_len)?,
            0,
            0,
        ];
        for (i, v) in header.into_iter().enumerate() {
            put(i.saturating_mul(4), v);
        }
        let mut i = 0;
        for l in self.locales.iter() {
            for n in l.names.iter() {
                if let Some(&(at, len)) = name_slots.get(i) {
                    if let Some(dst) = out.get_mut(at..at.saturating_add(len)) {
                        dst.copy_from_slice(n);
                    }
                }
                i = i.saturating_add(1);
            }
        }
        Ok(out)
    }
}

/// The offset of slot `slot` in a record (after the reference count).
fn slot_at(slot: usize) -> usize {
    slot.saturating_mul(8).saturating_add(4)
}

fn u32w(n: usize) -> Result<u32, ArchiveError> {
    u32::try_from(n).map_err(|_| ArchiveError::NoMemory)
}

fn align(n: usize, to: usize) -> usize {
    n.checked_next_multiple_of(to).unwrap_or(usize::MAX)
}

/// The name table's size: the first prime of at least `2 * count + 1` (at least 3) at which the
/// names' hashes all fall in different slots.
fn table_size(a: &Archive, count: usize) -> usize {
    let mut size = count.saturating_mul(2).saturating_add(1).max(3);
    loop {
        if is_prime(size) && distinct(a, size) {
            return size;
        }
        size = size.saturating_add(1);
    }
}

fn is_prime(n: usize) -> bool {
    n >= 2 && (2..).take_while(|d: &usize| d.saturating_mul(*d) <= n).all(|d| n.checked_rem(d) != Some(0))
}

fn distinct(a: &Archive, size: usize) -> bool {
    let mut seen: TryVec<usize> = TryVec::new();
    for n in a.locales.iter().flat_map(|l| l.names.iter()) {
        let k = usize::try_from(name_hash(n)).unwrap_or(0).checked_rem(size).unwrap_or(0);
        if seen.contains(&k) || seen.push(k).is_err() {
            return false;
        }
    }
    true
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn images(tag: u8) -> Images {
        let mut i: Images = Default::default();
        for (n, img) in i.iter_mut().enumerate() {
            if n != 6 {
                *img = TryVec::from_slice(&[tag, u8::try_from(n).unwrap(), 1, 2, 3]).unwrap();
            }
        }
        i
    }

    /// D.1.6: what is written reads back (names, shared records, images), and libniji's
    /// reader finds every name.
    #[test]
    fn written_archive_reads_back() {
        let mut a = Archive::default();
        a.add(&[b"en_US", b"en_US.iso88591"], images(1)).unwrap();
        a.add(&[b"de_DE.utf8"], images(2)).unwrap();
        let file = a.write().unwrap();
        let b = Archive::parse(&file).unwrap();
        assert_eq!(b.names().unwrap().as_slice(), [&b"de_DE.utf8"[..], b"en_US", b"en_US.iso88591"]);
        assert_eq!(b.locales.len(), 2);
        let leaked: &'static [u8] = Box::leak(file.as_slice().to_vec().into_boxed_slice());
        let r = niji::locale::archive::Archive::parse(leaked).unwrap();
        for (name, tag) in [(&b"en_US"[..], 1u8), (b"en_US.iso88591", 1), (b"de_DE.utf8", 2)] {
            let rec = r.lookup(name).unwrap();
            assert_eq!(rec.category(niji::locale::file::Cat::Time).unwrap(), &[tag, 2, 1, 2, 3]);
        }
        assert!(r.lookup(b"fr_FR.utf8").is_none());
    }

    /// D.1.6: the name hash (black-box fit to the reference's archives) and a table where each
    /// name has a slot of its own.
    #[test]
    fn hash_and_slots() {
        assert_eq!(name_hash(b""), 0);
        assert_eq!(name_hash(b"a"), (1u32 << 9) + 0x61);
        let mut a = Archive::default();
        for i in 0..200u32 {
            a.add(&[format!("xx_{i}.utf8").as_bytes()], images(0)).unwrap();
        }
        let file = a.write().unwrap();
        let size = usize::try_from(word(&file, 16).unwrap()).unwrap();
        assert!(size >= 401 && is_prime(size));
        assert!(distinct(&a, size));
    }

    /// D.1.6: identical images are stored once; removing a name keeps the locale while it
    /// has another; a damaged archive is refused, not read out of bounds.
    #[test]
    fn dedup_remove_and_damage() {
        let mut a = Archive::default();
        a.add(&[b"a", b"b"], images(7)).unwrap();
        a.add(&[b"c"], images(7)).unwrap();
        let one = a.write().unwrap().len();
        assert!(a.remove(b"a") && !a.remove(b"a"));
        assert!(a.contains(b"b"));
        a.remove(b"b");
        assert_eq!(a.locales.len(), 1);
        let mut solo = Archive::default();
        solo.add(&[b"c"], images(7)).unwrap();
        assert!(one < solo.write().unwrap().len() + 200, "the second locale's images are not stored again");
        let file = a.write().unwrap();
        for cut in [0, 10, 60, file.len() - 1] {
            assert!(Archive::parse(&file[..cut]).is_err(), "cut at {cut}");
        }
        let mut bad = file.as_slice().to_vec();
        bad[16..20].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(Archive::parse(&bad).unwrap_err(), ArchiveError::Damaged);
    }
}
