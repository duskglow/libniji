//! A category file (spec Appendix D.1.1) from its items: the magic word, the item count, the
//! offsets, then the items in order. An item that holds words (a `u32`, a wide string, a table)
//! starts at a multiple of 4 bytes; the zero bytes before it end the previous item, as in the
//! files the reference localedef writes (format inspection, UTIL-12). Every other item follows the
//! previous one directly.
#![allow(clippy::arithmetic_side_effects)] // sums of item lengths of one file, each bounded by u32 (checked at the end)

use crate::{Error, Result, TryVec};

/// The category files' magic words (format inspection of the reference localedef's output, the
/// same in every file it wrote; D.1.1 names `LC_CTYPE`'s and `LC_COLLATE`'s).
pub const MAGIC: [u32; 13] = [
    0x2009_0720, // LC_CTYPE
    0x2003_1114, // LC_NUMERIC
    0x2003_1117, // LC_TIME
    0x2005_1017, // LC_COLLATE
    0x2003_1111, // LC_MONETARY
    0x2003_1110, // LC_MESSAGES
    0,           // LC_ALL: no file
    0x2003_1112, // LC_PAPER
    0x2003_111d, // LC_NAME
    0x2003_111c, // LC_ADDRESS
    0x2003_111f, // LC_TELEPHONE
    0x2003_111e, // LC_MEASUREMENT
    0x2003_1119, // LC_IDENTIFICATION
];

/// The items of one category file being built.
#[derive(Debug, Default)]
pub struct File {
    items: TryVec<(bool, TryVec<u8>)>,
}

impl File {
    /// An empty item list.
    pub fn new() -> File {
        File::default()
    }

    /// The number of items so far.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether there are no items yet.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Append an item of bytes (`aligned`: it holds words and starts at a multiple of 4).
    pub fn raw(&mut self, aligned: bool, data: TryVec<u8>) -> Result<()> {
        self.items.push((aligned, data))?;
        Ok(())
    }

    /// A NUL-terminated multibyte string.
    pub fn string(&mut self, s: &[u8]) -> Result<()> {
        let mut v = TryVec::with_capacity(s.len() + 1)?;
        v.extend_from_slice(s)?;
        v.push(0)?;
        self.raw(false, v)
    }

    /// Strings, each NUL-terminated, one after the other (a string array item).
    pub fn strings<'a>(&mut self, list: impl Iterator<Item = &'a [u8]>) -> Result<()> {
        let mut v = TryVec::new();
        for s in list {
            v.extend_from_slice(s)?;
            v.push(0)?;
        }
        self.raw(false, v)
    }

    /// A NUL-terminated wide string.
    pub fn wide(&mut self, s: &[u32]) -> Result<()> {
        let mut v = TryVec::with_capacity(4 * s.len() + 4)?;
        for &c in s.iter().chain(&[0]) {
            v.extend_from_slice(&c.to_le_bytes())?;
        }
        self.raw(true, v)
    }

    /// Wide strings, each NUL-terminated, one after the other.
    pub fn wides<'a>(&mut self, list: impl Iterator<Item = &'a [u32]>) -> Result<()> {
        let mut v = TryVec::new();
        for s in list {
            for &c in s.iter().chain(&[0]) {
                v.extend_from_slice(&c.to_le_bytes())?;
            }
        }
        self.raw(true, v)
    }

    /// A `u32` word.
    pub fn word(&mut self, w: u32) -> Result<()> {
        self.raw(true, TryVec::from_slice(&w.to_le_bytes())?)
    }

    /// Words.
    pub fn words(&mut self, ws: &[u32]) -> Result<()> {
        let mut v = TryVec::with_capacity(4 * ws.len())?;
        for w in ws {
            v.extend_from_slice(&w.to_le_bytes())?;
        }
        self.raw(true, v)
    }

    /// A byte.
    pub fn byte(&mut self, b: u8) -> Result<()> {
        self.raw(false, TryVec::from_slice(&[b])?)
    }

    /// The file: header, offsets and items, with `magic`.
    pub fn finish(self, magic: u32) -> Result<TryVec<u8>> {
        let n = self.items.len();
        let header = 8 + 4 * n;
        let mut out: TryVec<u8> = TryVec::new();
        out.resize(header, 0)?;
        put(&mut out, 0, magic);
        put(&mut out, 4, u32::try_from(n).map_err(|_| Error::syntax(0, "too many items"))?);
        for (i, (aligned, data)) in self.items.into_inner().into_iter().enumerate() {
            if aligned {
                while out.len() % 4 != 0 {
                    out.push(0)?;
                }
            }
            let at = u32::try_from(out.len()).map_err(|_| Error::syntax(0, "a category file over 4 GiB"))?;
            put(&mut out, 8 + 4 * i, at);
            out.extend_from_slice(&data)?;
        }
        Ok(out)
    }
}

fn put(v: &mut [u8], at: usize, w: u32) {
    if let Some(d) = v.get_mut(at..at + 4) {
        d.copy_from_slice(&w.to_le_bytes());
    }
}
