//! The categories of strings, numbers and bytes (spec Appendix D.1.5): `LC_NUMERIC`, `LC_TIME`,
//! `LC_MONETARY`, `LC_MESSAGES`, `LC_PAPER`, `LC_NAME`, `LC_ADDRESS`, `LC_TELEPHONE`,
//! `LC_MEASUREMENT` and `LC_IDENTIFICATION`.
//!
//! Item order is the `<langinfo.h>` index order (`spec/abi/langinfo.tsv`). Values a source leaves
//! out take the value observed in the reference localedef's output (format inspection, UTIL-12):
//! an empty string, `CHAR_MAX` (127) for a byte, the `int_*` and `duo_*` monetary values the
//! corresponding national ones, the alternative month names the month names, and the `wide`
//! form of each string its code points.
#![allow(clippy::arithmetic_side_effects)] // date and count arithmetic on values of one source line, range-checked at parse

use crate::build::{File, MAGIC};
use crate::charset::Charset;
use crate::ctype::Ctype;
use core::ops::Deref;
use crate::lex::{self, Tok};
use crate::source::Stmt;
use crate::{Error, Result, TryVec};

/// `CHAR_MAX`: a byte item's "not available" (POSIX `localeconv`), written for `-1`.
pub const CHAR_MAX: u8 = 127;

/// One operand of a statement.
#[derive(Debug, PartialEq, Eq)]
pub enum Arg {
    /// A string, as code points.
    Str(TryVec<u32>),
    /// A number.
    Num(i64),
    /// A symbolic name `<Uxxxx>`: one character.
    Char(u32),
}

/// The operands of a statement, without the `;` separators.
pub fn args(s: &Stmt) -> Result<TryVec<Arg>> {
    let mut out = TryVec::new();
    for t in s.args() {
        match t {
            Tok::Semi => {}
            Tok::Str(raw) => out.push(Arg::Str(lex::decode_string(raw, s.esc, s.no)?))?,
            Tok::Sym(name) => out.push(Arg::Char(lex::ucs_name(name).ok_or(Error::syntax(s.no, "unknown symbolic name"))?))?,
            Tok::Word(w) => {
                let n = core::str::from_utf8(w).ok().and_then(|t| t.parse::<i64>().ok()).ok_or(Error::syntax(s.no, "a value is not a number or string"))?;
                out.push(Arg::Num(n))?;
            }
            _ => return Err(Error::syntax(s.no, "unexpected punctuation")),
        }
    }
    Ok(out)
}

/// The statements of one category, looked up by keyword (the last statement wins, so a
/// definition after `copy` replaces the copied one).
pub struct Body<'a> {
    stmts: &'a [Stmt],
    /// The built-in defaults of a category the source does not define (`defaults.rs`, D-31.24):
    /// the few items a source cannot state are written as the reference writes them.
    defaults: bool,
}

impl<'a> Body<'a> {
    /// The category's statements.
    pub fn new(stmts: &'a [Stmt]) -> Body<'a> {
        Body { stmts, defaults: false }
    }

    /// The statements of `defaults.rs` for a category the source does not define.
    pub fn defaults(stmts: &'a [Stmt]) -> Body<'a> {
        Body { stmts, defaults: true }
    }

    /// The last statement with this keyword.
    pub fn get(&self, kw: &[u8]) -> Option<&'a Stmt> {
        self.stmts.iter().rev().find(|s| s.keyword() == kw)
    }

    /// The operands of the last statement with this keyword (empty when absent).
    pub fn args(&self, kw: &[u8]) -> Result<TryVec<Arg>> {
        match self.get(kw) {
            Some(s) => args(s),
            None => Ok(TryVec::new()),
        }
    }

    /// The code points of the keyword's first operand: a string, a character, or a number's
    /// decimal text; `None` when absent.
    pub fn text(&self, kw: &[u8]) -> Result<Option<TryVec<u32>>> {
        Ok(match self.args(kw)?.into_inner().into_iter().next() {
            Some(Arg::Str(s)) => Some(s),
            Some(Arg::Char(c)) => Some(TryVec::from_slice(&[c])?),
            Some(Arg::Num(n)) => Some(decimal(n)?),
            None => None,
        })
    }

    /// The keyword's strings (a list operand).
    pub fn strings(&self, kw: &[u8]) -> Result<TryVec<TryVec<u32>>> {
        let mut out = TryVec::new();
        for a in self.args(kw)?.into_inner() {
            match a {
                Arg::Str(s) => out.push(s)?,
                Arg::Char(c) => out.push(TryVec::from_slice(&[c])?)?,
                Arg::Num(n) => out.push(decimal(n)?)?,
            }
        }
        Ok(out)
    }

    /// The keyword's first number.
    pub fn number(&self, kw: &[u8]) -> Result<Option<i64>> {
        Ok(self.args(kw)?.iter().find_map(|a| if let Arg::Num(n) = a { Some(*n) } else { None }))
    }

    /// The keyword's numbers.
    pub fn numbers(&self, kw: &[u8]) -> Result<TryVec<i64>> {
        let mut out = TryVec::new();
        for a in self.args(kw)?.iter() {
            if let Arg::Num(n) = a {
                out.push(*n)?;
            }
        }
        Ok(out)
    }
}

fn decimal(n: i64) -> Result<TryVec<u32>> {
    let mut digits: TryVec<u32> = TryVec::new();
    let mut v = n.unsigned_abs();
    loop {
        digits.push(u32::from(b'0') + u32::try_from(v % 10).unwrap_or(0))?;
        v /= 10;
        if v == 0 {
            break;
        }
    }
    if n < 0 {
        digits.push(u32::from(b'-'))?;
    }
    digits.reverse();
    Ok(digits)
}

/// The multibyte form of `s` in the charmap; characters it lacks are left out (`LC_CTYPE`'s own
/// digit strings).
pub fn mb(cs: &Charset, s: &[u32]) -> Result<TryVec<u8>> {
    let mut out = TryVec::new();
    for &c in s {
        cs.encode(c, &mut out)?;
    }
    Ok(out)
}

/// The charmap with the locale's `LC_CTYPE` (its transliteration table): how a value of every
/// other category becomes bytes. Probed (LOCALEDEF-TRANSLIT): a character the charmap lacks is
/// transliterated at compile time ([`Ctype::transliterate`]); a value with a character neither
/// encodes is written empty, whole, and reported ([`Enc::convert`]).
pub struct Enc<'c> {
    /// The target charmap.
    pub cs: &'c Charset,
    /// The locale's parsed `LC_CTYPE`, when it defines one that parsed.
    pub ctype: Option<&'c Ctype>,
}

impl Deref for Enc<'_> {
    type Target = Charset;

    fn deref(&self) -> &Charset {
        self.cs
    }
}

impl Enc<'_> {
    /// The bytes of `s`; `None` when one of its characters is neither in the charmap nor
    /// transliterable.
    pub fn convert(&self, s: &[u32]) -> Result<Option<TryVec<u8>>> {
        let mut out = TryVec::new();
        for &c in s {
            if !self.cs.encode(c, &mut out)? && !self.ctype.map_or(Ok(false), |t| t.transliterate(self.cs, c, &mut out))? {
                return Ok(None);
            }
        }
        Ok(Some(out))
    }

    /// The bytes of a value: [`Enc::convert`], or empty when it fails.
    pub fn mb(&self, s: &[u32]) -> Result<TryVec<u8>> {
        Ok(self.convert(s)?.unwrap_or_else(TryVec::new))
    }
}

/// A byte item's value: `-1` is `CHAR_MAX`; other values are taken modulo 256.
fn byte_of(n: i64) -> u8 {
    if n == -1 { CHAR_MAX } else { u8::try_from(n & 0xff).unwrap_or(CHAR_MAX) }
}

struct Out<'c> {
    f: File,
    cs: &'c Enc<'c>,
}

impl Out<'_> {
    fn string(&mut self, s: &[u32]) -> Result<()> {
        let b = self.cs.mb(s)?;
        self.f.string(&b)
    }

    /// A string's wide form: empty when its multibyte form is (probed, LOCALEDEF-TRANSLIT: a
    /// string the charmap cannot take is written empty in both forms).
    fn wide(&mut self, s: &[u32]) -> Result<()> {
        let ok = self.cs.convert(s)?.is_some();
        self.f.wide(if ok { s } else { &[] })
    }

    fn opt(&mut self, s: Option<&TryVec<u32>>) -> Result<()> {
        self.string(s.map_or(&[][..], |v| v))
    }

    fn codeset(&mut self) -> Result<()> {
        self.f.string(self.cs.name())
    }
}

fn text_or_empty(b: &Body<'_>, kw: &[u8]) -> Result<TryVec<u32>> {
    Ok(b.text(kw)?.unwrap_or_default())
}

/// `LC_NUMERIC` (6 items).
pub fn numeric(b: &Body<'_>, cs: &Enc<'_>) -> Result<TryVec<u8>> {
    let mut o = Out { f: File::new(), cs };
    let dp = text_or_empty(b, b"decimal_point")?;
    let ts = text_or_empty(b, b"thousands_sep")?;
    o.string(&dp)?;
    o.string(&ts)?;
    o.f.string(&grouping(&b.numbers(b"grouping")?)?)?;
    o.f.word(dp.first().copied().unwrap_or(0))?;
    o.f.word(ts.first().copied().unwrap_or(0))?;
    o.codeset()?;
    o.f.finish(MAGIC[1])
}

/// A grouping operand as bytes: each number, `-1` as `CHAR_MAX`.
fn grouping(ns: &[i64]) -> Result<TryVec<u8>> {
    let mut g = TryVec::new();
    for &n in ns {
        g.push(byte_of(n))?;
    }
    Ok(g)
}

/// `LC_MONETARY` (46 items).
pub fn monetary(b: &Body<'_>, cs: &Enc<'_>) -> Result<TryVec<u8>> {
    let mut o = Out { f: File::new(), cs };
    let int_curr = text_or_empty(b, b"int_curr_symbol")?;
    let curr = text_or_empty(b, b"currency_symbol")?;
    let dp = text_or_empty(b, b"mon_decimal_point")?;
    let ts = text_or_empty(b, b"mon_thousands_sep")?;
    o.string(&int_curr)?;
    o.string(&curr)?;
    o.string(&dp)?;
    o.string(&ts)?;
    if b.defaults {
        // The defaults' `mon_grouping`: one `CHAR_MAX` byte, no terminator (D-31.24).
        let mut g = TryVec::new();
        g.push(CHAR_MAX)?;
        o.f.raw(false, g)?;
    } else {
        o.f.string(&grouping(&b.numbers(b"mon_grouping")?)?)?;
    }
    o.string(&text_or_empty(b, b"positive_sign")?)?;
    o.string(&text_or_empty(b, b"negative_sign")?)?;
    let num = |kw: &[u8]| -> Result<u8> { Ok(b.number(kw)?.map_or(CHAR_MAX, byte_of)) };
    let national = [num(b"int_frac_digits")?, num(b"frac_digits")?, num(b"p_cs_precedes")?, num(b"p_sep_by_space")?, num(b"n_cs_precedes")?, num(b"n_sep_by_space")?, num(b"p_sign_posn")?, num(b"n_sign_posn")?];
    for v in national {
        o.f.byte(v)?;
    }
    // CRNCYSTR: the currency symbol after `-` (it precedes), `+` (it follows) or `.` (it replaces the radix character).
    let mut crncy = TryVec::new();
    if !curr.is_empty() {
        match national[2] {
            1 => crncy.push(u32::from(b'-'))?,
            0 => crncy.push(u32::from(b'+'))?,
            _ => {}
        }
        if !crncy.is_empty() {
            crncy.extend_from_slice(&curr)?;
        }
    } else if b.defaults {
        // The defaults' CRNCYSTR: `-` alone (D-31.24).
        crncy.push(u32::from(b'-'))?;
    }
    o.string(&crncy)?;
    // The international forms default to the national ones.
    let or = |kw: &[u8], d: u8| -> Result<u8> { Ok(b.number(kw)?.map_or(d, byte_of)) };
    let intl = [
        or(b"int_p_cs_precedes", national[2])?,
        or(b"int_p_sep_by_space", national[3])?,
        or(b"int_n_cs_precedes", national[4])?,
        or(b"int_n_sep_by_space", national[5])?,
        or(b"int_p_sign_posn", national[6])?,
        or(b"int_n_sign_posn", national[7])?,
    ];
    for v in intl {
        o.f.byte(v)?;
    }
    // The `duo_*` values default to the corresponding single-currency ones.
    let duo_int = b.text(b"duo_int_curr_symbol")?;
    o.string(duo_int.as_deref().unwrap_or(&int_curr))?;
    let duo = b.text(b"duo_currency_symbol")?;
    o.string(duo.as_deref().unwrap_or(&curr))?;
    let duo_bytes = [
        or(b"duo_int_frac_digits", national[0])?,
        or(b"duo_frac_digits", national[1])?,
        or(b"duo_p_cs_precedes", national[2])?,
        or(b"duo_p_sep_by_space", national[3])?,
        or(b"duo_n_cs_precedes", national[4])?,
        or(b"duo_n_sep_by_space", national[5])?,
        or(b"duo_int_p_cs_precedes", intl[0])?,
        or(b"duo_int_p_sep_by_space", intl[1])?,
        or(b"duo_int_n_cs_precedes", intl[2])?,
        or(b"duo_int_n_sep_by_space", intl[3])?,
        or(b"duo_p_sign_posn", national[6])?,
        or(b"duo_n_sign_posn", national[7])?,
        or(b"duo_int_p_sign_posn", intl[4])?,
        or(b"duo_int_n_sign_posn", intl[5])?,
    ];
    for v in duo_bytes {
        o.f.byte(v)?;
    }
    let word = |kw: &[u8], d: u32| -> Result<u32> { Ok(b.number(kw)?.and_then(|n| u32::try_from(n).ok()).unwrap_or(d)) };
    o.f.word(word(b"uno_valid_from", 10101)?)?;
    o.f.word(word(b"uno_valid_to", 99_991_231)?)?;
    o.f.word(word(b"duo_valid_from", 10101)?)?;
    o.f.word(word(b"duo_valid_to", 99_991_231)?)?;
    let rate = b.numbers(b"conversion_rate")?;
    let r = |i: usize| rate.get(i).and_then(|&n| u32::try_from(n).ok()).unwrap_or(1);
    o.f.words(&[r(0), r(1)])?;
    o.f.word(dp.first().copied().unwrap_or(0))?;
    o.f.word(ts.first().copied().unwrap_or(0))?;
    o.codeset()?;
    o.f.finish(MAGIC[4])
}

/// `LC_MESSAGES` (5 items).
pub fn messages(b: &Body<'_>, cs: &Enc<'_>) -> Result<TryVec<u8>> {
    let mut o = Out { f: File::new(), cs };
    for kw in [&b"yesexpr"[..], b"noexpr", b"yesstr", b"nostr"] {
        o.opt(b.text(kw)?.as_ref())?;
    }
    o.codeset()?;
    o.f.finish(MAGIC[5])
}

/// `LC_PAPER` (3 items).
pub fn paper(b: &Body<'_>, cs: &Enc<'_>) -> Result<TryVec<u8>> {
    let mut o = Out { f: File::new(), cs };
    for kw in [&b"height"[..], b"width"] {
        o.f.word(b.number(kw)?.and_then(|n| u32::try_from(n).ok()).unwrap_or(0))?;
    }
    o.codeset()?;
    o.f.finish(MAGIC[7])
}

/// `LC_NAME` (7 items).
pub fn name(b: &Body<'_>, cs: &Enc<'_>) -> Result<TryVec<u8>> {
    let mut o = Out { f: File::new(), cs };
    for kw in [&b"name_fmt"[..], b"name_gen", b"name_mr", b"name_mrs", b"name_miss", b"name_ms"] {
        o.opt(b.text(kw)?.as_ref())?;
    }
    o.codeset()?;
    o.f.finish(MAGIC[8])
}

/// `LC_ADDRESS` (13 items).
pub fn address(b: &Body<'_>, cs: &Enc<'_>) -> Result<TryVec<u8>> {
    let mut o = Out { f: File::new(), cs };
    for kw in [&b"postal_fmt"[..], b"country_name", b"country_post", b"country_ab2", b"country_ab3", b"country_car"] {
        o.opt(b.text(kw)?.as_ref())?;
    }
    o.f.word(b.number(b"country_num")?.and_then(|n| u32::try_from(n).ok()).unwrap_or(0))?;
    for kw in [&b"country_isbn"[..], b"lang_name", b"lang_ab", b"lang_term", b"lang_lib"] {
        o.opt(b.text(kw)?.as_ref())?;
    }
    o.codeset()?;
    o.f.finish(MAGIC[9])
}

/// `LC_TELEPHONE` (5 items).
pub fn telephone(b: &Body<'_>, cs: &Enc<'_>) -> Result<TryVec<u8>> {
    let mut o = Out { f: File::new(), cs };
    for kw in [&b"tel_int_fmt"[..], b"tel_dom_fmt", b"int_select", b"int_prefix"] {
        o.opt(b.text(kw)?.as_ref())?;
    }
    o.codeset()?;
    o.f.finish(MAGIC[10])
}

/// `LC_MEASUREMENT` (2 items).
pub fn measurement(b: &Body<'_>, cs: &Enc<'_>) -> Result<TryVec<u8>> {
    let mut o = Out { f: File::new(), cs };
    o.f.byte(b.number(b"measurement")?.map_or(CHAR_MAX, byte_of))?;
    o.codeset()?;
    o.f.finish(MAGIC[11])
}

/// `LC_IDENTIFICATION` (16 items). `category` lines are `"<standard>";LC_<NAME>`: the
/// standards are stored in category-number order, one string per category but `LC_ALL`.
pub fn identification(b: &Body<'_>, cs: &Enc<'_>) -> Result<TryVec<u8>> {
    let mut o = Out { f: File::new(), cs };
    for kw in [
        &b"title"[..], b"source", b"address", b"contact", b"email", b"tel", b"fax", b"language", b"territory", b"audience", b"application", b"abbreviation", b"revision", b"date",
    ] {
        o.opt(b.text(kw)?.as_ref())?;
    }
    let mut per: [Option<TryVec<u8>>; 13] = Default::default();
    for s in b.stmts.iter().filter(|s| s.keyword() == b"category") {
        let (Some(Tok::Str(raw)), Some(cat)) = (s.args().first(), s.args().iter().rev().find_map(Tok::word)) else {
            return Err(Error::syntax(s.no, "category needs a standard and a category name"));
        };
        let c = crate::source::number(cat).ok_or(Error::syntax(s.no, "category names no category"))?;
        let text = lex::decode_string(raw, s.esc, s.no)?;
        if let Some(slot) = per.get_mut(c) {
            *slot = Some(cs.mb(&text)?);
        }
    }
    o.f.strings(per.iter().enumerate().filter(|&(c, _)| c != 6).map(|(_, v)| v.as_deref().unwrap_or(&[])))?;
    o.codeset()?;
    o.f.finish(MAGIC[12])
}

/// The number of alternative digits `alt_digits` stores (the reference's fixed count).
pub const ALT_DIGITS: usize = 100;

/// `LC_TIME` (159 items).
pub fn time(b: &Body<'_>, cs: &Enc<'_>) -> Result<TryVec<u8>> {
    let mut o = Out { f: File::new(), cs };
    let list = |kw: &[u8], n: usize| -> Result<TryVec<TryVec<u32>>> {
        let mut v = b.strings(kw)?;
        v.truncate(n);
        while v.len() < n {
            v.push(TryVec::new())?;
        }
        Ok(v)
    };
    let abday = list(b"abday", 7)?;
    let day = list(b"day", 7)?;
    let abmon = list(b"abmon", 12)?;
    let mon = list(b"mon", 12)?;
    let am_pm = list(b"am_pm", 2)?;
    let fmts = [text_or_empty(b, b"d_t_fmt")?, text_or_empty(b, b"d_fmt")?, text_or_empty(b, b"t_fmt")?, text_or_empty(b, b"t_fmt_ampm")?];
    for s in abday.iter().chain(day.iter()).chain(abmon.iter()).chain(mon.iter()).chain(am_pm.iter()).chain(fmts.iter()) {
        o.string(s)?;
    }
    let eras = b.strings(b"era")?;
    let mut era_mb = TryVec::new();
    for e in eras.iter() {
        era_mb.push(cs.mb(e)?)?;
    }
    o.f.strings(era_mb.iter().map(|v| v.as_slice()))?;
    let era_year = text_or_empty(b, b"era_year")?;
    let era_d_fmt = text_or_empty(b, b"era_d_fmt")?;
    o.string(&era_year)?;
    o.string(&era_d_fmt)?;
    let mut alt = b.strings(b"alt_digits")?;
    alt.truncate(ALT_DIGITS);
    while alt.len() < ALT_DIGITS {
        alt.push(TryVec::new())?;
    }
    let mut alt_mb = TryVec::new();
    for a in alt.iter() {
        alt_mb.push(cs.mb(a)?)?;
    }
    o.f.strings(alt_mb.iter().map(|v| v.as_slice()))?;
    let era_d_t_fmt = text_or_empty(b, b"era_d_t_fmt")?;
    let era_t_fmt = text_or_empty(b, b"era_t_fmt")?;
    o.string(&era_d_t_fmt)?;
    o.string(&era_t_fmt)?;
    o.f.word(u32::try_from(eras.len()).unwrap_or(0))?;
    o.f.raw(true, era_entries(&eras, cs, b.get(b"era").map_or(0, |s| s.no))?)?;
    // The defaults write these wide strings empty up to `t_fmt` (`t_fmt_ampm`'s is kept; D-31.24).
    let names = abday.len() + day.len() + abmon.len() + mon.len() + am_pm.len() + 3;
    for (i, s) in abday.iter().chain(day.iter()).chain(abmon.iter()).chain(mon.iter()).chain(am_pm.iter()).chain(fmts.iter()).enumerate() {
        o.wide(if b.defaults && i < names { &[] } else { s })?;
    }
    o.wide(&era_year)?;
    o.wide(&era_d_fmt)?;
    o.f.wides(alt.iter().map(|v| v.as_slice()))?;
    o.wide(&era_d_t_fmt)?;
    o.wide(&era_t_fmt)?;
    // week: ndays; 1stday (a date, a word); 1stweek.
    let week = b.numbers(b"week")?;
    let wk = |i: usize, d: i64| week.get(i).copied().unwrap_or(d);
    o.f.byte(byte_of(wk(0, 7)))?;
    o.f.word(u32::try_from(wk(1, 19_971_130)).unwrap_or(0))?;
    o.f.byte(byte_of(wk(2, 4)))?;
    let small = |kw: &[u8], d: i64| -> Result<u8> { Ok(byte_of(b.number(kw)?.unwrap_or(d))) };
    o.f.byte(small(b"first_weekday", 1)?)?;
    o.f.byte(small(b"first_workday", 2)?)?;
    o.f.byte(small(b"cal_direction", 1)?)?;
    o.string(&text_or_empty(b, b"timezone")?)?;
    let date_fmt = text_or_empty(b, b"date_fmt")?;
    o.string(&date_fmt)?;
    o.wide(&date_fmt)?;
    o.codeset()?;
    let alt_mon = if b.get(b"alt_mon").is_some() { list(b"alt_mon", 12)? } else { clone_list(&mon)? };
    let ab_alt_mon = if b.get(b"ab_alt_mon").is_some() { list(b"ab_alt_mon", 12)? } else { clone_list(&abmon)? };
    for s in alt_mon.iter() {
        o.string(s)?;
    }
    for s in alt_mon.iter() {
        o.wide(s)?;
    }
    for s in ab_alt_mon.iter() {
        o.string(s)?;
    }
    for s in ab_alt_mon.iter() {
        o.wide(s)?;
    }
    o.f.finish(MAGIC[2])
}

/// `_NL_TIME_ERA_ENTRIES`: per era (`direction:offset:start:end:name:format`), the words
/// direction (the character), offset, start year − 1900, month − 1, day, the same for the end
/// (`+*` and `-*` as `i32::MAX` and `i32::MIN` in all three), then the name and the format as
/// multibyte strings, zero bytes to a multiple of 4, and both again as wide strings (layout
/// from format inspection of the reference localedef's output, UTIL-12).
fn era_entries(eras: &[TryVec<u32>], cs: &Enc<'_>, no: u32) -> Result<TryVec<u8>> {
    let mut out = TryVec::new();
    for e in eras {
        let fields: TryVec<&[u32]> = {
            let mut v = TryVec::new();
            let mut rest: &[u32] = e;
            // Six fields; the format (the last) may itself contain ':'.
            for _ in 0..5 {
                let at = rest.iter().position(|&c| c == u32::from(b':')).ok_or(Error::syntax(no, "an era needs six fields"))?;
                v.push(rest.get(..at).unwrap_or(&[]))?;
                rest = rest.get(at + 1..).unwrap_or(&[]);
            }
            v.push(rest)?;
            v
        };
        let f = |i: usize| fields.get(i).copied().unwrap_or(&[]);
        let dir = f(0).first().copied().unwrap_or(u32::from(b'+'));
        let offset = parse_int(f(1)).ok_or(Error::syntax(no, "an era offset is not a number"))?;
        let mut words: TryVec<i32> = TryVec::new();
        words.push(i32::try_from(dir).unwrap_or(0))?;
        words.push(i32::try_from(offset).unwrap_or(0))?;
        for d in [f(2), f(3)] {
            let ymd = era_date(d).ok_or(Error::syntax(no, "an era date is not y/m/d, +* or -*"))?;
            for w in ymd {
                words.push(w)?;
            }
        }
        for w in words.iter() {
            out.extend_from_slice(&w.to_le_bytes())?;
        }
        out.extend_from_slice(&cs.mb(f(4))?)?;
        out.push(0)?;
        out.extend_from_slice(&cs.mb(f(5))?)?;
        out.push(0)?;
        while out.len() % 4 != 0 {
            out.push(0)?;
        }
        for &c in f(4).iter().chain(&[0]).chain(f(5)).chain(&[0]) {
            out.extend_from_slice(&c.to_le_bytes())?;
        }
    }
    Ok(out)
}

fn clone_list(v: &[TryVec<u32>]) -> Result<TryVec<TryVec<u32>>> {
    let mut out = TryVec::with_capacity(v.len())?;
    for s in v {
        out.push(s.try_clone()?)?;
    }
    Ok(out)
}

fn parse_int(s: &[u32]) -> Option<i64> {
    let (neg, digits) = match s.first() {
        Some(&c) if c == u32::from(b'-') => (true, s.get(1..)?),
        Some(&c) if c == u32::from(b'+') => (false, s.get(1..)?),
        _ => (false, s),
    };
    if digits.is_empty() || digits.len() > 12 {
        return None;
    }
    let mut v: i64 = 0;
    for &c in digits {
        let d = c.checked_sub(u32::from(b'0')).filter(|&d| d < 10)?;
        v = v * 10 + i64::from(d);
    }
    Some(if neg { -v } else { v })
}

fn era_date(d: &[u32]) -> Option<[i32; 3]> {
    if d == [u32::from(b'+'), u32::from(b'*')] {
        return Some([i32::MAX; 3]);
    }
    if d == [u32::from(b'-'), u32::from(b'*')] {
        return Some([i32::MIN; 3]);
    }
    let mut parts = d.split(|&c| c == u32::from(b'/'));
    let y = parse_int(parts.next()?)?;
    let m = parse_int(parts.next()?)?;
    let day = parse_int(parts.next()?)?;
    if parts.next().is_some() {
        return None;
    }
    // A negative year counts back from 1 BC (`-0001`), which is year 0 of the `tm_year` scale.
    let y = if y < 0 { y + 1 } else { y };
    Some([i32::try_from(y - 1900).ok()?, i32::try_from(m - 1).ok()?, i32::try_from(day).ok()?])
}
