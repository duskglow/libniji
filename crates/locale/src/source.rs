//! A locale source split into its categories (POSIX XBD 7.3): `LC_<NAME>` … `END LC_<NAME>`
//! sections of statements, with `copy "<locale>"` replaced by the statements of the same
//! category of that locale's source (recursively, at most [`MAX_COPY_DEPTH`] deep, so a cycle is
//! an error rather than a hang).

use crate::lex::{self, Tok};
use crate::{Error, Files, Kind, Result, TryVec};

/// How deep `copy` may nest (the installed sources use at most 4).
pub const MAX_COPY_DEPTH: u32 = 16;

/// The categories a source can define, by their C numbers (`LC_ALL`, 6, is not one).
pub const NAMES: [(&[u8], usize); 12] = [
    (b"LC_CTYPE", 0),
    (b"LC_NUMERIC", 1),
    (b"LC_TIME", 2),
    (b"LC_COLLATE", 3),
    (b"LC_MONETARY", 4),
    (b"LC_MESSAGES", 5),
    (b"LC_PAPER", 7),
    (b"LC_NAME", 8),
    (b"LC_ADDRESS", 9),
    (b"LC_TELEPHONE", 10),
    (b"LC_MEASUREMENT", 11),
    (b"LC_IDENTIFICATION", 12),
];

/// The category number of a section name.
pub fn number(name: &[u8]) -> Option<usize> {
    NAMES.iter().find(|(n, _)| *n == name).map(|&(_, c)| c)
}

/// One statement: a logical line of a category, with the escape character of its file.
#[derive(Debug)]
pub struct Stmt {
    /// The line number in its file.
    pub no: u32,
    /// The tokens.
    pub toks: TryVec<Tok>,
    /// The escape character of the file the line came from.
    pub esc: u8,
}

impl Stmt {
    /// The statement's keyword (its first token, when a word).
    pub fn keyword(&self) -> &[u8] {
        self.toks.first().and_then(Tok::word).unwrap_or(&[])
    }

    /// The tokens after the keyword.
    pub fn args(&self) -> &[Tok] {
        self.toks.get(1..).unwrap_or(&[])
    }
}

/// A parsed source: the statements of each category (index = category number), `None` for a
/// category the source does not define.
#[derive(Debug, Default)]
pub struct Source {
    /// The statements per category number 0…12.
    pub cats: [Option<TryVec<Stmt>>; 13],
}

/// Parse the source `text`, loading `copy` targets through `files`.
pub fn parse(text: &[u8], files: &mut dyn Files) -> Result<Source> {
    parse_at(text, files, 0)
}

fn parse_at(text: &[u8], files: &mut dyn Files, depth: u32) -> Result<Source> {
    let (lines, chars) = lex::lines(text)?;
    let mut src = Source::default();
    let mut cur: Option<(usize, TryVec<Stmt>)> = None;
    for line in lines.into_inner() {
        let first = line.toks.first().and_then(Tok::word).unwrap_or(&[]);
        match cur.take() {
            None => {
                let Some(c) = number(first) else {
                    // Text between sections: the installed sources have none but `LC_*` lines; refuse anything else.
                    return Err(Error::syntax(line.no, "a statement outside any category"));
                };
                cur = Some((c, TryVec::new()));
            }
            Some((c, mut body)) => {
                if first == b"END" {
                    let name = line.toks.get(1).and_then(Tok::word).unwrap_or(&[]);
                    if number(name) != Some(c) {
                        return Err(Error::syntax(line.no, "END names another category"));
                    }
                    if let Some(slot) = src.cats.get_mut(c) {
                        *slot = Some(body);
                    }
                    continue;
                }
                if first == b"copy" {
                    let Some(Tok::Str(name)) = line.toks.get(1) else { return Err(Error::syntax(line.no, "copy needs a locale name")) };
                    if depth >= MAX_COPY_DEPTH {
                        return Err(Error::syntax(line.no, "copy nests too deep"));
                    }
                    let name = lex::decode_string(name, chars.escape, line.no)?;
                    let name = ascii_name(&name).ok_or(Error::syntax(line.no, "copy names a locale that is not ASCII"))?;
                    let text = files.read(Kind::Locale, &name)?;
                    let mut other = parse_at(&text, files, depth + 1)?;
                    let Some(stmts) = other.cats.get_mut(c).and_then(Option::take) else {
                        return Err(Error::syntax(line.no, "copy names a locale without this category"));
                    };
                    for s in stmts.into_inner() {
                        body.push(s)?;
                    }
                } else {
                    body.push(Stmt { no: line.no, toks: line.toks, esc: chars.escape })?;
                }
                cur = Some((c, body));
            }
        }
    }
    if cur.is_some() {
        return Err(Error::syntax(0, "a category has no END line"));
    }
    Ok(src)
}

/// The bytes of a name made of ASCII characters.
pub fn ascii_name(cps: &[u32]) -> Option<TryVec<u8>> {
    let mut v = TryVec::new();
    for &c in cps {
        v.push(u8::try_from(c).ok().filter(u8::is_ascii)?).ok()?;
    }
    Some(v)
}
