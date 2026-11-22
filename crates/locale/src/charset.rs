//! The charmap a locale is compiled for (`-f`; POSIX XBD 6.4 "Character Set Description File"):
//! its name, `<mb_cur_max>`, the encoding of each code point, and the `WIDTH` section
//! (`<Ua>[...<Ub>] width` lines, `WIDTH_DEFAULT`; absent: 1).
//!
//! The mappings come from `niji::charmap` (the parser `iconv_open` uses); a charmap named
//! `UTF-8` is encoded by the UTF-8 rules (RFC 3629) for every code point it lists.
#![allow(clippy::arithmetic_side_effects)] // code point and byte arithmetic bounded by u32 values of one line

use niji::charmap::Charmap;

use crate::lex::{trim, ucs_name};
use crate::{Error, Result, TryVec};

/// A parsed charmap.
#[derive(Debug)]
pub struct Charset {
    map: Charmap,
    utf8: bool,
    /// `WIDTH` ranges: first, last, width; sorted by first.
    widths: TryVec<(u32, u32, u8)>,
    width_default: u8,
}

impl Charset {
    /// Parse a charmap's text.
    pub fn parse(text: &[u8]) -> Result<Charset> {
        let map = Charmap::parse(text).map_err(|_| Error::syntax(0, "the charmap has no mappings"))?;
        let utf8 = map.name() == b"UTF-8";
        let mut cs = Charset { map, utf8, widths: TryVec::new(), width_default: 1 };
        let mut in_width = false;
        for raw in text.split(|&b| b == b'\n') {
            let line = trim(raw);
            if let Some(rest) = line.strip_prefix(b"WIDTH_DEFAULT") {
                cs.width_default = number(trim(rest)).unwrap_or(1);
                continue;
            }
            if line == b"WIDTH" {
                in_width = true;
                continue;
            }
            if line.starts_with(b"END WIDTH") {
                in_width = false;
                continue;
            }
            if !in_width {
                continue;
            }
            let mut parts = line.split(|c| c.is_ascii_whitespace()).filter(|p| !p.is_empty());
            let (Some(range), Some(w)) = (parts.next(), parts.next()) else { continue };
            let Some(w) = number(w) else { continue };
            let (a, b) = match range.iter().position(|&c| c == b'.') {
                Some(dot) => (symbol(range.get(..dot).unwrap_or(&[])), symbol(trim_dots(range.get(dot..).unwrap_or(&[])))),
                None => (symbol(range), symbol(range)),
            };
            if let (Some(a), Some(b)) = (a, b) {
                cs.widths.push((a, b, w))?;
            }
        }
        cs.widths.sort_unstable_by_key(|r| r.0);
        Ok(cs)
    }

    /// The `<code_set_name>`.
    pub fn name(&self) -> &[u8] {
        self.map.name()
    }

    /// `<mb_cur_max>`.
    pub fn mb_cur_max(&self) -> u32 {
        u32::try_from(self.map.mb_cur_max()).unwrap_or(1)
    }

    /// Whether this is the UTF-8 charmap.
    pub fn is_utf8(&self) -> bool {
        self.utf8
    }

    /// Append the bytes of `cp` to `out`; `false` (nothing appended) when the charmap has none.
    pub fn encode(&self, cp: u32, out: &mut TryVec<u8>) -> Result<bool> {
        let mut buf = [0u8; 8];
        let n = if self.utf8 { utf8(cp, &mut buf) } else { self.map.encode(cp, &mut buf) };
        match n.and_then(|n| buf.get(..n)) {
            Some(b) => {
                out.extend_from_slice(b)?;
                Ok(true)
            }
            None => Ok(false),
        }
    }

    /// Whether the charmap maps `cp`.
    pub fn has(&self, cp: u32) -> bool {
        let mut buf = [0u8; 8];
        self.map.encode(cp, &mut buf).is_some()
    }

    /// The character the single byte `b` is, if it is one by itself.
    pub fn byte_char(&self, b: u8) -> Option<u32> {
        match self.map.decode(&[b]) {
            niji::charmap::Step::Char(cp, 1) => Some(cp),
            _ => None,
        }
    }

    /// The `WIDTH` of `cp` (the default when no range lists it).
    pub fn width(&self, cp: u32) -> u8 {
        let i = self.widths.partition_point(|r| r.0 <= cp);
        match i.checked_sub(1).and_then(|i| self.widths.get(i)) {
            Some(&(_, last, w)) if cp <= last => w,
            _ => self.width_default,
        }
    }

    /// The parsed charmap.
    pub fn map(&self) -> &Charmap {
        &self.map
    }
}

fn trim_dots(b: &[u8]) -> &[u8] {
    let n = b.iter().take_while(|&&c| c == b'.').count();
    b.get(n..).unwrap_or(&[])
}

fn symbol(b: &[u8]) -> Option<u32> {
    ucs_name(b.strip_prefix(b"<")?.strip_suffix(b">")?)
}

fn number(b: &[u8]) -> Option<u8> {
    core::str::from_utf8(b).ok()?.parse().ok()
}

/// The UTF-8 bytes of `cp` (RFC 3629, and the 5- and 6-byte forms of the original definition for
/// values above U+10FFFF, which UTF-8 charmaps with `<mb_cur_max> 6` admit).
pub fn utf8(cp: u32, out: &mut [u8; 8]) -> Option<usize> {
    let n = match cp {
        0..=0x7f => {
            out[0] = u8::try_from(cp).ok()?;
            return Some(1);
        }
        0x80..=0x7ff => 2,
        0x800..=0xffff => 3,
        0x1_0000..=0x1f_ffff => 4,
        0x20_0000..=0x3ff_ffff => 5,
        0x400_0000..=0x7fff_ffff => 6,
        _ => return None,
    };
    let lead: u8 = [0, 0, 0xc0, 0xe0, 0xf0, 0xf8, 0xfc].get(n).copied()?;
    let mut v = cp;
    for i in (1..n).rev() {
        *out.get_mut(i)? = 0x80 | u8::try_from(v & 0x3f).ok()?;
        v >>= 6;
    }
    out[0] = lead | u8::try_from(v).ok()?;
    Some(n)
}
