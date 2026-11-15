//! The transcript file format (docs/transcripts.md): one case of a utility, its inputs and the outcome the reference
//! gave when it was recorded.
//!
//! The file is UTF-8 text, one directive per line (`key: value`); `#` lines and blank lines are skipped. Bytes that are
//! not printable ASCII are written as escapes (`\\`, `\t`, `\r`, `\n`, `\xHH`), so every byte string, binary output
//! included, has one spelling, and a trailing space or tab survives an editor that strips line ends.

use std::fmt::Write as _;

/// How the command is started.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Namespace {
    /// A plain child process (the default).
    None,
    /// A user and mount namespace: the binds are mounted, then the command runs with the caller's own uid and gid.
    User,
    /// A user and mount namespace in which the command runs as uid 0 (the caller mapped to root, no privilege outside).
    Root,
}

/// One library of a `cache:` directive, written with `liella_ld::cache::write`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CacheEntry {
    pub flags: u32,
    pub key: Vec<u8>,
    pub path: Vec<u8>,
    pub hwcaps: Option<Vec<u8>>,
}

/// One item of the private root, created before the command runs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Setup {
    Dir(Vec<u8>),
    File(Vec<u8>, Vec<u8>),
    Link(Vec<u8>, Vec<u8>),
    /// A loader cache from Liella's writer: path, generator text, entries in configuration order.
    Cache(Vec<u8>, Vec<u8>, Vec<CacheEntry>),
    /// A minimal shared object (ET_DYN, PT_LOAD + PT_DYNAMIC): path, its `DT_SONAME` if any, the number of bytes
    /// kept when the file is to be cut short, and its ELF class and machine.
    Elf(Vec<u8>, Option<Vec<u8>>, Option<usize>, ElfKind),
}

/// The class and machine of an `elf:` item (`class=`): the native x86-64 ELF64 (the default), or one of the two
/// foreign classes a multilib x86-64 system holds, i386 (ELF32, EM_386) and x32 (ELF32, EM_X86_64).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ElfKind {
    X86_64,
    I386,
    X32,
}

/// How the command ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Status {
    Exit(i32),
    Signal(i32),
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::Exit(n) => write!(f, "exit {n}"),
            Status::Signal(n) => write!(f, "signal {n}"),
        }
    }
}

/// What the command left in the private root, relative to the tree before it ran.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Written {
    /// A file created or changed, with its bytes.
    File(Vec<u8>),
    /// A file known only by its sha256 and size (a recorded binary or large file).
    Digest(String, u64),
    Dir,
    Link(Vec<u8>),
    /// Something of another type (a fifo, a socket, a device): its `st_mode` type bits.
    Other(u32),
    Removed,
}

/// The observable result of one run, after normalisation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Outcome {
    pub status: Status,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    /// Sorted by path (bytes).
    pub written: Vec<(Vec<u8>, Written)>,
}

/// A parsed transcript.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Transcript {
    pub name: String,
    pub clauses: Vec<String>,
    pub command: String,
    pub args: Vec<Vec<u8>>,
    pub env: Vec<(Vec<u8>, Vec<u8>)>,
    pub stdin: Vec<u8>,
    pub namespace: Namespace,
    /// (path inside the private root, absolute path it is mounted over).
    pub binds: Vec<(Vec<u8>, Vec<u8>)>,
    pub setup: Vec<Setup>,
    /// Extra literal replacements applied after the private root's path (`normalise: <from> => <to>`).
    pub normalise: Vec<(Vec<u8>, Vec<u8>)>,
    /// Written files compared as a loader cache's entry list (`as-cache: <path>`), not as bytes.
    pub as_cache: Vec<Vec<u8>>,
    pub expected: Option<Outcome>,
    /// The text before the first outcome directive: kept as written when `record` replaces the outcome.
    pub head: String,
}

/// The token that stands for the private root's path in arguments, environment values, stdin, link targets and (after
/// normalisation) in every output.
pub(crate) const ROOT_TOKEN: &[u8] = b"@ROOT@";

/// Files written inline in a record at most this size and only when every byte is printable ASCII, tab or newline;
/// others are recorded as sha256 and size.
pub(crate) const INLINE_MAX: usize = 64 * 1024;

/// Escape `bytes` for a directive value or a block line. `token` also escapes spaces (a value split on spaces).
pub(crate) fn escape(bytes: &[u8], token: bool) -> String {
    let mut s = String::with_capacity(bytes.len());
    for (i, &b) in bytes.iter().enumerate() {
        let last = i + 1 == bytes.len();
        match b {
            b'\\' => s.push_str("\\\\"),
            b'\t' => s.push_str("\\t"),
            b'\r' => s.push_str("\\r"),
            b'\n' => s.push_str("\\n"),
            b' ' if token || last || i == 0 => s.push_str("\\x20"),
            0x21..=0x7e | b' ' => s.push(b as char),
            _ => {
                let _ = write!(s, "\\x{b:02x}");
            }
        }
    }
    s
}

/// The inverse of [`escape`].
pub(crate) fn unescape(s: &str) -> Result<Vec<u8>, String> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] != b'\\' {
            out.push(b[i]);
            i += 1;
            continue;
        }
        match b.get(i + 1) {
            Some(b'\\') => out.push(b'\\'),
            Some(b't') => out.push(b'\t'),
            Some(b'r') => out.push(b'\r'),
            Some(b'n') => out.push(b'\n'),
            Some(b'x') => {
                let hex = s.get(i + 2..i + 4).ok_or_else(|| format!("short \\x escape in {s:?}"))?;
                out.push(u8::from_str_radix(hex, 16).map_err(|_| format!("bad \\x escape in {s:?}"))?);
                i += 2;
            }
            _ => return Err(format!("bad escape in {s:?}")),
        }
        i += 2;
    }
    Ok(out)
}

/// A path inside the private root: relative, no empty, `.` or `..` component.
fn rel_path(s: &str) -> Result<Vec<u8>, String> {
    let p = unescape(s)?;
    if p.is_empty() || p[0] == b'/' || p.split(|&c| c == b'/').any(|c| c.is_empty() || c == b"." || c == b"..") {
        return Err(format!("{s:?}: a path inside the private root must be relative, without empty, `.` or `..` parts"));
    }
    Ok(p)
}

/// Write a byte string as block lines (`| text` per line; `|~ text` for a last line without a newline).
pub(crate) fn render_block(out: &mut String, bytes: &[u8]) {
    if bytes.is_empty() {
        return;
    }
    let body = bytes.strip_suffix(b"\n");
    let (lines, last_open) = match body {
        Some(b) => (b, false),
        None => (bytes, true),
    };
    let parts: Vec<&[u8]> = lines.split(|&c| c == b'\n').collect();
    for (i, line) in parts.iter().enumerate() {
        let marker = if last_open && i + 1 == parts.len() { "|~" } else { "|" };
        out.push_str(marker);
        if !line.is_empty() {
            out.push(' ');
            out.push_str(&escape(line, false));
        }
        out.push('\n');
    }
}

/// Render the outcome directives (what `record` writes after the head).
pub(crate) fn render_outcome(o: &Outcome) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "status: {}", o.status);
    s.push_str("stdout:\n");
    render_block(&mut s, &o.stdout);
    s.push_str("stderr:\n");
    render_block(&mut s, &o.stderr);
    for (path, w) in &o.written {
        let p = escape(path, true);
        match w {
            Written::File(bytes) => {
                let inline = bytes.len() <= INLINE_MAX && bytes.iter().all(|&b| b == b'\t' || b == b'\n' || (0x20..0x7f).contains(&b));
                if inline {
                    let _ = writeln!(s, "written-file: {p}");
                    render_block(&mut s, bytes);
                } else {
                    let _ = writeln!(s, "written-file: {p} sha256 {} size {}", super::compare::sha256(bytes), bytes.len());
                }
            }
            Written::Digest(h, n) => {
                let _ = writeln!(s, "written-file: {p} sha256 {h} size {n}");
            }
            Written::Dir => {
                let _ = writeln!(s, "written-dir: {p}");
            }
            Written::Link(t) => {
                let _ = writeln!(s, "written-link: {p} -> {}", escape(t, true));
            }
            Written::Other(m) => {
                let _ = writeln!(s, "written-other: {p} mode {m:o}");
            }
            Written::Removed => {
                let _ = writeln!(s, "written-removed: {p}");
            }
        }
    }
    s
}

const OUTCOME_KEYS: &[&str] = &["status", "stdout", "stderr", "written-file", "written-dir", "written-link", "written-other", "written-removed"];

/// Parse a transcript. `what` names it in messages.
pub(crate) fn parse(text: &str, what: &str) -> Result<Transcript, String> {
    let lines: Vec<&str> = text.lines().collect();
    let mut t = Transcript {
        name: String::new(),
        clauses: Vec::new(),
        command: String::new(),
        args: Vec::new(),
        env: Vec::new(),
        stdin: Vec::new(),
        namespace: Namespace::None,
        binds: Vec::new(),
        setup: Vec::new(),
        normalise: Vec::new(),
        as_cache: Vec::new(),
        expected: None,
        head: String::new(),
    };
    let mut out = Outcome { status: Status::Exit(0), stdout: Vec::new(), stderr: Vec::new(), written: Vec::new() };
    let mut seen_status = false;
    let mut in_outcome = false;
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let n = i + 1;
        i += 1;
        let err = |m: String| format!("{what}:{n}: {m}");
        if line.trim().is_empty() || line.starts_with('#') {
            if !in_outcome {
                t.head.push_str(line);
                t.head.push('\n');
            }
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            return Err(err(format!("not a directive: {line:?}")));
        };
        let value = value.strip_prefix(' ').unwrap_or(value);
        let is_outcome = OUTCOME_KEYS.contains(&key);
        if in_outcome && !is_outcome {
            return Err(err(format!("`{key}:` after the outcome: the recorded outcome comes last")));
        }
        if is_outcome {
            in_outcome = true;
        } else {
            t.head.push_str(line);
            t.head.push('\n');
        }
        // A block follows these directives: the `|` lines after them.
        let mut block = || -> Result<Vec<u8>, String> {
            let mut b = Vec::new();
            while i < lines.len() && lines[i].starts_with('|') {
                let l = lines[i];
                let (open, rest) = match l.strip_prefix("|~") {
                    Some(r) => (true, r),
                    None => (false, &l[1..]),
                };
                let rest = match rest.strip_prefix(' ') {
                    Some(r) => r,
                    None if rest.is_empty() => rest,
                    None => return Err(format!("{what}:{}: a block line is `| text`, `|`, or `|~ text`", i + 1)),
                };
                b.extend(unescape(rest).map_err(|e| format!("{what}:{}: {e}", i + 1))?);
                if !in_outcome {
                    t.head.push_str(l);
                    t.head.push('\n');
                }
                i += 1;
                if open {
                    if i < lines.len() && lines[i].starts_with('|') {
                        return Err(format!("{what}:{}: `|~` must be the last line of its block", i + 1));
                    }
                } else {
                    b.push(b'\n');
                }
            }
            Ok(b)
        };
        match key {
            "name" => t.name = value.to_string(),
            "clause" => t.clauses.push(value.to_string()),
            "command" => {
                if value.is_empty() || value.contains('/') || value.contains(' ') {
                    return Err(err(format!("command {value:?}: a bare utility name")));
                }
                t.command = value.to_string();
            }
            "arg" => t.args.push(unescape(value).map_err(err)?),
            "env" => {
                let (k, v) = value.split_once('=').ok_or_else(|| err(format!("env {value:?}: NAME=value")))?;
                t.env.push((unescape(k).map_err(err)?, unescape(v).map_err(err)?));
            }
            "stdin" => {
                if !value.is_empty() {
                    return Err(err("stdin: the bytes follow as a block".into()));
                }
                t.stdin = block()?;
            }
            "namespace" => {
                t.namespace = match value {
                    "none" => Namespace::None,
                    "user" => Namespace::User,
                    "root" => Namespace::Root,
                    _ => return Err(err(format!("namespace {value:?}: none, user or root"))),
                }
            }
            "bind" => {
                let (src, dst) = value.split_once(' ').ok_or_else(|| err("bind: <path in the root> <absolute path>".into()))?;
                let dst = unescape(dst).map_err(err)?;
                if dst.first() != Some(&b'/') {
                    return Err(err("bind: the target is an absolute path".into()));
                }
                t.binds.push((rel_path(src).map_err(err)?, dst));
            }
            "dir" => t.setup.push(Setup::Dir(rel_path(value).map_err(err)?)),
            "file" => {
                let p = rel_path(value).map_err(err)?;
                let body = block()?;
                t.setup.push(Setup::File(p, body));
            }
            "link" => {
                let (p, target) = value.split_once(" -> ").ok_or_else(|| err("link: <path> -> <target>".into()))?;
                t.setup.push(Setup::Link(rel_path(p).map_err(err)?, unescape(target).map_err(err)?));
            }
            "cache" => t.setup.push(Setup::Cache(rel_path(value).map_err(err)?, Vec::new(), Vec::new())),
            "generator" => match t.setup.last_mut() {
                Some(Setup::Cache(_, g, _)) => *g = unescape(value).map_err(err)?,
                _ => return Err(err("generator: belongs to the `cache:` before it".into())),
            },
            "entry" => {
                let Some(Setup::Cache(_, _, entries)) = t.setup.last_mut() else {
                    return Err(err("entry: belongs to the `cache:` before it".into()));
                };
                let f: Vec<&str> = value.split(' ').collect();
                let usage = || err("entry: <flags, 0x hex> <key> <path> [hwcaps=<name>]".into());
                if f.len() < 3 || f.len() > 4 {
                    return Err(usage());
                }
                let flags = f[0].strip_prefix("0x").and_then(|h| u32::from_str_radix(h, 16).ok()).ok_or_else(usage)?;
                let hwcaps = match f.get(3) {
                    Some(h) => Some(unescape(h.strip_prefix("hwcaps=").ok_or_else(usage)?).map_err(err)?),
                    None => None,
                };
                entries.push(CacheEntry { flags, key: unescape(f[1]).map_err(err)?, path: unescape(f[2]).map_err(err)?, hwcaps });
            }
            "elf" => {
                let mut f = value.split(' ');
                let p = rel_path(f.next().unwrap_or_default()).map_err(err)?;
                let (mut soname, mut cut, mut kind) = (None, None, ElfKind::X86_64);
                for o in f {
                    if let Some(s) = o.strip_prefix("soname=") {
                        soname = Some(unescape(s).map_err(err)?);
                    } else if let Some(n) = o.strip_prefix("cut=").and_then(|n| n.parse().ok()) {
                        cut = Some(n);
                    } else if let Some(k) = o.strip_prefix("class=").and_then(|k| match k {
                        "x86-64" => Some(ElfKind::X86_64),
                        "i386" => Some(ElfKind::I386),
                        "x32" => Some(ElfKind::X32),
                        _ => None,
                    }) {
                        kind = k;
                    } else {
                        return Err(err("elf: <path> [soname=<name>] [cut=<bytes>] [class=x86-64|i386|x32]".into()));
                    }
                }
                t.setup.push(Setup::Elf(p, soname, cut, kind));
            }
            "as-cache" => t.as_cache.push(rel_path(value).map_err(err)?),
            "normalise" => {
                let (from, to) = value.split_once(" => ").ok_or_else(|| err("normalise: <literal> => <replacement>".into()))?;
                let from = unescape(from).map_err(err)?;
                if from.is_empty() {
                    return Err(err("normalise: an empty literal".into()));
                }
                t.normalise.push((from, unescape(to).map_err(err)?));
            }
            "status" => {
                seen_status = true;
                let (kind, num) = value.split_once(' ').ok_or_else(|| err("status: exit <n> | signal <n>".into()))?;
                let num: i32 = num.parse().map_err(|_| err(format!("status {value:?}")))?;
                out.status = match kind {
                    "exit" => Status::Exit(num),
                    "signal" => Status::Signal(num),
                    _ => return Err(err(format!("status {value:?}: exit <n> | signal <n>"))),
                };
            }
            "stdout" => out.stdout = block()?,
            "stderr" => out.stderr = block()?,
            _ if key.starts_with("written-") => {
                let w = written(key, value).map_err(err)?;
                let w = match w {
                    (p, Written::File(_)) => {
                        let body = block()?;
                        (p, Written::File(body))
                    }
                    other => other,
                };
                if out.written.last().is_some_and(|(p, _)| *p >= w.0) {
                    return Err(err("written paths are sorted and unique".into()));
                }
                out.written.push(w);
            }
            _ => return Err(err(format!("unknown directive `{key}:`"))),
        }
    }
    let need = |ok: bool, m: &str| if ok { Ok(()) } else { Err(format!("{what}: {m}")) };
    need(!t.name.is_empty(), "no `name:`")?;
    need(!t.command.is_empty(), "no `command:`")?;
    need(!t.clauses.is_empty(), "no `clause:` (a spec clause id, or `none (<why>)`)")?;
    need(t.binds.is_empty() || t.namespace != Namespace::None, "`bind:` needs `namespace: user` or `namespace: root`")?;
    if in_outcome {
        need(seen_status, "a recorded outcome without `status:`")?;
        t.expected = Some(out);
    }
    Ok(t)
}

/// One `written-*:` directive. A `written-file:` without a digest returns an empty `File`; its block follows.
fn written(key: &str, value: &str) -> Result<(Vec<u8>, Written), String> {
    let mut f = value.split(' ');
    let p = rel_path(f.next().unwrap_or_default())?;
    let rest: Vec<&str> = f.collect();
    let w = match (key, rest.as_slice()) {
        ("written-file", []) => Written::File(Vec::new()),
        ("written-file", ["sha256", h, "size", n]) if h.len() == 64 => Written::Digest(h.to_string(), n.parse().map_err(|_| format!("size {n:?}"))?),
        ("written-dir", []) => Written::Dir,
        ("written-link", ["->", t]) => Written::Link(unescape(t)?),
        ("written-other", ["mode", m]) => Written::Other(u32::from_str_radix(m, 8).map_err(|_| format!("mode {m:?}"))?),
        ("written-removed", []) => Written::Removed,
        _ => return Err(format!("`{key}: {value}` is not a written-* directive")),
    };
    Ok((p, w))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_round_trips_every_byte() {
        // Trace: no spec clause (test-harness tooling; UTIL-HARNESS transcript format).
        let all: Vec<u8> = (0..=255u8).chain(*b"  a b  ").collect();
        for token in [false, true] {
            let e = escape(&all, token);
            assert!(e.bytes().all(|b| (0x20..0x7f).contains(&b)), "{e}");
            assert!(!e.ends_with(' ') && !e.starts_with(' '));
            assert!(!token || !e.contains(' '), "{e}");
            assert_eq!(unescape(&e).unwrap(), all);
        }
    }

    #[test]
    fn a_recorded_outcome_parses_back_to_itself() {
        // Trace: no spec clause (test-harness tooling; UTIL-HARNESS transcript format).
        let o = Outcome {
            status: Status::Exit(1),
            stdout: b"one\n\ttwo \n\nlast".to_vec(),
            stderr: b"err\n".to_vec(),
            written: vec![
                (b"a".to_vec(), Written::File(b"text\n".to_vec())),
                (b"b".to_vec(), Written::File(vec![0, 1, 2])),
                (b"c".to_vec(), Written::Dir),
                (b"d".to_vec(), Written::Link(b"../x y".to_vec())),
                (b"e".to_vec(), Written::Removed),
            ],
        };
        let text = format!("name: n\nclause: none (format test)\ncommand: true\n{}", render_outcome(&o));
        let t = parse(&text, "t").unwrap();
        let mut want = o.clone();
        want.written[1].1 = Written::Digest(super::super::compare::sha256(&[0, 1, 2]), 3);
        assert_eq!(t.expected, Some(want));
        assert_eq!(t.head, "name: n\nclause: none (format test)\ncommand: true\n");
    }

    #[test]
    fn the_outcome_must_come_last_and_binds_need_a_namespace() {
        // Trace: no spec clause (test-harness tooling; UTIL-HARNESS transcript format).
        let e = parse("name: n\nclause: x\ncommand: c\nstatus: exit 0\narg: -p\n", "t").unwrap_err();
        assert!(e.contains("comes last"), "{e}");
        let e = parse("name: n\nclause: x\ncommand: c\nbind: etc/x /etc/x\n", "t").unwrap_err();
        assert!(e.contains("needs `namespace"), "{e}");
        let e = parse("name: n\nclause: x\ncommand: c\nfile: ../x\n", "t").unwrap_err();
        assert!(e.contains("relative"), "{e}");
    }
}
