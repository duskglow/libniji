//! `LC_CTYPE` (spec Appendix D.1.3, D.1.2): classes, case and other maps, widths, digits and the
//! transliteration table, from the source statements (POSIX XBD 7.3.1) and the charmap.
//!
//! Source forms: a class keyword (`upper`, ..., or a name declared by `charclass`) or
//! `class "<name>";` followed by `;`-separated characters and ranges `<a>..<b>`; a map keyword
//! (`toupper`, `tolower`, a name declared by `charconv`) or `map "<name>";` followed by pairs
//! `(<a>,<b>)`; `outdigit`; and between `translit_start` and `translit_end`, entries
//! `<from> <to>;<to>...` (each side a character or a string), `include "<file>";""` and
//! `default_missing`.
//!
//! Derived classes: `alnum` is `alpha` and `digit` together with what the source lists. Only code
//! points up to U+10FFFF are tabulated.
#![allow(clippy::indexing_slicing, clippy::arithmetic_side_effects)] // dense per-code-point arrays of `LIMIT` entries, indexed by code points checked against `LIMIT`; offsets within one table

use alloc::collections::BTreeMap;

use crate::build::{File, MAGIC};
use crate::cats::mb;
use crate::charset::Charset;
use crate::lex::{self, Tok};
use crate::source::{self, Stmt};
use crate::{Error, Files, Kind, Result, TryVec};

/// One past the last code point tabulated.
pub const LIMIT: u32 = 0x11_0000;

/// The standard classes in `<ctype.h>` bit order (Appendix C.9).
pub const STANDARD_CLASSES: [&[u8]; 12] = [b"upper", b"lower", b"alpha", b"digit", b"xdigit", b"space", b"print", b"graph", b"blank", b"cntrl", b"punct", b"alnum"];

/// The item index of the first class table (after the fixed items `0..72`).
const FIRST_TABLE: u32 = 72;

/// A set of code points below [`LIMIT`].
struct Set(TryVec<u32>);

impl Set {
    fn new() -> Result<Set> {
        let mut v = TryVec::new();
        v.resize(usize::try_from(LIMIT / 32).unwrap_or(0), 0)?;
        Ok(Set(v))
    }
    fn add(&mut self, cp: u32) {
        if cp < LIMIT {
            self.0[usize::try_from(cp >> 5).unwrap_or(0)] |= 1 << (cp & 31);
        }
    }
    fn has(&self, cp: u32) -> bool {
        cp < LIMIT && self.0[usize::try_from(cp >> 5).unwrap_or(0)] >> (cp & 31) & 1 != 0
    }
    fn union(&mut self, other: &Set) {
        for (a, b) in self.0.iter_mut().zip(other.0.iter()) {
            *a |= *b;
        }
    }
}

/// A transliteration entry.
#[derive(Debug)]
pub struct Translit {
    /// The characters replaced.
    pub from: TryVec<u32>,
    /// The alternatives, in order.
    pub to: TryVec<TryVec<u32>>,
    /// Per alternative: written as a string or word (text in the charmap), not a symbol.
    pub text: TryVec<bool>,
}

/// A parsed `LC_CTYPE`.
pub struct Ctype {
    classes: TryVec<(TryVec<u8>, Set)>,
    maps: TryVec<(TryVec<u8>, BTreeMap<u32, u32>)>,
    outdigits: Option<TryVec<u32>>,
    translit: TryVec<Translit>,
    default_missing: Option<TryVec<u32>>,
    /// For each character with a one-character entry, the entry a value of another category uses
    /// when the charmap lacks the character (see [`Pick`]).
    pick: BTreeMap<u32, usize>,
}

/// The compile-time choice among a file's transliteration entries, probed (LOCALEDEF-TRANSLIT):
/// the file's own entries first, the later of two for one character winning, then its
/// `include`s in order, the first that has the character winning, each resolved the same way.
/// The runtime table ([`write_translit`]) keeps its own order; only a one-character from-string
/// is looked up (the reference does not accept a longer one in this form).
#[derive(Default)]
struct Pick {
    own: BTreeMap<u32, usize>,
    incs: TryVec<BTreeMap<u32, usize>>,
}

impl Pick {
    fn finish(self) -> BTreeMap<u32, usize> {
        let mut m = self.own;
        for inc in self.incs.into_inner() {
            for (k, v) in inc {
                m.entry(k).or_insert(v);
            }
        }
        m
    }
}

impl Ctype {
    /// Append to `out` the bytes for `ch`, a character the charmap `cs` lacks, through the entry
    /// [`Pick`] chose: its first alternative that is not empty and that `cs` encodes whole (an
    /// alternative is not transliterated again; `default_missing` is not used here). `false`
    /// (and `out` unchanged): no such alternative. Probed (LOCALEDEF-TRANSLIT).
    pub fn transliterate(&self, cs: &Charset, ch: u32, out: &mut TryVec<u8>) -> Result<bool> {
        let Some(e) = self.pick.get(&ch).and_then(|&i| self.translit.get(i)) else { return Ok(false) };
        let mut one = TryVec::new();
        'alts: for alt in e.to.iter().filter(|a| !a.is_empty()) {
            one.clear();
            for &c in alt.iter() {
                if !cs.encode(c, &mut one)? {
                    continue 'alts;
                }
            }
            out.extend_from_slice(&one)?;
            return Ok(true);
        }
        Ok(false)
    }

    fn class_index(&mut self, name: &[u8]) -> Result<usize> {
        if let Some(i) = self.classes.iter().position(|(n, _)| n.as_slice() == name) {
            return Ok(i);
        }
        self.classes.push((TryVec::from_slice(name)?, Set::new()?))?;
        Ok(self.classes.len() - 1)
    }

    fn map_index(&mut self, name: &[u8]) -> Result<usize> {
        if let Some(i) = self.maps.iter().position(|(n, _)| n.as_slice() == name) {
            return Ok(i);
        }
        self.maps.push((TryVec::from_slice(name)?, BTreeMap::new()))?;
        Ok(self.maps.len() - 1)
    }
}

/// Parse the statements of `LC_CTYPE`, loading `include`d transliteration files through `files`.
pub fn parse(stmts: &[Stmt], files: &mut dyn Files) -> Result<Ctype> {
    let mut c = Ctype { classes: TryVec::new(), maps: TryVec::new(), outdigits: None, translit: TryVec::new(), default_missing: None, pick: BTreeMap::new() };
    let mut pick = Pick::default();
    for name in STANDARD_CLASSES {
        c.class_index(name)?;
    }
    for name in [&b"toupper"[..], b"tolower"] {
        c.map_index(name)?;
    }
    let mut declared_classes: TryVec<TryVec<u8>> = TryVec::new();
    let mut declared_maps: TryVec<TryVec<u8>> = TryVec::new();
    let mut in_translit = false;
    for s in stmts {
        let kw = s.keyword();
        if in_translit {
            if kw == b"translit_end" {
                in_translit = false;
            } else {
                translit_stmt(&mut c, s, files, 0, &mut pick)?;
            }
            continue;
        }
        match kw {
            b"translit_start" => in_translit = true,
            b"charclass" => {
                for t in s.args() {
                    if let Tok::Word(w) = t {
                        // Declaration order is table order.
                        c.class_index(w)?;
                        declared_classes.push(w.try_clone()?)?;
                    }
                }
            }
            b"charconv" => {
                for t in s.args() {
                    if let Tok::Word(w) = t {
                        c.map_index(w)?;
                        declared_maps.push(w.try_clone()?)?;
                    }
                }
            }
            b"class" => {
                let (name, rest) = named(s)?;
                let i = c.class_index(&name)?;
                add_list(&mut c.classes[i].1, rest, s.no)?;
            }
            b"map" => {
                let (name, rest) = named(s)?;
                let i = c.map_index(&name)?;
                add_pairs(&mut c.maps[i].1, rest, s.no)?;
            }
            b"outdigit" => {
                let mut set = TryVec::new();
                for_each_char(s.args(), s.no, &mut |cp| set.push(cp).map_err(Error::from))?;
                c.outdigits = Some(set);
            }
            b"toupper" | b"tolower" => {
                let i = c.map_index(kw)?;
                add_pairs(&mut c.maps[i].1, s.args(), s.no)?;
            }
            _ if STANDARD_CLASSES.contains(&kw) || declared_classes.iter().any(|d| d.as_slice() == kw) => {
                let i = c.class_index(kw)?;
                add_list(&mut c.classes[i].1, s.args(), s.no)?;
            }
            _ if declared_maps.iter().any(|d| d.as_slice() == kw) => {
                let i = c.map_index(kw)?;
                add_pairs(&mut c.maps[i].1, s.args(), s.no)?;
            }
            _ => return Err(Error::syntax(s.no, "unknown LC_CTYPE keyword")),
        }
    }
    // alnum = alpha + digit.
    let (alpha, digit, alnum) = (2usize, 3usize, 11usize);
    let mut both = Set::new()?;
    both.union(&c.classes[alpha].1);
    both.union(&c.classes[digit].1);
    c.classes[alnum].1.union(&both);
    c.pick = pick.finish();
    Ok(c)
}

/// `class "<name>"; <list>` or `map "<name>"; <pairs>`: the name and the rest.
fn named(s: &Stmt) -> Result<(TryVec<u8>, &[Tok])> {
    let a = s.args();
    let Some(Tok::Str(raw)) = a.first() else { return Err(Error::syntax(s.no, "a class or map name must be a string")) };
    let name = source::ascii_name(&lex::decode_string(raw, s.esc, s.no)?).ok_or(Error::syntax(s.no, "a class or map name is not ASCII"))?;
    let rest = a.get(1..).unwrap_or(&[]);
    Ok((name, rest.strip_prefix(&[Tok::Semi]).unwrap_or(rest)))
}

/// Call `f` for every character of a `;`-separated list of characters and ranges.
fn for_each_char(toks: &[Tok], no: u32, f: &mut dyn FnMut(u32) -> Result<()>) -> Result<()> {
    let mut i = 0;
    while i < toks.len() {
        match &toks[i] {
            Tok::Semi => i += 1,
            Tok::Sym(a) => {
                // A name the charmap does not define (left unresolved by `resolve_names`) is
                // no character of this locale: skipped, alone or as a range's end (spec §31.8).
                let a = lex::ucs_name(a);
                let dots = toks.get(i + 1).and_then(Tok::word).is_some_and(|w| w == b".." || w == b"...");
                if dots {
                    let Some(Tok::Sym(b)) = toks.get(i + 2) else { return Err(Error::syntax(no, "a range needs an end")) };
                    let (Some(a), Some(b)) = (a, lex::ucs_name(b)) else {
                        i += 3;
                        continue;
                    };
                    if b < a {
                        return Err(Error::syntax(no, "a range ends before it starts"));
                    }
                    for cp in a..=b.min(LIMIT - 1) {
                        f(cp)?;
                    }
                    i += 3;
                } else {
                    if let Some(a) = a {
                        f(a)?;
                    }
                    i += 1;
                }
            }
            Tok::Word(w) => {
                // A literal character.
                let s = lex::decode_string(w, b'\0', no)?;
                for &cp in s.iter() {
                    f(cp)?;
                }
                i += 1;
            }
            _ => return Err(Error::syntax(no, "unexpected token in a character list")),
        }
    }
    Ok(())
}

fn add_list(set: &mut Set, toks: &[Tok], no: u32) -> Result<()> {
    for_each_char(toks, no, &mut |cp| {
        set.add(cp);
        Ok(())
    })
}

/// Pairs `(<a>,<b>)` separated by `;`.
fn add_pairs(map: &mut BTreeMap<u32, u32>, toks: &[Tok], no: u32) -> Result<()> {
    let mut i = 0;
    while i < toks.len() {
        match (&toks[i], toks.get(i + 1), toks.get(i + 2), toks.get(i + 3), toks.get(i + 4)) {
            (Tok::Semi, ..) => i += 1,
            (Tok::Open, Some(Tok::Sym(a)), Some(Tok::Comma), Some(Tok::Sym(b)), Some(Tok::Close)) => {
                // A pair naming a character the charmap does not define is skipped.
                if let (Some(a), Some(b)) = (lex::ucs_name(a), lex::ucs_name(b)) {
                    map.insert(a, b);
                }
                i += 5;
            }
            _ => return Err(Error::syntax(no, "a map entry is not (<a>,<b>)")),
        }
    }
    Ok(())
}

/// One side of a transliteration entry: a character, a string, or a literal word.
fn side(t: &Tok, esc: u8, no: u32) -> Result<TryVec<u32>> {
    match t {
        Tok::Sym(n) => Ok(TryVec::from_slice(&[lex::ucs_name(n).ok_or(Error::syntax(no, "unknown symbolic name"))?])?),
        Tok::Str(raw) => lex::decode_string(raw, esc, no),
        Tok::Word(w) => lex::decode_string(w, b'\0', no),
        _ => Err(Error::syntax(no, "a transliteration side is not a character or string")),
    }
}

fn translit_stmt(c: &mut Ctype, s: &Stmt, files: &mut dyn Files, depth: u32, pick: &mut Pick) -> Result<()> {
    match s.keyword() {
        b"include" => {
            let Some(Tok::Str(raw)) = s.args().first() else { return Err(Error::syntax(s.no, "include needs a file name")) };
            if depth >= source::MAX_COPY_DEPTH {
                return Err(Error::syntax(s.no, "include nests too deep"));
            }
            let name = source::ascii_name(&lex::decode_string(raw, s.esc, s.no)?).ok_or(Error::syntax(s.no, "include names a file that is not ASCII"))?;
            let text = files.read(Kind::Locale, &name)?;
            let mut other = source::parse(&text, files)?;
            let stmts = other.cats[0].take().ok_or(Error::syntax(s.no, "an included file has no LC_CTYPE"))?;
            let mut inside = false;
            let mut sub = Pick::default();
            for t in stmts.iter() {
                match t.keyword() {
                    b"translit_start" => inside = true,
                    b"translit_end" => inside = false,
                    _ if inside => translit_stmt(c, t, files, depth + 1, &mut sub)?,
                    _ => {}
                }
            }
            pick.incs.push(sub.finish())?;
            Ok(())
        }
        b"default_missing" => {
            let mut v = TryVec::new();
            for t in s.args() {
                if !matches!(t, Tok::Semi) {
                    v.extend_from_slice(&side(t, s.esc, s.no)?)?;
                }
            }
            c.default_missing = Some(v);
            Ok(())
        }
        b"translit_ignore" => Ok(()),
        _ => {
            let Some(first) = s.toks.first() else { return Ok(()) };
            let from = side(first, s.esc, s.no)?;
            let (mut to, mut text) = (TryVec::new(), TryVec::new());
            for t in s.args() {
                if !matches!(t, Tok::Semi) {
                    to.push(side(t, s.esc, s.no)?)?;
                    text.push(!matches!(t, Tok::Sym(_)))?;
                }
            }
            if let [one] = from.as_slice() {
                pick.own.insert(*one, c.translit.len());
            }
            c.translit.push(Translit { from, to, text })?;
            Ok(())
        }
    }
}

/// The class bit of standard class `k` in the 16-bit `__ctype_b` mask (Appendix C.9).
const fn isbit(k: u32) -> u16 {
    if k < 8 { (1 << k) << 8 } else { (1 << k) >> 8 }
}

/// Write `LC_CTYPE` (`defaults`: the built-in defaults of an undefined `LC_CTYPE`, D-31.24).
pub fn write(c: &Ctype, cs: &Charset, defaults: bool) -> Result<TryVec<u8>> {
    let mut f = File::new();
    let upper = |cp: u32| c.maps[0].1.get(&cp).copied().unwrap_or(cp);
    let lower = |cp: u32| c.maps[1].1.get(&cp).copied().unwrap_or(cp);
    let byte_of = |cp: u32| -> Option<u8> {
        let mut v = TryVec::new();
        match cs.encode(cp, &mut v) {
            Ok(true) if v.len() == 1 => v.first().copied(),
            _ => None,
        }
    };
    // The byte tables: entry b + 128 for b in −128…255; −128…−1 repeat bytes 128…255, but −1 (EOF) is 0.
    let mut class = [0u16; 384];
    let mut toupper = [0i32; 384];
    let mut tolower = [0i32; 384];
    let mut class32 = [0u32; 256];
    let mut toupper32 = [0u32; 256];
    let mut tolower32 = [0u32; 256];
    // The 16-bit class mask of a code point.
    let mask = |cp: u32| {
        c.classes.iter().take(12).enumerate().filter(|(_, (_, set))| set.has(cp)).fold(0u16, |m, (k, _)| m | isbit(u32::try_from(k).unwrap_or(0)))
    };
    for b in 0..256u32 {
        let ch = cs.byte_char(u8::try_from(b).unwrap_or(0));
        let bits = ch.map_or(0, mask);
        let bu = ch.and_then(|cp| byte_of(upper(cp))).map_or(b, u32::from);
        let bl = ch.and_then(|cp| byte_of(lower(cp))).map_or(b, u32::from);
        let i = usize::try_from(b).unwrap_or(0);
        class[i + 128] = bits;
        toupper[i + 128] = i32::try_from(bu).unwrap_or(0);
        tolower[i + 128] = i32::try_from(bl).unwrap_or(0);
        // The 32-bit tables are indexed by code point (U+0000…U+00FF), whatever the charmap:
        // the mask in the high half, the maps' values.
        class32[i] = u32::from(mask(b)) << 16;
        toupper32[i] = upper(b);
        tolower32[i] = lower(b);
        // −128…−2 repeat bytes 128…254 (the mapped byte as an unsigned value).
        if b >= 128 && b != 255 {
            class[i - 128] = bits;
            toupper[i - 128] = i32::try_from(bu).unwrap_or(0);
            tolower[i - 128] = i32::try_from(bl).unwrap_or(0);
        }
    }
    toupper[127] = -1;
    tolower[127] = -1;
    let mut v = TryVec::new();
    for w in class {
        v.extend_from_slice(&w.to_le_bytes())?;
    }
    f.raw(true, v)?; // 0 CLASS
    f.raw(true, i32s(&toupper)?)?; // 1 TOUPPER
    f.raw(true, TryVec::new())?; // 2 GAP1
    f.raw(true, i32s(&tolower)?)?; // 3 TOLOWER
    f.raw(true, TryVec::new())?; // 4 GAP2
    f.words(&class32)?; // 5 CLASS32
    for _ in 6..10 {
        f.raw(true, TryVec::new())?; // GAP3..GAP6
    }
    // 10 CLASS_NAMES, 11 MAP_NAMES: NUL-ended names, an empty name ending the list; word-aligned items.
    f.raw(true, name_list(c.classes.iter().map(|(n, _)| n.as_slice()))?)?;
    f.raw(true, name_list(c.maps.iter().map(|(n, _)| n.as_slice()))?)?;
    // 12 WIDTH: printable characters the charmap has; others are absent (−1).
    let print = &c.classes[6].1;
    // The null character has width 0 (POSIX wcwidth).
    f.raw(true, table_u8(&|cp| if cp == 0 { Some(0) } else if print.has(cp) && cs.has(cp) { Some(cs.width(cp)) } else { None })?)?;
    f.word(cs.mb_cur_max())?; // 13
    f.string(cs.name())?; // 14 CODESET
    f.words(&toupper32)?; // 15
    f.words(&tolower32)?; // 16
    let nclasses = u32::try_from(c.classes.len()).unwrap_or(0);
    f.word(FIRST_TABLE)?; // 17 CLASS_OFFSET
    f.word(FIRST_TABLE + nclasses)?; // 18 MAP_OFFSET
    // 19..60: digits. Input digits are the ASCII ones; output digits `outdigit` or ASCII.
    f.word(1)?; // 19 INDIGITS_MB_LEN
    for d in 0..10u32 {
        f.string(&mb(cs, &[u32::from(b'0') + d])?)?;
    }
    if defaults {
        // The defaults have no wide input digits: length 0, ten empty items (D-31.24).
        f.word(0)?; // 30
        for _ in 0..10 {
            f.raw(true, TryVec::new())?;
        }
    } else {
        f.word(1)?; // 30 INDIGITS_WC_LEN
        // 31..40: one wide character each, no terminator.
        for d in 0..10u32 {
            f.word(u32::from(b'0') + d)?;
        }
    }
    let out: [u32; 10] = core::array::from_fn(|d| c.outdigits.as_ref().and_then(|o| o.get(d).copied()).unwrap_or(u32::from(b'0') + u32::try_from(d).unwrap_or(0)));
    for &d in &out {
        f.string(&mb(cs, &[d])?)?;
    }
    for &d in &out {
        f.word(d)?;
    }
    write_translit(&mut f, c, cs)?; // 61..65
    let missing = c.default_missing.as_ref();
    f.word(missing.map_or(0, |m| u32::try_from(m.len()).unwrap_or(0)))?; // 66
    match missing {
        Some(m) => f.words(m)?, // 67: the wide characters, no terminator
        None => f.raw(true, TryVec::new())?,
    }
    f.word(0)?; // 68 TRANSLIT_IGNORE_LEN
    f.raw(true, TryVec::new())?; // 69 TRANSLIT_IGNORE
    // 70 MAP_TO_NONASCII: an ASCII character maps outside ASCII. 71 NONASCII_CASE: the byte case
    // tables are not ASCII's (a byte above 0x7f has a case mapping, or an ASCII letter lacks its
    // ASCII one), so ASCII-only case folding of bytes would be wrong.
    let to_nonascii = c.maps.iter().take(2).any(|(_, m)| m.iter().any(|(&a, &b)| a < 0x80 && b >= 0x80));
    f.word(u32::from(to_nonascii))?;
    let nonascii_case = (0..256usize).any(|b| {
        let a = u8::try_from(b).unwrap_or(0);
        let (u, l) = (toupper[b + 128], tolower[b + 128]);
        u != i32::from(a.to_ascii_uppercase()) || l != i32::from(a.to_ascii_lowercase())
    });
    f.word(u32::from(nonascii_case))?;
    // Each class table is preceded by the class over the charmap's single bytes: 256 bits, bit b
    // for byte b (outside every item; the reader does not use it).
    for (_, set) in c.classes.iter() {
        let mut bytes = [0u8; 32];
        for b in 0..=255u8 {
            if cs.byte_char(b).is_some_and(|cp| set.has(cp)) {
                bytes[usize::from(b >> 3)] |= 1 << (b & 7);
            }
        }
        f.extend_last(&bytes)?;
        f.raw(true, table_bits(set)?)?;
    }
    for (_, m) in c.maps.iter() {
        f.raw(true, table_i32(&|cp| m.get(&cp).map(|&t| i32::try_from(t).unwrap_or(0).wrapping_sub(i32::try_from(cp).unwrap_or(0))).filter(|&d| d != 0))?)?;
    }
    f.finish(MAGIC[0])
}

fn name_list<'a>(names: impl Iterator<Item = &'a [u8]>) -> Result<TryVec<u8>> {
    let mut v = TryVec::new();
    for n in names {
        v.extend_from_slice(n)?;
        v.push(0)?;
    }
    v.push(0)?;
    Ok(v)
}

fn i32s(v: &[i32]) -> Result<TryVec<u8>> {
    let mut out = TryVec::with_capacity(4 * v.len())?;
    for w in v {
        out.extend_from_slice(&w.to_le_bytes())?;
    }
    Ok(out)
}

/// Items 61…65: the table size, the from-index, the from-strings, the to-index and the to-lists,
/// sorted by the from-string. The last definition of a from-string wins (statements in source
/// order, an `include` where it stands: a file's own entries after its includes, a locale's after
/// those it copies); alternatives the charmap cannot encode are left out (an entry may keep none).
fn write_translit(f: &mut File, c: &Ctype, cs: &Charset) -> Result<()> {
    let mut order: TryVec<usize> = TryVec::new();
    for i in 0..c.translit.len() {
        order.push(i)?;
    }
    order.sort_by(|&a, &b| c.translit[a].from.as_slice().cmp(c.translit[b].from.as_slice()).then(a.cmp(&b)));
    let mut kept: TryVec<usize> = TryVec::new();
    for &i in order.iter() {
        if let Some(k) = kept.last_mut().filter(|k| c.translit[**k].from.as_slice() == c.translit[i].from.as_slice()) {
            *k = i;
            continue;
        }
        kept.push(i)?;
    }
    let mut scratch = TryVec::new();
    let mut encodable = |s: &[u32]| -> Result<bool> {
        for &ch in s {
            scratch.clear();
            if !cs.encode(ch, &mut scratch)? {
                return Ok(false);
            }
        }
        Ok(true)
    };
    let (mut from_idx, mut from_tbl, mut to_idx, mut to_tbl) = (TryVec::new(), TryVec::new(), TryVec::new(), TryVec::new());
    let mut seen: BTreeMap<alloc::vec::Vec<u32>, u32> = BTreeMap::new();
    for &i in kept.iter() {
        let e = &c.translit[i];
        from_idx.push(u32::try_from(from_tbl.len()).unwrap_or(0))?;
        from_tbl.extend_from_slice(&e.from)?;
        from_tbl.push(0)?;
        let mut one = TryVec::new();
        for (alt, &text) in e.to.iter().zip(e.text.iter()) {
            // A string must be text in the charmap; a symbol is kept as its code point.
            if text && !encodable(alt)? {
                continue;
            }
            one.extend_from_slice(alt)?;
            one.push(0)?;
        }
        one.push(0)?;
        // Identical alternative lists are stored once.
        let at = match seen.get(one.as_slice()) {
            Some(&at) => at,
            None => {
                let at = u32::try_from(to_tbl.len()).unwrap_or(0);
                to_tbl.extend_from_slice(&one)?;
                seen.insert(one.into_inner(), at);
                at
            }
        };
        to_idx.push(at)?;
    }
    f.word(u32::try_from(kept.len()).unwrap_or(0))?;
    f.words(&from_idx)?;
    f.words(&from_tbl)?;
    f.words(&to_idx)?;
    f.words(&to_tbl)
}

/// Table geometry: level-3 blocks of 64 code points, level-2 blocks of 32 level-3 blocks.
const SHIFT2: u32 = 6;
const SHIFT1: u32 = 11;
const MASK2: u32 = (1 << (SHIFT1 - SHIFT2)) - 1;

/// A three-level table (D.1.2) whose level-3 block for the 64 code points from `base` is
/// `block(base)` (`None`: absent, all default); identical blocks are stored once.
fn table3(mask3: u32, block: &dyn Fn(u32) -> Result<Option<TryVec<u8>>>) -> Result<TryVec<u8>> {
    let nblocks1 = LIMIT >> SHIFT1;
    // Level-3 blocks per level-1 entry.
    let mut l2s: TryVec<Option<[u32; 32]>> = TryVec::new();
    let mut l3_data: TryVec<u8> = TryVec::new();
    let mut seen: BTreeMap<alloc::vec::Vec<u8>, u32> = BTreeMap::new();
    for i1 in 0..nblocks1 {
        let mut l2 = [u32::MAX; 32];
        let mut any = false;
        for i2 in 0..=MASK2 {
            let base = (i1 << SHIFT1) | (i2 << SHIFT2);
            if let Some(b) = block(base)? {
                let at = match seen.get(b.as_slice()) {
                    Some(&at) => at,
                    None => {
                        let at = u32::try_from(l3_data.len()).map_err(|_| Error::syntax(0, "a table over 4 GiB"))?;
                        l3_data.extend_from_slice(&b)?;
                        seen.insert(b.into_inner(), at);
                        at
                    }
                };
                l2[usize::try_from(i2).unwrap_or(0)] = at;
                any = true;
            }
        }
        l2s.push(any.then_some(l2))?;
    }
    let bound = l2s.iter().rposition(Option::is_some).map_or(0, |p| p + 1);
    let header = 20 + 4 * bound;
    let n2 = l2s.iter().take(bound).filter(|e| e.is_some()).count();
    let l3_start = header + n2 * 4 * 32;
    let mut out: TryVec<u8> = TryVec::new();
    for w in [SHIFT1, u32::try_from(bound).unwrap_or(0), SHIFT2, MASK2, mask3] {
        out.extend_from_slice(&w.to_le_bytes())?;
    }
    let mut next2 = header;
    for e in l2s.iter().take(bound) {
        let off = if e.is_some() {
            let o = next2;
            next2 += 4 * 32;
            o
        } else {
            0
        };
        out.extend_from_slice(&u32::try_from(off).unwrap_or(0).to_le_bytes())?;
    }
    for l2 in l2s.iter().take(bound).flatten() {
        for &at in l2 {
            let off = if at == u32::MAX { 0 } else { u32::try_from(l3_start).unwrap_or(0) + at };
            out.extend_from_slice(&off.to_le_bytes())?;
        }
    }
    out.extend_from_slice(&l3_data)?;
    Ok(out)
}

/// A bit table of `set` (level-3 blocks: two words of 32 bits).
fn table_bits(set: &Set) -> Result<TryVec<u8>> {
    table3(1, &|base| {
        let w0 = set.0[usize::try_from(base >> 5).unwrap_or(0)];
        let w1 = set.0[usize::try_from((base >> 5) + 1).unwrap_or(0)];
        if w0 == 0 && w1 == 0 {
            return Ok(None);
        }
        let mut b = TryVec::with_capacity(8)?;
        b.extend_from_slice(&w0.to_le_bytes())?;
        b.extend_from_slice(&w1.to_le_bytes())?;
        Ok(Some(b))
    })
}

/// A value table of `u32` (`absent` for an entry without a value in a block that has values).
pub(crate) fn table_u32(v: &dyn Fn(u32) -> Option<u32>, absent: u32) -> Result<TryVec<u8>> {
    table3(63, &|base| {
        if (base..base + 64).all(|cp| v(cp).is_none()) {
            return Ok(None);
        }
        let mut b = TryVec::with_capacity(256)?;
        for cp in base..base + 64 {
            b.extend_from_slice(&v(cp).unwrap_or(absent).to_le_bytes())?;
        }
        Ok(Some(b))
    })
}

/// A value table of `i32` (absent entries 0).
pub(crate) fn table_i32(v: &dyn Fn(u32) -> Option<i32>) -> Result<TryVec<u8>> {
    table3(63, &|base| {
        if (base..base + 64).all(|cp| v(cp).is_none()) {
            return Ok(None);
        }
        let mut b = TryVec::with_capacity(256)?;
        for cp in base..base + 64 {
            b.extend_from_slice(&v(cp).unwrap_or(0).to_le_bytes())?;
        }
        Ok(Some(b))
    })
}

/// A value table of bytes (absent entries 0xff).
fn table_u8(v: &dyn Fn(u32) -> Option<u8>) -> Result<TryVec<u8>> {
    table3(63, &|base| {
        if (base..base + 64).all(|cp| v(cp).is_none()) {
            return Ok(None);
        }
        let mut b = TryVec::with_capacity(64)?;
        for cp in base..base + 64 {
            b.push(v(cp).unwrap_or(0xff))?;
        }
        Ok(Some(b))
    })
}
