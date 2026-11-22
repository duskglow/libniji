//! The `localedef` command (spec §31.8): its options, the input search paths, the directory
//! form of the output, and its messages and statuses.
//!
//! [`run`] does all of it over a [`Host`] (the file system as the program sees it), so the
//! shipped program (`niji_boundary::localedef`) and the tests run the same code. A NAME without
//! `/` goes into the locale archive ([`crate::archive`], D.1.6) unless `--no-archive`.

use crate::archive::{self, Archive, ArchiveError};
use crate::{Error, ErrorKind, Files, Kind, TryVec};

/// The input directories (spec §31.8, probe `localedef-help-paths`).
pub const CHARMAPS: &[u8] = b"/usr/share/i18n/charmaps";
/// Where a locale source named without `/` is found.
pub const LOCALES: &[u8] = b"/usr/share/i18n/locales";
/// The default output directory of `--no-archive` with a NAME without `/`.
pub const LOCALE_PATH: &[u8] = b"/usr/lib/locale";
/// The charmap used when `-f` is not given.
pub const DEFAULT_CHARMAP: &[u8] = b"ANSI_X3.4-1968";
/// The largest input file read (and the largest decompressed charmap): far above any real one.
pub const MAX_INPUT: usize = 64 << 20;

/// The file system as `localedef` uses it; errors are `errno` values.
pub trait Host {
    /// The whole file at `path` (at most `max` bytes).
    fn read(&mut self, path: &[u8], max: usize) -> core::result::Result<TryVec<u8>, i32>;
    /// Make the directory `path` (mode 0777 before the umask).
    fn mkdir(&mut self, path: &[u8]) -> core::result::Result<(), i32>;
    /// Write `data` as the file `path`, replacing any file there.
    fn write(&mut self, path: &[u8], data: &[u8]) -> core::result::Result<(), i32>;
    /// The standard input, to its end (at most `max` bytes): the source when `-i` is not given.
    fn read_stdin(&mut self, max: usize) -> core::result::Result<TryVec<u8>, i32>;
    /// Hold an exclusive lock on the directory `dir` until the program exits (the archive's
    /// directory, while the archive is read, changed and replaced).
    fn lock(&mut self, dir: &[u8]) -> core::result::Result<(), i32>;
    /// The description of `errno` (`strerror`'s text).
    fn error_text(&self, errno: i32) -> &'static [u8];
}

/// What a run printed and its exit status.
#[derive(Debug, Default)]
pub struct Outcome {
    /// The standard output.
    pub out: TryVec<u8>,
    /// The standard error.
    pub err: TryVec<u8>,
    /// The exit status: 0, 1 (warnings), 4 (errors, nothing written).
    pub status: u8,
}

/// `EEXIST`, `ENOENT`, `ENOMEM`.
const EEXIST: i32 = 17;
const ENOENT: i32 = 2;
const ENOMEM: i32 = 12;

/// The file of each category in the directory form, by category number (6, `LC_ALL`, has none).
pub const FILES: [&[u8]; 13] = [
    b"LC_CTYPE",
    b"LC_NUMERIC",
    b"LC_TIME",
    b"LC_COLLATE",
    b"LC_MONETARY",
    b"LC_MESSAGES/SYS_LC_MESSAGES",
    b"",
    b"LC_PAPER",
    b"LC_NAME",
    b"LC_ADDRESS",
    b"LC_TELEPHONE",
    b"LC_MEASUREMENT",
    b"LC_IDENTIFICATION",
];

/// The category names, by number.
const NAMES: [&[u8]; 13] = [
    b"LC_CTYPE",
    b"LC_NUMERIC",
    b"LC_TIME",
    b"LC_COLLATE",
    b"LC_MONETARY",
    b"LC_MESSAGES",
    b"",
    b"LC_PAPER",
    b"LC_NAME",
    b"LC_ADDRESS",
    b"LC_TELEPHONE",
    b"LC_MEASUREMENT",
    b"LC_IDENTIFICATION",
];

/// The long options: name, short letter (or 0), whether it takes an argument.
const LONG: &[(&str, u8, bool)] = &[
    ("charmap", b'f', true),
    ("inputfile", b'i', true),
    ("repertoire-map", b'u', true),
    ("force", b'c', false),
    ("no-hard-links", 0, false),
    ("no-warnings", 0, true),
    ("posix", 0, false),
    ("prefix", 0, true),
    ("quiet", 0, false),
    ("verbose", b'v', false),
    ("warnings", 0, true),
    ("add-to-archive", 0, false),
    ("alias-file", b'A', true),
    ("big-endian", 0, false),
    ("delete-from-archive", 0, false),
    ("list-archive", 0, false),
    ("little-endian", 0, false),
    ("no-archive", 0, false),
    ("replace", 0, false),
    ("help", b'?', false),
    ("usage", 0, false),
    ("version", b'V', false),
];

/// The short options that take an argument.
const SHORT_ARG: &[u8] = b"fiuA";
/// The short options without one.
const SHORT_FLAG: &[u8] = b"cv?V";

const TRY: &[u8] = b"Try `localedef --help' or `localedef --usage' for more information.\n";

const USAGE: &[u8] = b"Usage: localedef [-cv?V] [-f FILE] [-i FILE] [-u FILE] [-A FILE]
            [--charmap=FILE] [--inputfile=FILE] [--repertoire-map=FILE]
            [--force] [--no-hard-links] [--no-warnings=<warnings>] [--posix]
            [--prefix=PATH] [--quiet] [--verbose] [--warnings=<warnings>]
            [--add-to-archive] [--alias-file=FILE] [--big-endian]
            [--delete-from-archive] [--list-archive] [--little-endian]
            [--no-archive] [--replace] [--help] [--usage] [--version] NAME
  or:  localedef [OPTION...] [--add-to-archive|--delete-from-archive] FILE...
  or:  localedef [OPTION...] --list-archive [FILE]
";

const HELP: &[u8] = b"Usage: localedef [OPTION...] NAME
  or:  localedef [OPTION...] [--add-to-archive|--delete-from-archive] FILE...
  or:  localedef [OPTION...] --list-archive [FILE]
Compile locale specification

 Input Files:
  -f, --charmap=FILE         Symbolic character names defined in FILE
  -i, --inputfile=FILE       Source definitions are found in FILE
  -u, --repertoire-map=FILE  FILE contains mapping from symbolic names to UCS4
                             values

 Output control:
  -c, --force                Create output even if warning messages were issued
                           \x20
      --no-hard-links        Do not create hard links between installed
                             locales
      --no-warnings=<warnings>   Comma-separated list of warnings to disable;
                             supported warnings are: ascii, intcurrsym
      --posix                Strictly conform to POSIX
      --prefix=PATH          Optional output file prefix
      --quiet                Suppress warnings and information messages
  -v, --verbose              Print more messages
      --warnings=<warnings>  Comma-separated list of warnings to enable;
                             supported warnings are: ascii, intcurrsym

 Archive control:
      --add-to-archive       Add locales named by parameters to archive
  -A, --alias-file=FILE      locale.alias file to consult when making archive
      --big-endian           Generate big-endian output
      --delete-from-archive  Remove locales named by parameters from archive
      --list-archive         List content of archive
      --little-endian        Generate little-endian output
      --no-archive           Don't add new data to archive
      --replace              Replace existing archive content

  -?, --help                 Give this help list
      --usage                Give a short usage message
  -V, --version              Print program version

Mandatory or optional arguments to long options are also mandatory or optional
for any corresponding short options.

System's directory for character maps : /usr/share/i18n/charmaps
\t\t       repertoire maps: /usr/share/i18n/repertoiremaps
\t\t       locale path    : /usr/lib/locale:/usr/share/i18n
";

/// libniji's identity line (D-31.4).
const VERSION: &[u8] = b"localedef (libniji) 2.39\n";

/// The options of one run.
#[derive(Default)]
struct Opts<'a> {
    charmap: Option<&'a [u8]>,
    input: Option<&'a [u8]>,
    prefix: Option<&'a [u8]>,
    no_archive: bool,
    replace: bool,
    force: bool,
    big_endian: bool,
    archive_mode: Option<&'static str>,
    quiet: bool,
    names: TryVec<&'a [u8]>,
}

/// A failed allocation while printing: the run reports it and stops.
struct NoMem;

fn put(v: &mut TryVec<u8>, parts: &[&[u8]]) -> core::result::Result<(), NoMem> {
    for p in parts {
        v.extend_from_slice(p).map_err(|_| NoMem)?;
    }
    Ok(())
}

fn join(parts: &[&[u8]]) -> core::result::Result<TryVec<u8>, NoMem> {
    let mut v = TryVec::new();
    put(&mut v, parts)?;
    Ok(v)
}

/// Run `localedef` with the arguments after argv\[0\].
pub fn run(args: &[&[u8]], host: &mut dyn Host) -> Outcome {
    let mut o = Outcome::default();
    if body(args, host, &mut o).is_err() {
        o.err.clear();
        let _ = o.err.extend_from_slice(b"localedef: memory exhausted\n");
        o.status = 4;
    }
    o
}

/// What the option parser decided.
enum Parsed<'a> {
    Run(Opts<'a>),
    /// Print `out` / `err` and exit with `status`.
    Done(&'static [u8], &'static [u8], u8),
}

fn usage_error(o: &mut Outcome, parts: &[&[u8]]) -> core::result::Result<(), NoMem> {
    put(&mut o.err, parts)?;
    put(&mut o.err, &[b"\n", TRY])?;
    o.status = 4;
    Ok(())
}

fn parse<'a>(args: &[&'a [u8]], o: &mut Outcome) -> core::result::Result<Option<Parsed<'a>>, NoMem> {
    let mut opts = Opts::default();
    let mut i = 0;
    let mut operands_only = false;
    while let Some(&a) = args.get(i) {
        i = i.saturating_add(1);
        if operands_only || a.len() < 2 || a.first() != Some(&b'-') {
            opts.names.push(a).map_err(|_| NoMem)?;
            continue;
        }
        if a == b"--" {
            operands_only = true;
            continue;
        }
        if let Some(long) = a.strip_prefix(b"--") {
            let (name, value) = match long.iter().position(|&b| b == b'=') {
                Some(p) => (long.get(..p).unwrap_or_default(), long.get(p.saturating_add(1)..)),
                None => (long, None),
            };
            let exact = LONG.iter().find(|l| l.0.as_bytes() == name);
            let found = match exact {
                Some(l) => *l,
                None => {
                    let mut hits = LONG.iter().filter(|l| l.0.as_bytes().starts_with(name));
                    match (hits.next(), hits.next()) {
                        (Some(l), None) => *l,
                        (None, _) => {
                            usage_error(o, &[b"localedef: unrecognized option '", a, b"'"])?;
                            return Ok(None);
                        }
                        (Some(_), Some(_)) => {
                            put(&mut o.err, &[b"localedef: option '", a, b"' is ambiguous; possibilities:"])?;
                            for l in LONG.iter().filter(|l| l.0.as_bytes().starts_with(name)) {
                                put(&mut o.err, &[b" '--", l.0.as_bytes(), b"'"])?;
                            }
                            usage_error(o, &[])?;
                            return Ok(None);
                        }
                    }
                }
            };
            let (lname, short, takes) = found;
            let value = match (takes, value) {
                (false, Some(_)) => {
                    usage_error(o, &[b"localedef: option '--", lname.as_bytes(), b"' doesn't allow an argument"])?;
                    return Ok(None);
                }
                (true, None) => match args.get(i) {
                    Some(&v) => {
                        i = i.saturating_add(1);
                        Some(v)
                    }
                    None => {
                        usage_error(o, &[b"localedef: option '--", lname.as_bytes(), b"' requires an argument"])?;
                        return Ok(None);
                    }
                },
                (_, v) => v,
            };
            if let Some(p) = apply(&mut opts, short, lname, value) {
                return Ok(Some(p));
            }
            continue;
        }
        // A cluster of short options; one that takes an argument ends it.
        let mut k = 1;
        while let Some(&c) = a.get(k) {
            k = k.saturating_add(1);
            if SHORT_ARG.contains(&c) {
                let value = match a.get(k..).filter(|r| !r.is_empty()) {
                    Some(r) => r,
                    None => match args.get(i) {
                        Some(&v) => {
                            i = i.saturating_add(1);
                            v
                        }
                        None => {
                            usage_error(o, &[b"localedef: option requires an argument -- '", &[c], b"'"])?;
                            return Ok(None);
                        }
                    },
                };
                if let Some(p) = apply(&mut opts, c, "", Some(value)) {
                    return Ok(Some(p));
                }
                break;
            }
            if !SHORT_FLAG.contains(&c) {
                usage_error(o, &[b"localedef: invalid option -- '", &[c], b"'"])?;
                return Ok(None);
            }
            if let Some(p) = apply(&mut opts, c, "", None) {
                return Ok(Some(p));
            }
        }
    }
    Ok(Some(Parsed::Run(opts)))
}

/// Record one option; `Some` when it ends the run at once (help, usage, version).
fn apply<'a>(opts: &mut Opts<'a>, short: u8, long: &str, value: Option<&'a [u8]>) -> Option<Parsed<'a>> {
    match (short, long) {
        (b'f', _) => opts.charmap = value,
        (b'i', _) => opts.input = value,
        (b'c', _) => opts.force = true,
        (b'?', _) => return Some(Parsed::Done(HELP, b"", 0)),
        (b'V', _) => return Some(Parsed::Done(VERSION, b"", 0)),
        (_, "usage") => return Some(Parsed::Done(USAGE, b"", 0)),
        (_, "prefix") => opts.prefix = value,
        (_, "no-archive") => opts.no_archive = true,
        (_, "replace") => opts.replace = true,
        (_, "big-endian") => opts.big_endian = true,
        (_, "little-endian") => opts.big_endian = false,
        (_, "quiet") => opts.quiet = true,
        (_, "add-to-archive") => opts.archive_mode = Some("add-to-archive"),
        (_, "delete-from-archive") => opts.archive_mode = Some("delete-from-archive"),
        (_, "list-archive") => opts.archive_mode = Some("list-archive"),
        // Accepted, with no effect on the directory form: -u (characters are named by the charmap), -v, -A, --no-hard-links,
        // --posix, --replace, --warnings, --no-warnings.
        _ => {}
    }
    None
}

/// The input files of one compile, read through the host; the first failure is kept for the message.
struct Inputs<'h> {
    host: &'h mut dyn Host,
    failed: Option<(Kind, i32)>,
}

impl Inputs<'_> {
    fn read_path(&mut self, kind: Kind, parts: &[&[u8]]) -> core::result::Result<TryVec<u8>, i32> {
        let path = join(parts).map_err(|_| ENOMEM)?;
        let r = self.host.read(&path, MAX_INPUT);
        if let Err(e) = r {
            self.failed.get_or_insert((kind, e));
        }
        r
    }
}

impl Files for Inputs<'_> {
    fn read(&mut self, kind: Kind, name: &[u8]) -> crate::Result<TryVec<u8>> {
        let missing = |e: i32| if e == ENOMEM { Error::from(niji::error::Errno::ENOMEM) } else { Error::missing("an input file cannot be read") };
        let has_slash = name.contains(&b'/');
        match kind {
            Kind::Locale if has_slash => self.read_path(kind, &[name]).map_err(missing),
            Kind::Locale => self.read_path(kind, &[LOCALES, b"/", name]).map_err(missing),
            Kind::Charmap if has_slash => self.read_path(kind, &[name]).map_err(missing),
            Kind::Charmap => match self.read_path(kind, &[CHARMAPS, b"/", name]) {
                Ok(t) => Ok(t),
                Err(first) => {
                    let gz = self.read_path(kind, &[CHARMAPS, b"/", name, b".gz"]).map_err(|_| missing(first))?;
                    self.failed = None;
                    niji::charmap::inflate::gunzip(&gz, MAX_INPUT).map_err(|_| Error::syntax(0, "the compressed charmap is damaged"))
                }
            },
        }
    }
}

fn body(args: &[&[u8]], host: &mut dyn Host, o: &mut Outcome) -> core::result::Result<(), NoMem> {
    let opts = match parse(args, o)? {
        None => return Ok(()),
        Some(Parsed::Done(out, err, status)) => {
            put(&mut o.out, &[out])?;
            put(&mut o.err, &[err])?;
            o.status = status;
            return Ok(());
        }
        Some(Parsed::Run(opts)) => opts,
    };
    if let Some(mode) = opts.archive_mode {
        return archive_command(mode, &opts, host, o);
    }
    let [name] = opts.names.as_slice() else {
        put(&mut o.out, &[TRY])?;
        o.status = 4;
        return Ok(());
    };
    let name = *name;
    if opts.big_endian {
        put(&mut o.err, &[b"[error] big-endian output is not supported\n"])?;
        o.status = 4;
        return Ok(());
    }
    // Where the locale goes: the directory NAME (with `/`), a directory under the locale path named with the
    // codeset normalized (--no-archive), or the archive.
    let parts = niji::locale::name::parse(name);
    let stored = niji::locale::name::archive_form(&parts);
    let dir = if name.contains(&b'/') {
        Some(join(&[name])?)
    } else if opts.no_archive {
        Some(join(&[opts.prefix.unwrap_or_default(), LOCALE_PATH, b"/", stored.as_bytes()])?)
    } else {
        None
    };
    let shown: &[u8] = opts.input.unwrap_or(b"<stdin>");
    // The output directory is made before any input is read (probed: it is left, empty, when the run then fails);
    // a failure here is reported when the files are written.
    if let Some(dir) = &dir {
        let _ = host.mkdir(dir.strip_suffix(b"/").unwrap_or(dir));
    }
    let mut charmap = opts.charmap.unwrap_or(DEFAULT_CHARMAP);
    let mut inputs = Inputs { host, failed: None };
    let mut errors = false;
    // The source is read first, so a missing one is reported as such whatever the charmap; without -i it is
    // the standard input.
    let text = match opts.input {
        Some(input) => match Files::read(&mut inputs, Kind::Locale, input) {
            Ok(t) => t,
            Err(e) => {
                if e.kind == ErrorKind::NoMemory {
                    return Err(NoMem);
                }
                let errno = inputs.failed.map_or(ENOENT, |f| f.1);
                put(&mut o.err, &[b"[error] cannot open locale definition file `", input, b"': ", inputs.host.error_text(errno), b"\n"])?;
                o.status = 4;
                return Ok(());
            }
        },
        None => match inputs.host.read_stdin(MAX_INPUT) {
            Ok(t) => t,
            Err(ENOMEM) => return Err(NoMem),
            Err(errno) => {
                put(&mut o.err, &[b"[error] cannot read the locale definition from standard input: ", inputs.host.error_text(errno), b"\n"])?;
                o.status = 4;
                return Ok(());
            }
        },
    };
    inputs.failed = None;
    // A charmap that cannot be read is an error; the default one stands in for it (probed), so the source is
    // still checked, and with -c compiled.
    let mut map = Files::read(&mut inputs, Kind::Charmap, charmap);
    if let Err(e) = &map {
        if e.kind == ErrorKind::NoMemory {
            return Err(NoMem);
        }
        let errno = inputs.failed.map_or(ENOENT, |f| f.1);
        let default = if charmap == DEFAULT_CHARMAP { &b"default "[..] } else { b"" };
        put(&mut o.err, &[b"[error] ", default, b"character map file `", charmap, b"' not found: ", inputs.host.error_text(errno), b"\n"])?;
        errors = true;
        inputs.failed = None;
        if charmap != DEFAULT_CHARMAP {
            charmap = DEFAULT_CHARMAP;
            map = Files::read(&mut inputs, Kind::Charmap, charmap);
            if let Err(e) = &map {
                if e.kind == ErrorKind::NoMemory {
                    return Err(NoMem);
                }
                let errno = inputs.failed.map_or(ENOENT, |f| f.1);
                put(&mut o.err, &[b"[error] default character map file `", charmap, b"' not found: ", inputs.host.error_text(errno), b"\n"])?;
            }
        }
        inputs.failed = None;
    }
    let mut line = [0u8; 10];
    let Ok(map) = map else {
        put(&mut o.err, &[b"[error] no output file produced because errors were issued\n"])?;
        o.status = 4;
        return Ok(());
    };
    let cs = match crate::charset::Charset::parse(&map) {
        Ok(cs) => cs,
        Err(e) if e.kind == ErrorKind::NoMemory => return Err(NoMem),
        Err(e) => {
            put(&mut o.err, &[charmap, b":", decimal(e.line, &mut line), b": ", e.what.as_bytes(), b"\n"])?;
            put(&mut o.err, &[b"[error] no output file produced because errors were issued\n"])?;
            o.status = 4;
            return Ok(());
        }
    };
    let out = match crate::compile_text(&text, &cs, &mut inputs) {
        Ok(out) => out,
        Err(e) => {
            match (e.kind, inputs.failed) {
                (ErrorKind::NoMemory, _) => return Err(NoMem),
                (ErrorKind::Missing, Some((Kind::Locale, errno))) => {
                    put(&mut o.err, &[b"[error] cannot open a locale definition file the source names (copy or include): ", inputs.host.error_text(errno), b"\n"])?;
                }
                _ => {
                    put(&mut o.err, &[shown, b":", decimal(e.line, &mut line), b": ", e.what.as_bytes(), b"\n"])?;
                }
            }
            put(&mut o.err, &[b"[error] no output file produced because errors were issued\n"])?;
            o.status = 4;
            return Ok(());
        }
    };
    // Syntax errors the compile went past (#7534, probed): a line outside any category is skipped and the
    // locale still written, status 1; a category with an error needs -c (it gets its default), status 1.
    for e in out.errors.iter() {
        put(&mut o.err, &[shown, b":", decimal(e.line, &mut line), b": ", e.what.as_bytes(), b"\n"])?;
        o.status = 1;
    }
    let category_errors = out.errors.len() > out.outside;
    let host = inputs.host;
    for (n, defined) in out.defined.iter().enumerate() {
        if !defined && n != 6 && !opts.quiet {
            put(&mut o.err, &[b"[warning] No definition for ", NAMES.get(n).copied().unwrap_or_default(), b" category found\n"])?;
            o.status = 1;
        }
    }
    if (errors || category_errors) && !opts.force {
        put(&mut o.err, &[b"[error] no output file produced because errors were issued\n"])?;
        o.status = 4;
        return Ok(());
    }
    match dir {
        Some(dir) => write_dir(host, &dir, &out.files, o),
        None => {
            // A NAME without a codeset is stored under the charmap's too (probed: `en_US` with ISO-8859-1 is
            // `en_US` and `en_US.iso88591`, one locale).
            let with_codeset = niji::locale::name::archive_form(&niji::locale::name::Parts { codeset: Some(cs.name()), ..parts });
            let mut names: TryVec<&[u8]> = TryVec::new();
            names.push(stored.as_bytes()).map_err(|_| NoMem)?;
            if parts.codeset.is_none() && !cs.name().is_empty() {
                names.push(with_codeset.as_bytes()).map_err(|_| NoMem)?;
            }
            let mut images: archive::Images = Default::default();
            for (img, file) in images.iter_mut().zip(out.files) {
                if let Some(f) = file {
                    *img = f;
                }
            }
            let path = archive_path(&opts)?;
            let _ = host.lock(&path_dir(&opts)?);
            let Some(mut a) = load_for_change(host, &path, o)? else { return Ok(()) };
            a.add(&names, images).map_err(|_| NoMem)?;
            store(host, &path, &a, o)
        }
    }
}

/// The largest archive read.
const ARCHIVE_MAX: usize = 1 << 30;

/// The archive's directory: the locale path after `--prefix`.
fn path_dir(opts: &Opts<'_>) -> core::result::Result<TryVec<u8>, NoMem> {
    join(&[opts.prefix.unwrap_or_default(), LOCALE_PATH])
}

/// The archive's file.
fn archive_path(opts: &Opts<'_>) -> core::result::Result<TryVec<u8>, NoMem> {
    join(&[opts.prefix.unwrap_or_default(), LOCALE_PATH, b"/locale-archive"])
}

/// The archive at `path` to change: a missing one is empty; one that cannot be read or is damaged
/// is an `[error]` line, status 4, `None`.
fn load_for_change(host: &mut dyn Host, path: &[u8], o: &mut Outcome) -> core::result::Result<Option<Archive>, NoMem> {
    let why: &[u8] = match host.read(path, ARCHIVE_MAX) {
        Err(ENOENT) => return Ok(Some(Archive::default())),
        Err(ENOMEM) => return Err(NoMem),
        Err(e) => host.error_text(e),
        Ok(d) => match Archive::parse(&d) {
            Ok(a) => return Ok(Some(a)),
            Err(ArchiveError::NoMemory) => return Err(NoMem),
            Err(ArchiveError::Damaged) => b"the file is damaged",
        },
    };
    put(&mut o.err, &[b"[error] cannot open locale archive \"", path, b"\": ", why, b"\n"])?;
    o.status = 4;
    Ok(None)
}

/// Replace the archive at `path` with `a` (probed message when its directory is missing: status 1).
fn store(host: &mut dyn Host, path: &[u8], a: &Archive, o: &mut Outcome) -> core::result::Result<(), NoMem> {
    let data = a.write().map_err(|_| NoMem)?;
    if let Err(e) = host.write(path, &data) {
        put(&mut o.err, &[b"cannot create temporary file: ", path, b".XXXXXX: ", host.error_text(e), b"\n"])?;
        o.status = 1;
    }
    Ok(())
}

/// `--list-archive`, `--add-to-archive`, `--delete-from-archive` (probed statuses and messages).
fn archive_command(mode: &str, opts: &Opts<'_>, host: &mut dyn Host, o: &mut Outcome) -> core::result::Result<(), NoMem> {
    let path = archive_path(opts)?;
    if mode == "list-archive" {
        // An operand names the archive (the rest are ignored); a missing default archive lists nothing.
        let (file, named) = match opts.names.first() {
            Some(f) => (join(&[f])?, true),
            None => (path, false),
        };
        let why: &[u8] = match host.read(&file, ARCHIVE_MAX) {
            Err(ENOENT) if !named => return Ok(()),
            Err(ENOMEM) => return Err(NoMem),
            Err(e) => host.error_text(e),
            Ok(d) => match Archive::parse(&d) {
                Ok(a) => {
                    for n in a.names().map_err(|_| NoMem)?.iter() {
                        put(&mut o.out, &[n, b"\n"])?;
                    }
                    return Ok(());
                }
                Err(ArchiveError::NoMemory) => return Err(NoMem),
                Err(ArchiveError::Damaged) => b"the file is damaged",
            },
        };
        put(&mut o.err, &[b"cannot open locale archive \"", &file, b"\": ", why, b"\n"])?;
        o.status = 1;
        return Ok(());
    }
    if opts.names.is_empty() {
        return Ok(());
    }
    let _ = host.lock(&path_dir(opts)?);
    let Some(mut a) = load_for_change(host, &path, o)? else { return Ok(()) };
    let mut changed = false;
    for &operand in opts.names.iter() {
        if mode == "delete-from-archive" {
            // Exact names (no normalization); a missing one is a message, status unchanged.
            if a.remove(operand) {
                changed = true;
            } else {
                put(&mut o.err, &[b"locale \"", operand, b"\" not in archive\n"])?;
            }
            continue;
        }
        put(&mut o.out, &[b"Adding ", operand, b"\n"])?;
        let base = operand.strip_suffix(b"/").unwrap_or(operand);
        let last = base.rsplit(|&b| b == b'/').next().unwrap_or(base);
        let stored = niji::locale::name::archive_form(&niji::locale::name::parse(last));
        if a.contains(stored.as_bytes()) && !opts.replace {
            put(&mut o.err, &[b"locale '", stored.as_bytes(), b"' already exists\n"])?;
            o.status = 255;
            continue;
        }
        let mut images: archive::Images = Default::default();
        let mut first_error = None;
        let mut found = false;
        for (img, file) in images.iter_mut().zip(FILES) {
            if file.is_empty() {
                continue;
            }
            match host.read(&join(&[base, b"/", file])?, MAX_INPUT) {
                Ok(d) => {
                    *img = d;
                    found = true;
                }
                Err(ENOMEM) => return Err(NoMem),
                Err(e) => {
                    first_error.get_or_insert(e);
                }
            }
        }
        if !found {
            let e = first_error.unwrap_or(ENOENT);
            put(&mut o.err, &[b"stat of \"", operand, b"\" failed: ", host.error_text(e), b": ignored\n"])?;
            continue;
        }
        a.add(&[stored.as_bytes()], images).map_err(|_| NoMem)?;
        changed = true;
    }
    if changed {
        store(host, &path, &a, o)?;
    }
    Ok(())
}

/// Write the directory form: `dir` itself (one level: its parent must exist) and a file per category.
fn write_dir(host: &mut dyn Host, dir: &[u8], compiled: &crate::Compiled, o: &mut Outcome) -> core::result::Result<(), NoMem> {
    let fail = |o: &mut Outcome, host: &dyn Host, shown: &[u8], errno: i32| -> core::result::Result<(), NoMem> {
        put(&mut o.err, &[b"[error] cannot write output files to `", shown, b"': ", host.error_text(errno), b"\n"])?;
        o.status = 4;
        Ok(())
    };
    let base = dir.strip_suffix(b"/").unwrap_or(dir);
    if let Err(e) = host.mkdir(base) {
        if e != EEXIST {
            return fail(o, host, base, e);
        }
    }
    if compiled.get(5).is_some_and(Option::is_some) {
        if let Err(e) = host.mkdir(&join(&[base, b"/LC_MESSAGES"])?) {
            if e != EEXIST {
                return fail(o, host, &join(&[base, b"/"])?, e);
            }
        }
    }
    for (n, file) in compiled.iter().enumerate() {
        let Some(data) = file else { continue };
        let path = join(&[base, b"/", FILES.get(n).copied().unwrap_or_default()])?;
        if let Err(e) = host.write(&path, data.as_slice()) {
            return fail(o, host, &join(&[base, b"/"])?, e);
        }
    }
    Ok(())
}

/// `n` in decimal, in `buf`.
fn decimal(mut n: u32, buf: &mut [u8; 10]) -> &[u8] {
    let mut at = buf.len();
    while at > 0 {
        at = at.saturating_sub(1);
        if let (Some(d), Some(&digit)) = (buf.get_mut(at), b"0123456789".get(usize::try_from(n % 10).unwrap_or(0))) {
            *d = digit;
        }
        n /= 10;
        if n == 0 {
            break;
        }
    }
    buf.get(at..).unwrap_or_default()
}
