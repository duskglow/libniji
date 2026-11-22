//! The lexical layer of locale sources (POSIX XBD 7.3 "Locale Definition"; localedef(1)):
//! `comment_char` and `escape_char` directives, comment lines, line continuation with the escape
//! character, and the tokens of one logical line.
//!
//! Tokens: words (keywords, numbers, `IGNORE`, the ellipses `..`, `...`, and anything else that
//! is not punctuation), symbolic names `<...>`, strings `"..."` (kept raw, decoded by
//! [`decode_string`]), and the separators `;`, `,`, `(`, `)`.
#![allow(clippy::indexing_slicing, clippy::arithmetic_side_effects)] // byte positions within one line: every index is checked against its length first

use crate::{Error, Result, TryVec};

/// One token.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Tok {
    /// A word: a keyword, a number, an ellipsis, or other bare text.
    Word(TryVec<u8>),
    /// A symbolic name without its angle brackets (escapes removed).
    Sym(TryVec<u8>),
    /// A string's raw contents without the quotes (escapes kept).
    Str(TryVec<u8>),
    /// `;`.
    Semi,
    /// `,`.
    Comma,
    /// `(`.
    Open,
    /// `)`.
    Close,
}

impl Tok {
    /// The word's text, if this is a word.
    pub fn word(&self) -> Option<&[u8]> {
        match self {
            Tok::Word(w) => Some(w),
            _ => None,
        }
    }
}

/// A logical line: its first physical line number and its tokens.
#[derive(Debug)]
pub struct Line {
    /// The 1-based number of the line's first physical line.
    pub no: u32,
    /// The tokens.
    pub toks: TryVec<Tok>,
}

/// The characters a source file declares, with the defaults (`#` and `\`; POSIX).
#[derive(Clone, Copy, Debug)]
pub struct Chars {
    /// The comment character.
    pub comment: u8,
    /// The escape character.
    pub escape: u8,
}

/// Split a source file into logical lines of tokens. Comment lines and empty lines are dropped;
/// `comment_char` and `escape_char` lines change the characters for the rest of the file.
pub fn lines(text: &[u8]) -> Result<(TryVec<Line>, Chars)> {
    let mut chars = Chars { comment: b'#', escape: b'\\' };
    let mut out = TryVec::new();
    let mut logical: TryVec<u8> = TryVec::new();
    let mut start = 0u32;
    let mut no = 0u32;
    for raw in text.split(|&b| b == b'\n') {
        no = no.saturating_add(1);
        let raw = raw.strip_suffix(b"\r").unwrap_or(raw);
        if logical.is_empty() {
            start = no;
            let t = trim(raw);
            // A comment line ends at its physical end, even when it ends with the escape character.
            if t.first() == Some(&chars.comment) {
                continue;
            }
        }
        // A line ending in an unescaped escape character continues on the next one.
        let (body, cont) = continued(raw, chars.escape);
        logical.extend_from_slice(body)?;
        if cont {
            continue;
        }
        let line = core::mem::take(&mut logical);
        let t = trim(&line);
        if t.is_empty() {
            continue;
        }
        if let Some(rest) = directive(t, b"comment_char") {
            chars.comment = rest.ok_or(Error::syntax(start, "comment_char needs one character"))?;
            continue;
        }
        if let Some(rest) = directive(t, b"escape_char") {
            chars.escape = rest.ok_or(Error::syntax(start, "escape_char needs one character"))?;
            continue;
        }
        let toks = tokens(t, chars, start)?;
        if !toks.is_empty() {
            out.push(Line { no: start, toks })?;
        }
    }
    Ok((out, chars))
}

/// `Some(Some(c))` when `t` is `<name> c`, `Some(None)` when it is the name with a bad operand.
fn directive(t: &[u8], name: &[u8]) -> Option<Option<u8>> {
    let rest = t.strip_prefix(name)?;
    if !rest.first().is_some_and(|c| c.is_ascii_whitespace()) {
        return None;
    }
    let r = trim(rest);
    Some(if r.len() == 1 { r.first().copied() } else { None })
}

/// The line without a final unescaped escape character, and whether it had one.
fn continued(raw: &[u8], esc: u8) -> (&[u8], bool) {
    let t = trim_end(raw);
    // Count the escape characters at the end: an odd count ends in an unescaped one.
    let n = t.iter().rev().take_while(|&&b| b == esc).count();
    if n % 2 == 1 { (&t[..t.len() - 1], true) } else { (raw, false) }
}

fn tokens(t: &[u8], chars: Chars, no: u32) -> Result<TryVec<Tok>> {
    let esc = chars.escape;
    let mut out = TryVec::new();
    let mut i = 0;
    while i < t.len() {
        let c = t[i];
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        match c {
            b';' => {
                out.push(Tok::Semi)?;
                i += 1;
            }
            b',' => {
                out.push(Tok::Comma)?;
                i += 1;
            }
            b'(' => {
                out.push(Tok::Open)?;
                i += 1;
            }
            b')' => {
                out.push(Tok::Close)?;
                i += 1;
            }
            b'"' => {
                let mut j = i + 1;
                while j < t.len() && t[j] != b'"' {
                    j += if t[j] == esc { 2 } else { 1 };
                }
                if j >= t.len() {
                    return Err(Error::syntax(no, "unterminated string"));
                }
                out.push(Tok::Str(TryVec::from_slice(&t[i + 1..j])?))?;
                i = j + 1;
            }
            b'<' if is_symbol_start(t, i, esc) => {
                let mut name = TryVec::new();
                let mut j = i + 1;
                while j < t.len() && t[j] != b'>' {
                    if t[j] == esc && j + 1 < t.len() {
                        j += 1;
                    }
                    name.push(t[j])?;
                    j += 1;
                }
                out.push(Tok::Sym(name))?;
                i = j + 1;
            }
            _ => {
                // A bare word runs to white space, a separator or the start of a symbol or string. An escape
                // character takes the next byte literally (`/;` is a semicolon in a word).
                let mut w = TryVec::new();
                let mut j = i;
                while j < t.len() {
                    let b = t[j];
                    if b.is_ascii_whitespace() || matches!(b, b';' | b',' | b'(' | b')' | b'"') || (b == b'<' && is_symbol_start(t, j, esc)) {
                        break;
                    }
                    // `..` between two symbols, or a word followed by one: an ellipsis is a word of its own.
                    if b == b'.' && !w.is_empty() && w.iter().all(|&x| x != b'.') && t.get(j + 1) == Some(&b'.') {
                        break;
                    }
                    if b != b'.' && !w.is_empty() && w.iter().all(|&x| x == b'.') {
                        break;
                    }
                    if b == esc && j + 1 < t.len() && !t[j + 1].is_ascii_alphanumeric() {
                        w.push(t[j + 1])?;
                        j += 2;
                        continue;
                    }
                    w.push(b)?;
                    j += 1;
                }
                if j == i {
                    // A lone byte no rule takes (an unmatched `<`): a word of one byte.
                    w.push(t[i])?;
                    j = i + 1;
                }
                out.push(Tok::Word(w))?;
                i = j;
            }
        }
    }
    Ok(out)
}

/// Whether the `<` at `i` begins a symbolic name: a matching `>` follows before white space.
fn is_symbol_start(t: &[u8], i: usize, esc: u8) -> bool {
    let mut j = i + 1;
    while j < t.len() {
        match t[j] {
            b'>' => return j > i + 1,
            b if b.is_ascii_whitespace() => return false,
            b if b == esc => j += 2,
            _ => j += 1,
        }
    }
    false
}

/// The bytes without leading and trailing white space.
pub fn trim(b: &[u8]) -> &[u8] {
    let s = b.iter().position(|c| !c.is_ascii_whitespace()).unwrap_or(b.len());
    trim_end(&b[s..])
}

fn trim_end(b: &[u8]) -> &[u8] {
    let e = b.iter().rposition(|c| !c.is_ascii_whitespace()).map_or(0, |p| p + 1);
    &b[..e]
}

/// One character of a string or symbol list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Piece {
    /// A code point.
    Char(u32),
    /// A symbolic name that is not `<Uxxxx>`, by its position in the string (start, end of the name).
    Name(usize, usize),
}

/// The code point of a symbolic name `Uxxxx` or `Uxxxxxxxx` (hexadecimal), if it is one.
pub fn ucs_name(name: &[u8]) -> Option<u32> {
    let hex = name.strip_prefix(b"U")?;
    if hex.len() != 4 && hex.len() != 8 {
        return None;
    }
    let s = core::str::from_utf8(hex).ok()?;
    let v = u32::from_str_radix(s, 16).ok()?;
    (v <= 0x7fff_ffff).then_some(v)
}

/// Decode a string's raw contents into code points: `<Uxxxx>` names, the escape character
/// followed by a character (that character), `/xHH`-style byte escapes are not characters and are
/// refused, and literal UTF-8 text (the installed sources are UTF-8).
pub fn decode_string(raw: &[u8], esc: u8, no: u32) -> Result<TryVec<u32>> {
    let mut out = TryVec::new();
    let mut i = 0;
    while i < raw.len() {
        let c = raw[i];
        if c == b'<' && is_symbol_start(raw, i, esc) {
            let mut j = i + 1;
            let mut name = TryVec::new();
            while j < raw.len() && raw[j] != b'>' {
                if raw[j] == esc && j + 1 < raw.len() {
                    j += 1;
                }
                name.push(raw[j])?;
                j += 1;
            }
            let cp = ucs_name(&name).ok_or(Error::syntax(no, "unknown symbolic name in a string"))?;
            out.push(cp)?;
            i = j + 1;
            continue;
        }
        if c == esc && i + 1 < raw.len() {
            out.push(u32::from(raw[i + 1]))?;
            i += 2;
            continue;
        }
        let (cp, n) = utf8_at(&raw[i..]).ok_or(Error::syntax(no, "a string is not UTF-8"))?;
        out.push(cp)?;
        i += n;
    }
    Ok(out)
}

/// The UTF-8 character at the start of `b` and its length.
fn utf8_at(b: &[u8]) -> Option<(u32, usize)> {
    let c0 = *b.first()?;
    let (n, init) = match c0 {
        0x00..=0x7f => return Some((u32::from(c0), 1)),
        0xc2..=0xdf => (2, u32::from(c0 & 0x1f)),
        0xe0..=0xef => (3, u32::from(c0 & 0x0f)),
        0xf0..=0xf4 => (4, u32::from(c0 & 0x07)),
        _ => return None,
    };
    let mut cp = init;
    for k in 1..n {
        let c = *b.get(k)?;
        if c & 0xc0 != 0x80 {
            return None;
        }
        cp = (cp << 6) | u32::from(c & 0x3f);
    }
    Some((cp, n))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(src: &[u8]) -> Vec<Vec<Tok>> {
        lines(src).map(|(l, _)| l.iter().map(|l| l.toks.to_vec()).collect()).unwrap_or_default()
    }

    fn w(s: &[u8]) -> Tok {
        Tok::Word(TryVec::from(s.to_vec()))
    }

    fn sym(s: &[u8]) -> Tok {
        Tok::Sym(TryVec::from(s.to_vec()))
    }

    #[test]
    fn directives_continuation_and_tokens() {
        // no spec clause: POSIX XBD 7.3 (comment_char, escape_char, continuation lines).
        let t = toks(b"comment_char %\nescape_char /\n% a comment /\nupper <U0041>..<U005A>;/\n <U00C0>\ngrouping 3;-1\n");
        assert_eq!(t, vec![vec![w(b"upper"), sym(b"U0041"), w(b".."), sym(b"U005A"), Tok::Semi, sym(b"U00C0")], vec![w(b"grouping"), w(b"3"), Tok::Semi, w(b"-1")]]);
    }

    #[test]
    fn strings_decode_names_escapes_and_utf8() {
        // no spec clause: POSIX XBD 7.3 (strings of symbolic names and characters).
        let v = decode_string("<U0041>a/\"é".as_bytes(), b'/', 1).map(|v| v.to_vec());
        assert_eq!(v, Ok(vec![0x41, 0x61, 0x22, 0xe9]));
        assert!(decode_string(b"<foo>", b'/', 1).is_err());
    }
}
