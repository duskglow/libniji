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
//! * [`defaults`]: the POSIX defaults of a category a source does not define.
//! * [`charset`]: the charmap a locale is compiled for (code point to bytes, widths).
//! * [`prog`]: the `localedef` command over a [`prog::Host`] (options, search paths, the
//!   directory form, messages).
//!
//! The files this crate writes are read by `niji::locale` (the C library's reader), which the
//! tests use as the oracle. File access goes through [`Files`], so the library runs the same in
//! the shipped program and in tests.
#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]
#![cfg_attr(not(test), deny(clippy::disallowed_methods))] // S8: fallible allocation only

extern crate alloc;

pub mod archive;
pub mod build;
pub mod cats;
pub mod charset;
pub mod collate;
pub mod ctype;
pub mod defaults;
pub mod lex;
pub mod prog;
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
    /// A category the source cannot have at all (an empty `LC_COLLATE`): nothing is written,
    /// even with `-c`.
    Refused,
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

/// The compiled files of one locale, by category number (`LC_ALL`, 6, is `None`).
pub type Compiled = [Option<TryVec<u8>>; 13];

/// What one compile produced.
#[derive(Debug)]
pub struct Output {
    /// Every category's file (`LC_ALL`, 6, is `None`). A category the source does not define,
    /// or one whose statements have a syntax error, has its POSIX default ([`defaults`]).
    pub files: Compiled,
    /// The categories the source defines.
    pub defined: [bool; 13],
    /// The syntax errors the compile went past, in the order found: lines outside any category,
    /// then one per category that failed (that category has its default instead).
    pub errors: TryVec<Error>,
    /// How many of [`Output::errors`] are lines outside any category (the first ones).
    pub outside: usize,
    /// The categories whose statements failed, by number (one error each after the outside ones,
    /// in category order).
    pub failed: [bool; 13],
}

/// Compile the locale source `locale` for the charmap `charmap` (both read through `files`):
/// every category the source defines, written as its Appendix D.1 file. Strict: a syntax error
/// anywhere is the error, and a category the source does not define is `None`.
pub fn compile(locale: &[u8], charmap: &[u8], files: &mut dyn Files) -> Result<Compiled> {
    let mut out = compile_output(locale, charmap, files)?;
    if let Some(e) = out.errors.first() {
        return Err(*e);
    }
    for (slot, defined) in out.files.iter_mut().zip(out.defined) {
        if !defined {
            *slot = None;
        }
    }
    Ok(out.files)
}

/// Compile as [`compile`] does, going past syntax errors (recorded in [`Output::errors`]) and
/// writing the POSIX default for every category without a file of its own (spec §31.8: the
/// reference writes all twelve, #7534). A missing input or exhausted memory is still the error.
pub fn compile_output(locale: &[u8], charmap: &[u8], files: &mut dyn Files) -> Result<Output> {
    let cs = charset::Charset::parse(&files.read(Kind::Charmap, charmap)?)?;
    let text = files.read(Kind::Locale, locale)?;
    compile_text(&text, &cs, files)
}

/// [`compile_output`] of the source `text` (already read) for the charset `cs`.
pub fn compile_text(text: &[u8], cs: &charset::Charset, files: &mut dyn Files) -> Result<Output> {
    let mut src = source::parse(text, files)?;
    resolve_names(&mut src, cs)?;
    let outside = src.errors.len();
    let mut out = Output { files: Default::default(), defined: [false; 13], errors: src.errors, outside, failed: [false; 13] };
    for (n, stmts) in src.cats.iter().enumerate() {
        let Some(stmts) = stmts else { continue };
        if let Some(d) = out.defined.get_mut(n) {
            *d = true;
        }
        match category(n, stmts, cs, files, false) {
            Ok(file) => {
                if let Some(slot) = out.files.get_mut(n) {
                    *slot = file;
                }
            }
            // Refused at the category's END line (probed: `<file>:<END line>: LC_COLLATE: ...`).
            Err(e) if e.kind == ErrorKind::Refused => return Err(Error { line: src.ends.get(n).copied().unwrap_or(0), ..e }),
            Err(e) if e.kind == ErrorKind::Syntax => {
                out.errors.push(e)?;
                if let Some(f) = out.failed.get_mut(n) {
                    *f = true;
                }
            }
            Err(e) => return Err(e),
        }
    }
    if out.files.iter().enumerate().any(|(n, f)| n != 6 && f.is_none()) {
        let defaults = source::parse(defaults::SOURCE.as_bytes(), files)?;
        for (n, stmts) in defaults.cats.iter().enumerate() {
            let (Some(stmts), Some(slot)) = (stmts, out.files.get_mut(n)) else { continue };
            if slot.is_none() {
                *slot = category(n, stmts, cs, files, true)?;
            }
        }
    }
    Ok(out)
}

/// Write each symbolic name the charmap defines other than `<Uxxxx>` (a POSIX name such as
/// `<a>` or `<space>`, XBD 6.4 and 7.3) as the `<Uxxxxxxxx>` of its code point, in every
/// statement and string of `src`; names the charmap does not define are left as written.
fn resolve_names(src: &mut source::Source, cs: &charset::Charset) -> Result<()> {
    let resolve = |n: &[u8]| cs.map().named(n);
    for stmts in src.cats.iter_mut().flatten() {
        for s in stmts.iter_mut() {
            for t in s.toks.iter_mut() {
                match t {
                    lex::Tok::Sym(n) if lex::ucs_name(n).is_none() => {
                        if let Some(cp) = resolve(n) {
                            *n = TryVec::from_slice(&lex::ucs_text(cp))?;
                        }
                    }
                    lex::Tok::Str(raw) => {
                        if let Some(new) = lex::resolve_string(raw, s.esc, &resolve)? {
                            *raw = new;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

/// The file of category `n` from its statements (`None` for `LC_ALL`); `defaults`: the
/// statements are `defaults.rs`'s, for a category the source does not define (D-31.24).
fn category(n: usize, stmts: &[source::Stmt], cs: &charset::Charset, files: &mut dyn Files, defaults: bool) -> Result<Option<TryVec<u8>>> {
    let b = if defaults { cats::Body::defaults(stmts) } else { cats::Body::new(stmts) };
    Ok(Some(match n {
        0 => ctype::write(&ctype::parse(stmts, files)?, cs, defaults)?,
        1 => cats::numeric(&b, cs)?,
        2 => cats::time(&b, cs)?,
        4 => cats::monetary(&b, cs)?,
        5 => cats::messages(&b, cs)?,
        7 => cats::paper(&b, cs)?,
        8 => cats::name(&b, cs)?,
        9 => cats::address(&b, cs)?,
        10 => cats::telephone(&b, cs)?,
        11 => cats::measurement(&b, cs)?,
        12 => cats::identification(&b, cs)?,
        3 => collate::write(stmts, cs, defaults)?,
        _ => return Ok(None),
    }))
}
