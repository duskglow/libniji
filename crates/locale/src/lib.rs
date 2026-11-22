//! localedef's library (UTIL-12; spec §31.8, Appendix D.1; plan/utilities.md §4 "Locale").
//!
//! * [`lex`]: the lexical layer of locale sources (POSIX XBD 7.3).
//! * [`source`]: a source file split into its categories, with `copy` resolved.
//! * [`build`]: a category file (Appendix D.1.1) from its items.
//! * [`cats`]: the string, number and byte categories (`LC_NUMERIC`, `LC_TIME`, `LC_MONETARY`,
//!   `LC_MESSAGES`, `LC_PAPER`, `LC_NAME`, `LC_ADDRESS`, `LC_TELEPHONE`, `LC_MEASUREMENT`,
//!   `LC_IDENTIFICATION`; D.1.5).
//! * [`ctype`]: `LC_CTYPE` (D.1.3).
//! * [`collate`]: `LC_COLLATE` (D.1.4).
//! * [`charset`]: the charmap a locale is compiled for (code point to bytes, widths).
//!
//! The files this crate writes are read by `niji::locale` (the C library's reader), which the
//! tests use as the oracle. File access goes through [`Files`], so the library runs the same in
//! the shipped program and in tests.
#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]
#![cfg_attr(not(test), deny(clippy::disallowed_methods))] // S8: fallible allocation only

extern crate alloc;

pub mod build;
pub mod cats;
pub mod charset;
pub mod collate;
pub mod ctype;
pub mod lex;
pub mod source;

pub use niji::mem::TryVec;

use niji::error::Errno;

/// What went wrong, with the source line where one applies (0: none).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Error {
    /// The kind of failure.
    pub kind: ErrorKind,
    /// The 1-based source line, 0 when the failure has no line.
    pub line: u32,
    /// A fixed description.
    pub what: &'static str,
}

/// The kinds of failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    /// The source does not follow the syntax.
    Syntax,
    /// A file named by the source or the options cannot be read.
    Missing,
    /// Memory ran out.
    NoMemory,
}

impl Error {
    /// A syntax error at `line`.
    pub const fn syntax(line: u32, what: &'static str) -> Error {
        Error { kind: ErrorKind::Syntax, line, what }
    }

    /// A file that cannot be read.
    pub const fn missing(what: &'static str) -> Error {
        Error { kind: ErrorKind::Missing, line: 0, what }
    }
}

impl From<Errno> for Error {
    fn from(_: Errno) -> Error {
        Error { kind: ErrorKind::NoMemory, line: 0, what: "out of memory" }
    }
}

/// The result of this crate's operations.
pub type Result<T> = core::result::Result<T, Error>;

/// Which kind of input file a name refers to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A locale source (`-i`, `copy`, `include`): `/usr/share/i18n/locales/<name>`.
    Locale,
    /// A charmap (`-f`): `/usr/share/i18n/charmaps/<name>` (or `<name>.gz`).
    Charmap,
}

/// The input files, by name. The program implements it over the file system; tests over a map.
pub trait Files {
    /// The contents of the file `name` of `kind` (decompressed), or `Err` when there is none.
    fn read(&mut self, kind: Kind, name: &[u8]) -> Result<TryVec<u8>>;
}

/// The compiled files of one locale, by category number (`LC_ALL`, 6, and categories the
/// source does not define are `None`).
pub type Compiled = [Option<TryVec<u8>>; 13];

/// Compile the locale source `locale` for the charmap `charmap` (both read through `files`):
/// each category the source defines, written as its Appendix D.1 file.
pub fn compile(locale: &[u8], charmap: &[u8], files: &mut dyn Files) -> Result<Compiled> {
    let cs = charset::Charset::parse(&files.read(Kind::Charmap, charmap)?)?;
    let text = files.read(Kind::Locale, locale)?;
    let src = source::parse(&text, files)?;
    let mut out: Compiled = Default::default();
    for (n, stmts) in src.cats.iter().enumerate() {
        let Some(stmts) = stmts else { continue };
        let b = cats::Body::new(stmts);
        let file = match n {
            0 => ctype::write(&ctype::parse(stmts, files)?, &cs)?,
            1 => cats::numeric(&b, &cs)?,
            2 => cats::time(&b, &cs)?,
            4 => cats::monetary(&b, &cs)?,
            5 => cats::messages(&b, &cs)?,
            7 => cats::paper(&b, &cs)?,
            8 => cats::name(&b, &cs)?,
            9 => cats::address(&b, &cs)?,
            10 => cats::telephone(&b, &cs)?,
            11 => cats::measurement(&b, &cs)?,
            12 => cats::identification(&b, &cs)?,
            3 => collate::write(stmts, &cs)?,
            _ => continue,
        };
        if let Some(slot) = out.get_mut(n) {
            *slot = Some(file);
        }
    }
    Ok(out)
}
