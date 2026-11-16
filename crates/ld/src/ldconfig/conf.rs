//! The `ld.so.conf` reader (spec §31.2 Inputs; plan/utilities.md §4; ldconfig(8)).
//!
//! A configuration file is a list of lines: `#` starts a comment anywhere on a line, blank
//! lines are skipped, `include <pattern>...` reads the files the patterns name (each pattern a
//! glob, its matches in sorted order), `hwcap ...` is reported as ignored, and any other line is
//! one directory (surrounding blanks removed). Probe-settled (UTIL-LDCONFIG, black box):
//! * a relative include pattern is taken relative to the directory of the file that holds it,
//!   and with `-r` it is an error that stops the run (`need absolute file name for configuration
//!   file when using -r`, exit 1);
//! * a file that does not exist, an unreadable file and a pattern without matches are skipped
//!   silently; a bare `include` is a directory line named `include`.
//!
//! **libniji rule:** an include that would re-enter a file already being read (a cycle, found by
//! file identity) or nest deeper than [`DEPTH_MAX`] is refused with a message naming the file and
//! line, and the rest of the configuration is read as usual (the exit status does not change).

use alloc::vec::Vec;

use super::{push, Fs, Kind, Oom, Text};

/// `ENOTDIR`: a configured path that is not a directory.
const ENOTDIR: i32 = 20;

/// The deepest include nesting read; one deeper is refused with a message.
pub const DEPTH_MAX: usize = 32;
/// The largest configuration file read; the rest of a longer file is ignored.
pub const FILE_MAX: usize = 1 << 20;
/// The most directories a configuration may name in total (the rest are ignored with a message).
pub const DIRS_MAX: usize = 1 << 16;

/// One configured directory and where it was named.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dir {
    /// The directory as written (blanks removed).
    pub path: Vec<u8>,
    /// The file that named it, as messages show it.
    pub from: Vec<u8>,
    /// Its line number in that file (1-based).
    pub line: u32,
}

/// Why reading the configuration stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fatal {
    /// A relative include pattern while a root is in use.
    RelativeUnderRoot,
    /// Out of memory.
    Oom,
}

impl From<Oom> for Fatal {
    fn from(_: Oom) -> Self {
        Fatal::Oom
    }
}

/// Reads the configuration rooted at `file` (shown as `shown` in messages).
pub struct Reader<'f, F: Fs> {
    fs: &'f mut F,
    rooted: bool,
    /// File identities of the files being read (the include stack).
    stack: Vec<(u64, u64)>,
    /// The directories kept, in order, with their file identity (device, inode).
    pub kept: Vec<(Dir, (u64, u64))>,
}

/// `#` to the end of the line goes; then blanks at both ends.
fn strip(line: &[u8]) -> &[u8] {
    let line = match line.iter().position(|&c| c == b'#') {
        Some(i) => line.get(..i).unwrap_or_default(),
        None => line,
    };
    line.trim_ascii()
}

/// The first word of `s` and the rest.
fn word(s: &[u8]) -> (&[u8], &[u8]) {
    let s = s.trim_ascii_start();
    let end = s.iter().position(u8::is_ascii_whitespace).unwrap_or(s.len());
    (s.get(..end).unwrap_or_default(), s.get(end..).unwrap_or_default())
}

impl<'f, F: Fs> Reader<'f, F> {
    /// A reader; `rooted` = a `-r` root is in use.
    pub fn new(fs: &'f mut F, rooted: bool) -> Self {
        Reader { fs, rooted, stack: Vec::new(), kept: Vec::new() }
    }

    /// Takes `d` as the next directory to scan, checked at once (probe: the messages come in configuration order,
    /// between the reader's own): one that cannot be stat'ed, or is not a directory, is reported and dropped
    /// (`Can't stat <dir>: <error>`); one already kept under any name (same file identity) is reported with both
    /// places and dropped, the first name staying.
    pub fn add(&mut self, d: Dir, me: &[u8], err: &mut Text) -> Result<(), Oom> {
        let m = match self.fs.stat(&d.path) {
            Ok(m) if m.kind == Kind::Dir => m,
            other => {
                err.bytes(me).bytes(b": Can't stat ").bytes(&d.path).bytes(b": ");
                self.fs.error_text(other.err().unwrap_or(ENOTDIR), err);
                err.bytes(b"\n");
                return Ok(());
            }
        };
        let id = (m.dev, m.ino);
        if let Some((first, _)) = self.kept.iter().find(|k| k.1 == id) {
            err.bytes(me).bytes(b": Path `").bytes(&d.path).bytes(b"' given more than once\n(from ").bytes(&d.from).bytes(b":").num(u64::from(d.line));
            err.bytes(b" and ").bytes(&first.from).bytes(b":").num(u64::from(first.line)).bytes(b")\n");
            return Ok(());
        }
        push(&mut self.kept, (d, id))
    }

    /// Reads `path` (shown in messages as `shown`), nested includes included; diagnostics go to `err`.
    pub fn read(&mut self, path: &[u8], shown: &[u8], me: &[u8], err: &mut Text) -> Result<(), Fatal> {
        let mut text = Vec::new();
        if self.fs.read_file(path, FILE_MAX, &mut text).is_err() {
            return Ok(()); // probe: a missing or unreadable file is skipped silently
        }
        let id = match self.fs.stat(path) {
            Ok(m) => (m.dev, m.ino),
            Err(_) => return Ok(()),
        };
        push(&mut self.stack, id)?;
        let r = self.lines(&text, path, shown, me, err);
        self.stack.pop();
        r
    }

    fn lines(&mut self, text: &[u8], path: &[u8], shown: &[u8], me: &[u8], err: &mut Text) -> Result<(), Fatal> {
        for (n, raw) in text.split(|&c| c == b'\n').enumerate() {
            let line = u32::try_from(n).unwrap_or(u32::MAX).saturating_add(1);
            let s = strip(raw);
            if s.is_empty() {
                continue;
            }
            let (first, rest) = word(s);
            if first == b"include" && !rest.trim_ascii().is_empty() {
                let mut rest = rest;
                loop {
                    let (pat, more) = word(rest);
                    if pat.is_empty() {
                        break;
                    }
                    rest = more;
                    self.include(pat, path, shown, line, me, err)?;
                }
            } else if first == b"hwcap" && !rest.trim_ascii().is_empty() {
                err.bytes(me).bytes(b": ").bytes(shown).bytes(b":").num(u64::from(line)).bytes(b": hwcap directive ignored\n");
            } else if self.kept.len() >= DIRS_MAX {
                err.bytes(me).bytes(b": ").bytes(shown).bytes(b":").num(u64::from(line)).bytes(b": too many directories, ignored\n");
            } else {
                let mut d = Dir { path: Vec::new(), from: Vec::new(), line };
                super::extend(&mut d.path, s)?;
                super::extend(&mut d.from, shown)?;
                self.add(d, me, err)?;
            }
        }
        Ok(())
    }

    fn include(&mut self, pat: &[u8], from: &[u8], shown: &[u8], line: u32, me: &[u8], err: &mut Text) -> Result<(), Fatal> {
        let mut full = Vec::new();
        if pat.first() != Some(&b'/') {
            if self.rooted {
                return Err(Fatal::RelativeUnderRoot);
            }
            // Relative to the including file's directory (probe).
            if let Some(slash) = from.iter().rposition(|&c| c == b'/') {
                super::extend(&mut full, from.get(..=slash).unwrap_or_default())?;
            }
        }
        super::extend(&mut full, pat)?;
        let mut found = Vec::new();
        expand(self.fs, &full, &mut found)?;
        found.sort_unstable();
        for f in &found {
            let id = match self.fs.stat(f) {
                Ok(m) if m.kind == Kind::File => (m.dev, m.ino),
                _ => continue,
            };
            let mut msg_shown = Vec::new();
            // The reference shows an included file with the root in front (probe A); kept.
            super::extend(&mut msg_shown, self.fs.root())?;
            super::extend(&mut msg_shown, f)?;
            if self.stack.contains(&id) {
                err.bytes(me).bytes(b": ").bytes(shown).bytes(b":").num(u64::from(line)).bytes(b": include of ").bytes(&msg_shown).bytes(b" would loop (include cycle), ignored\n");
                continue;
            }
            if self.stack.len() >= DEPTH_MAX {
                err.bytes(me).bytes(b": ").bytes(shown).bytes(b":").num(u64::from(line)).bytes(b": include of ").bytes(&msg_shown).bytes(b" nests too deep, ignored\n");
                continue;
            }
            self.read(f, &msg_shown, me, err)?;
        }
        Ok(())
    }
}

/// Glob metacharacters.
fn has_meta(s: &[u8]) -> bool {
    s.iter().any(|&c| matches!(c, b'*' | b'?' | b'['))
}

/// The paths `pattern` names: every component with a metacharacter is matched against the sorted
/// directory listing; a pattern without one names itself (existence is checked by the caller).
pub fn expand<F: Fs>(fs: &mut F, pattern: &[u8], out: &mut Vec<Vec<u8>>) -> Result<(), Oom> {
    let absolute = pattern.first() == Some(&b'/');
    let comps: Vec<&[u8]> = pattern.split(|&c| c == b'/').filter(|c| !c.is_empty()).collect();
    let mut base = Vec::new();
    if absolute {
        push(&mut base, b'/')?;
    }
    walk(fs, &base, &comps, out, 0)
}

fn walk<F: Fs>(fs: &mut F, base: &[u8], comps: &[&[u8]], out: &mut Vec<Vec<u8>>, depth: usize) -> Result<(), Oom> {
    let Some((&first, rest)) = comps.split_first() else {
        if !base.is_empty() {
            let mut p = Vec::new();
            super::extend(&mut p, base)?;
            push(out, p)?;
        }
        return Ok(());
    };
    if depth > 64 {
        return Ok(());
    }
    let join = |name: &[u8]| -> Result<Vec<u8>, Oom> {
        let mut p = Vec::new();
        super::extend(&mut p, base)?;
        if !p.is_empty() && p.last() != Some(&b'/') {
            push(&mut p, b'/')?;
        }
        super::extend(&mut p, name)?;
        Ok(p)
    };
    if !has_meta(first) {
        let p = join(&unescape(first)?)?;
        return walk(fs, &p, rest, out, depth.saturating_add(1));
    }
    let dir: &[u8] = if base.is_empty() { b"." } else { base };
    let mut names = Vec::new();
    if fs.list(dir, &mut names).is_err() {
        return Ok(());
    }
    names.sort_unstable();
    for n in &names {
        if fnmatch(first, n) {
            let p = join(n)?;
            walk(fs, &p, rest, out, depth.saturating_add(1))?;
        }
    }
    Ok(())
}

/// A literal component with its backslash escapes removed.
fn unescape(s: &[u8]) -> Result<Vec<u8>, Oom> {
    let mut v = Vec::new();
    let mut i = s.iter();
    while let Some(&c) = i.next() {
        let c = if c == b'\\' { i.next().copied().unwrap_or(b'\\') } else { c };
        push(&mut v, c)?;
    }
    Ok(v)
}

/// The allow: every index here is at most the length of a slice (so at most `isize::MAX`), and
/// each step adds at most 3 to it, so the additions cannot overflow.
#[allow(clippy::arithmetic_side_effects)]
/// Shell pattern match of one name (POSIX 2.13: `*`, `?`, bracket expressions with `!` or `^`
/// and ranges, backslash escapes); a leading `.` is matched only by a literal `.`.
pub fn fnmatch(pat: &[u8], name: &[u8]) -> bool {
    if name.first() == Some(&b'.') && pat.first() != Some(&b'.') {
        return false;
    }
    // Iterative matcher with one backtrack point for the last `*` (linear in practice, no recursion).
    let (mut p, mut n) = (0usize, 0usize);
    let mut star: Option<(usize, usize)> = None;
    loop {
        if p < pat.len() {
            let c = pat.get(p).copied().unwrap_or(0);
            match c {
                b'*' => {
                    star = Some((p, n));
                    p += 1;
                    continue;
                }
                b'?' if n < name.len() => {
                    p += 1;
                    n += 1;
                    continue;
                }
                b'[' if n < name.len() => {
                    if let Some((ok, len)) = bracket(pat.get(p..).unwrap_or_default(), name.get(n).copied().unwrap_or(0)) {
                        if ok {
                            p += len;
                            n += 1;
                            continue;
                        }
                    } else if name.get(n) == Some(&b'[') {
                        p += 1;
                        n += 1;
                        continue;
                    }
                }
                b'\\' if n < name.len() => {
                    let lit = pat.get(p + 1).copied().unwrap_or(b'\\');
                    if name.get(n) == Some(&lit) {
                        p += if p + 1 < pat.len() { 2 } else { 1 };
                        n += 1;
                        continue;
                    }
                }
                _ if n < name.len() && name.get(n) == Some(&c) && c != b'?' && c != b'[' => {
                    p += 1;
                    n += 1;
                    continue;
                }
                _ => {}
            }
        } else if n == name.len() {
            return true;
        }
        match star {
            Some((sp, sn)) if sn < name.len() => {
                star = Some((sp, sn + 1));
                p = sp + 1;
                n = sn + 1;
            }
            _ => return false,
        }
    }
}

#[allow(clippy::arithmetic_side_effects)] // as `fnmatch`: indices bounded by `pat.len()`
/// A bracket expression at the start of `pat` against `c`: (matched, length of the expression),
/// or `None` when it is not closed (then `[` is an ordinary character).
fn bracket(pat: &[u8], c: u8) -> Option<(bool, usize)> {
    let mut i = 1;
    let neg = matches!(pat.get(i), Some(b'!' | b'^'));
    if neg {
        i += 1;
    }
    let mut hit = false;
    let mut first = true;
    loop {
        let b = *pat.get(i)?;
        if b == b']' && !first {
            return Some((hit != neg, i + 1));
        }
        first = false;
        let lo = if b == b'\\' {
            i += 1;
            *pat.get(i)?
        } else {
            b
        };
        if pat.get(i + 1) == Some(&b'-') && pat.get(i + 2).is_some_and(|&x| x != b']') {
            let hi = *pat.get(i + 2)?;
            hit |= lo <= c && c <= hi;
            i += 3;
        } else {
            hit |= lo == c;
            i += 1;
        }
    }
}
