//! `ldconfig` (spec §31.2, plan/utilities.md UTIL-3): the configuration reader, the directory
//! scan, the soname links and the cache, over a small file-system interface ([`Fs`]) so the same
//! engine serves the hosted front end (the transcript harness) and the freestanding product.
//!
//! Probe-settled behaviour (black box, the reference ldconfig in private roots, UTIL-LDCONFIG):
//! * configured directories are collected first (`Can't stat <dir>: <error>` for a missing one;
//!   ``Path `<dir>' given more than once`` with `(from <later> and <earlier>)` for one reached
//!   twice, by file identity, the first name kept), then scanned in order: command-line operands
//!   (`<cmdline>:0`), the configuration, the trusted directories (`<builtin>:0`);
//! * a directory's files named `lib*.so*` or `ld-*.so*` are read; files that are not ELF are
//!   skipped silently, a short one is reported as truncated; a symlink that cannot be followed is
//!   reported with `-v` (`Cannot stat <path>: <error>`) and, unless `-X` or its name ends in `.so`,
//!   removed;
//! * each soname's link points to the candidate with the highest name under the D.3.3
//!   comparison (regular files and symlinks alike, except the link itself); a file without
//!   `DT_SONAME` is keyed by its own name; a symlink whose name ends in `.so` and is a prefix of
//!   its target's soname (a development link) is recorded in the cache under its own name, gets no
//!   link and no `-v` line;
//! * libniji's soname-link rule (spec §31.2, settled by probes, LDCONFIG-DEV-ALIAS, GitHub issue #16): any other
//!   symlink that resolves to a library, a `.so` alias such as `libcmockery.so` included, is a candidate like a
//!   regular file and gets no cache entry of its own; a `.so` alias that dangles during the scan is no candidate;
//!   a `.so` regular file is a candidate (`libbar.so` with soname `libbar.so.3` gets `libbar.so.3 -> libbar.so`;
//!   one whose soname is its own name is recorded under it, no link). An existing soname link that already
//!   resolves to the winner's file (same device and inode) is left as it is and not reported as changed,
//!   whatever its text (`./libfoo.so.1.2`, or a chain through a development link): `-v` names the winner, without
//!   ` (changed)`. So an alias whose chain runs through the soname link itself never becomes that link's target,
//!   and no run makes a link loop;
//! * `-v` prints `<dir>: (from <file>:<line>)` and `\t<soname> -> <file>`, plus ` (changed)` when
//!   the link was created or replaced; `-X` makes no links, `-N` writes no cache (with both, the
//!   directories are still scanned and shown); `-n` takes only the operands and writes no cache;
//! * a directory's `glibc-hwcaps/<name>` subdirectories are scanned after it, each shown by `-v` as
//!   `<dir>: (hwcap: "<name>") (from ...)`; no links are made there and the cache entry names the
//!   file itself.
//!
//! libniji rules (the pending decision record of UTIL-LDCONFIG): directory listings and soname
//! groups are sorted, so the same tree gives the same output and byte-identical caches on any
//! file system; a soname link is replaced atomically (a temporary link renamed over it); the
//! cache is written to `<cache>~`, synced and renamed (the [`Fs`] does this); every ELF file is
//! read with bounds-checked positioned reads, never mapped; include cycles are refused with a
//! message; `-c old` and `-c compat` are refused (the loader reads only the new format, #941).

pub mod conf;
pub mod scan;
pub mod sysfs;

use alloc::vec::Vec;

use crate::arch::Arch;
use crate::cache;

/// Out of memory (allocation is fallible, S8).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Oom;

/// `v.push(x)` that reports exhaustion instead of aborting (S8).
pub fn push<T>(v: &mut Vec<T>, x: T) -> Result<(), Oom> {
    v.try_reserve(1).map_err(|_| Oom)?;
    #[allow(clippy::disallowed_methods)] // capacity reserved on the line above, so this cannot allocate
    v.push(x);
    Ok(())
}

/// `v.extend_from_slice(s)` that reports exhaustion instead of aborting (S8).
pub fn extend<T: Copy>(v: &mut Vec<T>, s: &[T]) -> Result<(), Oom> {
    v.try_reserve(s.len()).map_err(|_| Oom)?;
    #[allow(clippy::disallowed_methods)] // capacity reserved on the line above, so this cannot allocate
    v.extend_from_slice(s);
    Ok(())
}

/// An output stream being built; an allocation failure is remembered and reported once.
#[derive(Debug, Default)]
pub struct Text {
    /// The bytes written so far.
    pub buf: Vec<u8>,
    /// An append failed for lack of memory.
    pub oom: bool,
}

impl Text {
    /// Appends `b`.
    pub fn bytes(&mut self, b: &[u8]) -> &mut Self {
        if extend(&mut self.buf, b).is_err() {
            self.oom = true;
        }
        self
    }

    /// Appends `n` in decimal.
    pub fn num(&mut self, n: u64) -> &mut Self {
        let mut d = [0u8; 20];
        let mut i = d.len();
        let mut n = n;
        loop {
            i = i.saturating_sub(1);
            if let Some(slot) = d.get_mut(i) {
                *slot = b'0'.saturating_add(u8::try_from(n % 10).unwrap_or(0));
            }
            n /= 10;
            if n == 0 || i == 0 {
                break;
            }
        }
        let digits = d.get(i..).unwrap_or_default();
        self.bytes(digits)
    }
}

/// What a path names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A regular file.
    File,
    /// A directory.
    Dir,
    /// A symbolic link (from [`Fs::lstat`] only).
    Link,
    /// Anything else.
    Other,
}

/// What `stat` tells the engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Meta {
    /// The kind of file.
    pub kind: Kind,
    /// Device and inode: the file's identity.
    pub dev: u64,
    /// See `dev`.
    pub ino: u64,
}

/// Where replacing a file failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// Creating the temporary file `<path>~`.
    Create,
    /// Writing or syncing it.
    Write,
    /// Renaming it over `<path>`.
    Rename,
}

/// The file-system operations ldconfig needs. Paths are as the configuration names them; an
/// implementation with a root resolves every path inside it (a `-r` run). Errors are `errno` values.
pub trait Fs {
    /// The root shown in front of some paths in messages (empty without `-r`).
    fn root(&self) -> &[u8];
    /// Status, following symlinks.
    fn stat(&mut self, path: &[u8]) -> Result<Meta, i32>;
    /// Status of the path itself.
    fn lstat(&mut self, path: &[u8]) -> Result<Meta, i32>;
    /// The names in a directory (any order; `.` and `..` may be left out or not).
    fn list(&mut self, dir: &[u8], out: &mut Vec<Vec<u8>>) -> Result<(), i32>;
    /// Up to `buf.len()` bytes at `offset`; fewer only at the end of the file.
    fn read_at(&mut self, path: &[u8], offset: u64, buf: &mut [u8]) -> Result<usize, i32>;
    /// The whole file, at most `max` bytes.
    fn read_file(&mut self, path: &[u8], max: usize, out: &mut Vec<u8>) -> Result<(), i32>;
    /// A symlink's target.
    fn read_link(&mut self, path: &[u8], out: &mut Vec<u8>) -> Result<(), i32>;
    /// Makes `link` a symlink to `target`, replacing what is there atomically.
    fn replace_link(&mut self, target: &[u8], link: &[u8]) -> Result<(), i32>;
    /// Removes the directory entry `path` (a symlink itself, never what it points to).
    fn remove(&mut self, path: &[u8]) -> Result<(), i32>;
    /// Writes `data` to `<path>~`, syncs it and renames it over `path`.
    fn replace_file(&mut self, path: &[u8], data: &[u8]) -> Result<(), (Stage, i32)>;
    /// The text of an `errno` value (as strerror gives it).
    fn error_text(&self, errno: i32, out: &mut Text);
}

/// The default configuration file.
pub const CONF: &[u8] = b"/etc/ld.so.conf";
/// The default cache.
pub const CACHE: &[u8] = b"/etc/ld.so.cache";
/// The generator text libniji's caches carry (tag 0, spec Appendix D.3.1; D-31.4).
pub const GENERATOR: &[u8] = b"ldconfig (libniji) 2.39";

/// What the command line asks for.
#[derive(Debug, Default)]
pub struct Options<'a> {
    /// `-v`.
    pub verbose: bool,
    /// `-N`: no cache.
    pub no_cache: bool,
    /// `-X`: no links.
    pub no_links: bool,
    /// `-n`: only the operands.
    pub only_operands: bool,
    /// `-p`.
    pub print: bool,
    /// `-C`.
    pub cache: Option<&'a [u8]>,
    /// `-f`.
    pub conf: Option<&'a [u8]>,
    /// `-r`.
    pub root: Option<&'a [u8]>,
    /// `-c`.
    pub format: Option<&'a [u8]>,
    /// `-l`.
    pub library_mode: bool,
    /// The operands.
    pub operands: Vec<&'a [u8]>,
}

/// What [`parse_args`] decided.
#[derive(Debug)]
pub enum Parsed<'a> {
    /// Run with these options.
    Run(Options<'a>),
    /// Print this to stdout and exit with this status (`--help`, `--usage`, `--version`).
    Print(&'static [u8], u8),
    /// A usage error: the message (after `<program>: `), exit 64.
    Usage(Vec<u8>),
}

/// Exit status of a usage error (argp's).
pub const EXIT_USAGE: u8 = 64;

const USAGE: &[u8] = b"Usage: ldconfig [-ilnNpvX?V] [-c FORMAT] [-C CACHE] [-f CONF] [-r ROOT]\n            [--format=FORMAT] [--ignore-aux-cache] [--print-cache] [--verbose]\n            [--help] [--usage] [--version]\n";
const HELP: &[u8] = b"Usage: ldconfig [OPTION...]\nConfigure Dynamic Linker Run Time Bindings.\n\n  -c, --format=FORMAT        Format to use: new (the only one libniji writes)\n  -C CACHE                   Use CACHE as cache file\n  -f CONF                    Use CONF as configuration file\n  -i, --ignore-aux-cache     Ignore auxiliary cache file\n  -l                         Manually link individual libraries.\n  -n                         Only process directories specified on the command\n                             line.  Don't build cache.\n  -N                         Don't build cache\n  -p, --print-cache          Print cache\n  -r ROOT                    Change to and use ROOT as root directory\n  -v, --verbose              Generate verbose messages\n  -X                         Don't update symbolic links\n  -?, --help                 Give this help list\n      --usage                Give a short usage message\n  -V, --version              Print program version\n\nMandatory or optional arguments to long options are also mandatory or optional\nfor any corresponding short options.\n";
const VERSION: &[u8] = b"ldconfig (libniji) 2.39\n";

fn usage_error(what: &[u8], arg: &[u8]) -> Result<Vec<u8>, Oom> {
    let mut m = Vec::new();
    extend(&mut m, what)?;
    extend(&mut m, arg)?;
    Ok(m)
}

/// Parses the arguments after argv[0] (argp conventions: options and operands in any order,
/// clustered short options, `-Cx` and `-C x`, `--long=x` and `--long x`, `--` ends options).
pub fn parse_args<'a>(args: &[&'a [u8]]) -> Result<Parsed<'a>, Oom> {
    let mut o = Options::default();
    let mut i = 0usize;
    let mut operands_only = false;
    while let Some(&a) = args.get(i) {
        i = i.saturating_add(1);
        if operands_only || a == b"-" || a.first() != Some(&b'-') {
            push(&mut o.operands, a)?;
            continue;
        }
        if a == b"--" {
            operands_only = true;
            continue;
        }
        if let Some(long) = a.strip_prefix(b"--") {
            let (name, val) = match long.iter().position(|&c| c == b'=') {
                Some(e) => (long.get(..e).unwrap_or_default(), Some(long.get(e.saturating_add(1)..).unwrap_or_default())),
                None => (long, None),
            };
            const LONGS: [&[u8]; 7] = [b"format", b"ignore-aux-cache", b"print-cache", b"verbose", b"help", b"usage", b"version"];
            let matches: Vec<&[u8]> = LONGS.iter().copied().filter(|l| l.starts_with(name) && !name.is_empty()).collect();
            let full = match matches.as_slice() {
                [one] => *one,
                _ if LONGS.contains(&name) => name,
                [] => return Ok(Parsed::Usage(usage_error(b"unrecognized option '", a).and_then(|mut m| extend(&mut m, b"'").map(|()| m))?)),
                _ => return Ok(Parsed::Usage(usage_error(b"option '", a).and_then(|mut m| extend(&mut m, b"' is ambiguous").map(|()| m))?)),
            };
            if full == b"format" {
                let v = match val {
                    Some(v) => v,
                    None => match args.get(i) {
                        Some(&v) => {
                            i = i.saturating_add(1);
                            v
                        }
                        None => return Ok(Parsed::Usage(usage_error(b"option '--format' requires an argument", b"")?)),
                    },
                };
                o.format = Some(v);
                continue;
            }
            if val.is_some() {
                return Ok(Parsed::Usage(usage_error(b"option '--", full).and_then(|mut m| extend(&mut m, b"' doesn't allow an argument").map(|()| m))?));
            }
            match full {
                b"ignore-aux-cache" => {}
                b"print-cache" => o.print = true,
                b"verbose" => o.verbose = true,
                b"help" => return Ok(Parsed::Print(HELP, 0)),
                b"usage" => return Ok(Parsed::Print(USAGE, 0)),
                _ => return Ok(Parsed::Print(VERSION, 0)),
            }
            continue;
        }
        let mut j = 1usize;
        while let Some(&c) = a.get(j) {
            j = j.saturating_add(1);
            match c {
                b'i' => {}
                b'l' => o.library_mode = true,
                b'n' => o.only_operands = true,
                b'N' => o.no_cache = true,
                b'p' => o.print = true,
                b'v' => o.verbose = true,
                b'X' => o.no_links = true,
                b'?' => return Ok(Parsed::Print(HELP, 0)),
                b'V' => return Ok(Parsed::Print(VERSION, 0)),
                b'c' | b'C' | b'f' | b'r' => {
                    let v = match a.get(j..) {
                        Some(rest) if !rest.is_empty() => rest,
                        _ => match args.get(i) {
                            Some(&v) => {
                                i = i.saturating_add(1);
                                v
                            }
                            None => return Ok(Parsed::Usage(usage_error(b"option requires an argument -- '", &[c, b'\''])?)),
                        },
                    };
                    match c {
                        b'c' => o.format = Some(v),
                        b'C' => o.cache = Some(v),
                        b'f' => o.conf = Some(v),
                        _ => o.root = Some(v),
                    }
                    break;
                }
                _ => return Ok(Parsed::Usage(usage_error(b"invalid option -- '", &[c, b'\''])?)),
            }
        }
    }
    Ok(Parsed::Run(o))
}

/// The result of one run.
#[derive(Debug, Default)]
pub struct Outcome {
    /// Exit status.
    pub status: u8,
    /// Standard output.
    pub out: Text,
    /// Standard error.
    pub err: Text,
}

/// The basename of argv[0] (argp's short program name, used in the `Try` line).
fn short_name(argv0: &[u8]) -> &[u8] {
    argv0.rsplit(|&c| c == b'/').next().unwrap_or(argv0)
}

/// Runs `ldconfig` with `args` (after argv[0]) on `fs`. `generator` is the cache's tag 0 text.
/// `fs` must already resolve paths inside `-r ROOT` when one is given (see [`root_of`]).
pub fn run<A: Arch, F: Fs>(argv0: &[u8], args: &[&[u8]], fs: &mut F, generator: &[u8]) -> Outcome {
    let mut r = Outcome::default();
    let status = match parse_args(args) {
        Err(Oom) => None,
        Ok(Parsed::Print(text, st)) => {
            if text == USAGE || text == HELP {
                r.out.bytes(b"Usage: ").bytes(short_name(argv0)).bytes(text.get(b"Usage: ldconfig".len()..).unwrap_or_default());
            } else {
                r.out.bytes(text);
            }
            Some(st)
        }
        Ok(Parsed::Usage(msg)) => {
            let me = short_name(argv0);
            r.err.bytes(argv0).bytes(b": ").bytes(&msg).bytes(b"\nTry `").bytes(me).bytes(b" --help' or `").bytes(me).bytes(b" --usage' for more information.\n");
            Some(EXIT_USAGE)
        }
        Ok(Parsed::Run(o)) => scan::main::<A, F>(argv0, &o, fs, generator, &mut r),
    };
    r.status = match status {
        Some(s) if !r.out.oom && !r.err.oom => s,
        _ => {
            r.err.oom = false;
            r.err.bytes(argv0).bytes(b": out of memory\n");
            1
        }
    };
    r
}

/// The `-r` argument of `args`, if any, so the front end can build its [`Fs`] before [`run`].
pub fn root_of<'a>(args: &[&'a [u8]]) -> Option<&'a [u8]> {
    match parse_args(args) {
        Ok(Parsed::Run(o)) => o.root,
        _ => None,
    }
}

/// The `-p` label of a flags word (spec §31.2 Guarantees; D.3.1; the UTIL-HARNESS transcripts).
pub fn flags_text(flags: u32, out: &mut Text) {
    if flags & 0xff != 3 {
        out.bytes(b"unknown or unsupported flag");
        return;
    }
    out.bytes(b"libc6");
    let abi = flags & !0xff;
    let label: &[u8] = match abi {
        0 => return,
        0x100 | 0x400 | 0x500 | 0x700 => b"64bit",
        0x300 => b"x86-64",
        0x600 => b"N32",
        0x800 => b"x32",
        0x900 => b"hard-float",
        0xa00 => b"AArch64",
        0xb00 | 0xf00 => b"soft-float",
        0xc00 => b"nan2008",
        0xd00 => b"N32,nan2008",
        _ => {
            out.bytes(b",").num(u64::from(abi));
            return;
        }
    };
    out.bytes(b",").bytes(label);
}

/// `-p`: lists the cache `bytes` read from `name`.
pub fn print_cache(bytes: &[u8], name: &[u8], out: &mut Text) -> bool {
    let Some(c) = cache::Cache::parse(bytes) else { return false };
    out.num(u64::try_from(c.len()).unwrap_or(u64::MAX)).bytes(b" libs found in cache `").bytes(name).bytes(b"'\n");
    for i in 0..c.len() {
        let Some(e) = c.get(i) else { return false };
        out.bytes(b"\t").bytes(e.key).bytes(b" (");
        flags_text(e.flags, out);
        if let Some(h) = e.hwcaps {
            out.bytes(b", hwcap: \"").bytes(h).bytes(b"\"");
        }
        out.bytes(b") => ").bytes(e.path).bytes(b"\n");
    }
    if let Some(g) = c.generator() {
        out.bytes(b"Cache generated by: ").bytes(g).bytes(b"\n");
    }
    true
}
