//! `ld.so` run as a program (plan/ph14/design.md §19, ld.so(8)): what its arguments ask for, and the
//! text of `--version` and `--help` (phase 14 DL-5, D-27.13). Pure logic; the boundary writes the text.
//!
//! DL-5 added `--version` and `--help`; DL-6 (D-27.14) runs `ld.so PROGRAM [ARGS]`: a position-independent
//! program without dependencies (the boundary maps and enters it, [`crate::start`], [`crate::map`]).

use crate::arch::Arch;

/// The exit status of a refused start (design §1 "Failure").
pub const FAILURE: i32 = 127;

/// The exit status of a command line `ld.so` cannot use: no program named, an unknown option or an
/// option without its value (probed: 1).
pub const USAGE: i32 = 1;

/// What `ld.so` does with the program it names (ld.so(8)).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Run it (DL-6).
    Run,
    /// `--list`: list its dependencies as `LD_TRACE_LOADED_OBJECTS` does (DL-8).
    List,
    /// `--verify`: say by the exit status alone whether it is a dynamically linked program
    /// ([`verify_status`]).
    Verify,
}

/// The options before the program's name (ld.so(8); LDSO-RUN-PROGRAM probe). Each option that takes a
/// value takes the next argument (`--library-path=DIR` is an unknown option); a later copy of an
/// option replaces an earlier one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Options<'a> {
    /// `--library-path PATH`: used instead of `LD_LIBRARY_PATH` (which stays in the environment).
    pub library_path: Option<&'a [u8]>,
    /// `--preload LIST`: objects preloaded after the `LD_PRELOAD` ones, before the preload file's
    /// (colons or spaces separate them, as in `LD_PRELOAD`); the environment is not changed.
    pub preload: Option<&'a [u8]>,
    /// `--argv0 STRING`: the program's `argv[0]`.
    pub argv0: Option<&'a [u8]>,
    /// `--audit LIST`: auditors. libniji's loader has no rtld-audit interface: each named object
    /// gets the "cannot be loaded as audit interface" diagnostic and the program runs without it.
    pub audit: Option<&'a [u8]>,
    /// `--inhibit-cache`: the loader cache is not read.
    pub inhibit_cache: bool,
    /// `--inhibit-rpath LIST`: the objects whose `DT_RPATH` and `DT_RUNPATH` no search uses
    /// ([`rpath_inhibited`]).
    pub inhibit_rpath: Option<&'a [u8]>,
    /// `--glibc-hwcaps-prepend LIST`: `glibc-hwcaps` subdirectories searched before the built-in
    /// ones ([`hwcaps_subdirs`]).
    pub hwcaps_prepend: Option<&'a [u8]>,
    /// `--glibc-hwcaps-mask LIST`: the built-in subdirectories searched, limited to those named.
    pub hwcaps_mask: Option<&'a [u8]>,
}

/// What the command line asks of `ld.so`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action<'a> {
    /// `--version`: print the version and exit 0.
    Version,
    /// `--help`: print the usage and exit 0.
    Help,
    /// No program named: the usage hint on standard error, status [`USAGE`].
    Missing,
    /// An option this loader does not know, or one whose value is missing: status [`USAGE`].
    Unknown(&'a [u8]),
    /// `--list-tunables` (the last of it and the program modes): list the tunables and exit 0; a
    /// program named after it is not opened (probed).
    Tunables,
    /// `--list-diagnostics`: list the loader's diagnostics and exit 0, as for [`Action::Tunables`].
    Diagnostics,
    /// `PROGRAM`, the argument at index `at` (from 0, after `argv[0]`), with `mode` and the options
    /// before it.
    Program {
        /// Run, list or verify.
        mode: Mode,
        /// The program's name as given.
        name: &'a [u8],
        /// Its index among the arguments after `argv[0]`.
        at: usize,
        /// The options before it.
        options: Options<'a>,
    },
}

/// What the command line's last mode option asks for: a program mode, or one of the two lists that
/// need no program (probed: `--list-tunables --list` without a program is "missing program name",
/// `--list --list-tunables PROGRAM` lists the tunables).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Ask {
    Program(Mode),
    Tunables,
    Diagnostics,
}

/// The action for the arguments after `argv[0]`. Options are read up to the first argument that does
/// not start with `-`, the program's name; everything after it is the program's. `--version` and
/// `--help` end the scan where they stand.
pub fn action<'a>(args: impl IntoIterator<Item = &'a [u8]>) -> Action<'a> {
    let mut args = args.into_iter();
    let mut options = Options::default();
    let mut ask = Ask::Program(Mode::Run);
    let mut at = 0usize;
    while let Some(arg) = args.next() {
        let slot = match arg {
            b"--version" => return Action::Version,
            b"--help" => return Action::Help,
            b"--list" => {
                ask = Ask::Program(Mode::List);
                None
            }
            b"--verify" => {
                ask = Ask::Program(Mode::Verify);
                None
            }
            b"--list-tunables" => {
                ask = Ask::Tunables;
                None
            }
            b"--list-diagnostics" => {
                ask = Ask::Diagnostics;
                None
            }
            b"--inhibit-cache" => {
                options.inhibit_cache = true;
                None
            }
            b"--library-path" => Some(&mut options.library_path),
            b"--preload" => Some(&mut options.preload),
            b"--argv0" => Some(&mut options.argv0),
            b"--audit" => Some(&mut options.audit),
            b"--inhibit-rpath" => Some(&mut options.inhibit_rpath),
            b"--glibc-hwcaps-prepend" => Some(&mut options.hwcaps_prepend),
            b"--glibc-hwcaps-mask" => Some(&mut options.hwcaps_mask),
            [b'-', ..] => return Action::Unknown(arg),
            name => {
                return match ask {
                    Ask::Program(mode) => Action::Program { mode, name, at, options },
                    Ask::Tunables => Action::Tunables,
                    Ask::Diagnostics => Action::Diagnostics,
                };
            }
        };
        if let Some(slot) = slot {
            let Some(value) = args.next() else { return Action::Unknown(arg) };
            *slot = Some(value);
            at = at.wrapping_add(1);
        }
        at = at.wrapping_add(1);
    }
    match ask {
        Ask::Program(_) => Action::Missing,
        Ask::Tunables => Action::Tunables,
        Ask::Diagnostics => Action::Diagnostics,
    }
}

/// The entries of a colon-separated option value (`--inhibit-rpath`, the hwcaps options).
fn entries(list: &[u8]) -> impl Iterator<Item = &[u8]> {
    list.split(|&b| b == b':')
}

/// Whether `--inhibit-rpath LIST` names the object loaded as `path`: an entry equal to the path the
/// object was loaded by (bytes, no normalisation; the program's is empty), or a first entry that is
/// empty for the program (probed: `''` and `':'` name the program, `'x:'` does not; a soname and a
/// `..` spelling name nothing).
pub fn rpath_inhibited(list: &[u8], path: &[u8]) -> bool {
    let mut each = entries(list);
    each.next().is_some_and(|first| first == path) || (!path.is_empty() && each.any(|e| e == path))
}

/// The `glibc-hwcaps` subdirectories a search tries, highest first: `--glibc-hwcaps-prepend`'s names
/// in order, then the supported ones (`builtin`, [`Arch::hwcaps`]) that `--glibc-hwcaps-mask` names
/// (all of them without the option). Empty names are dropped, so `''` adds or keeps none (probed).
pub fn hwcaps_subdirs<'a>(builtin: &'a [&'a [u8]], prepend: Option<&'a [u8]>, mask: Option<&'a [u8]>) -> impl Iterator<Item = &'a [u8]> {
    let kept = move |h: &&[u8]| mask.is_none_or(|m| entries(m).any(|e| e == *h));
    entries(prepend.unwrap_or_default()).filter(|e| !e.is_empty()).chain(builtin.iter().copied().filter(kept))
}

/// `n` in decimal, or in hexadecimal with `0x` when `hex` (the forms of `--list-tunables` and
/// `--list-diagnostics`), written at the end of `buf`. Panic-free: the loader's every panic site is
/// a relocation each start pays for.
pub fn number(n: u64, hex: bool, buf: &mut [u8; 24]) -> &[u8] {
    let radix = if hex { 16 } else { 10 };
    let mut rest = n;
    let mut at = buf.len();
    loop {
        let d = u8::try_from(rest.checked_rem(radix).unwrap_or(0)).unwrap_or(0);
        at = at.wrapping_sub(1);
        if let Some(b) = buf.get_mut(at) {
            *b = if d < 10 { b'0'.wrapping_add(d) } else { b'a'.wrapping_add(d.wrapping_sub(10)) };
        }
        rest = rest.checked_div(radix).unwrap_or(0);
        if rest == 0 {
            break;
        }
    }
    if hex {
        for c in *b"x0" {
            at = at.wrapping_sub(1);
            if let Some(b) = buf.get_mut(at) {
                *b = c;
            }
        }
    }
    buf.get(at..).unwrap_or_default()
}

/// The quoted form of a string in `--list-diagnostics`, one piece per byte: `"` and `\` escaped by a
/// backslash, a byte outside printable ASCII as a backslash and its three octal digits.
pub fn quoted_byte(b: u8, out: &mut [u8; 4]) -> &[u8] {
    match b {
        b'"' | b'\\' => {
            *out = [b'\\', b, 0, 0];
            out.get(..2).unwrap_or_default()
        }
        0x20..=0x7e => {
            *out = [b, 0, 0, 0];
            out.get(..1).unwrap_or_default()
        }
        _ => {
            *out = [b'\\', b'0'.wrapping_add(b >> 6), b'0'.wrapping_add((b >> 3) & 7), b'0'.wrapping_add(b & 7)];
            out
        }
    }
}

/// The exit status of `ld.so --verify` for the program file `file` (LDSO-RUN-PROGRAM probe): 0 for an
/// object with a `PT_INTERP` and a `PT_DYNAMIC` segment (a dynamically linked program), 2 for one with
/// `PT_DYNAMIC` and no `PT_INTERP` (a shared object or a static PIE), 1 for anything else (a static
/// program, a file that is not an ELF object of this machine, or none). Nothing is written.
pub fn verify_status<A: Arch>(file: Option<&[u8]>) -> i32 {
    let Some(elf) = file.and_then(|f| crate::elf::Elf::parse::<A>(f).ok()) else { return 1 };
    let has = |t: u32| elf.program_headers().any(|ph| ph.p_type == t);
    match (has(crate::elf::pt::DYNAMIC), has(crate::elf::pt::INTERP)) {
        (true, true) => 0,
        (true, false) => 2,
        (false, _) => 1,
    }
}

/// Each object named in the `--audit` list `list` (colons or spaces separate them), in order.
pub fn audit_names(list: &[u8]) -> impl Iterator<Item = &[u8]> {
    list.split(|&b| b == b':' || b == b' ').filter(|n| !n.is_empty())
}

/// The diagnostic for the auditor `name` (`--audit`), in the order it is written: the loader has no
/// rtld-audit interface, so the object is not loaded and the program runs without it.
pub fn audit_ignored(name: &[u8]) -> [&[u8]; 3] {
    [b"ERROR: ld.so: object '", name, b"' cannot be loaded as audit interface: libniji's loader has no audit interface; ignored.\n"]
}

/// Where `ld.so PROGRAM` (and `--list PROGRAM`, `LD_TRACE_LOADED_OBJECTS`) finds the program file
/// (D-27.69, black-box probes of the reference, LDSO-PROGRAM-NO-CWD).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Found<'a> {
    /// The name holds a `/`: it is the path, opened as given (relative to the working directory
    /// when it does not start with `/`; the caller chose that by writing the `/`).
    AsGiven,
    /// A name without `/` that the loader cache maps to this absolute path.
    Cache(&'a [u8]),
    /// A name without `/` that the cache does not map to an absolute path: the start ends with
    /// `NAME: error while loading shared libraries: NAME: cannot open shared object file`, 127.
    NotFound,
}

/// Find the program named `name`. A name without `/` is looked up in the loader cache only (`flags`:
/// [`Arch::CACHE_FLAGS`]; `hwcaps`: the supported subdirectories, as for a library): never the
/// working directory, `LD_LIBRARY_PATH`, `PATH` or a default directory. The cache reader takes only
/// absolute paths (a relative one is an inconsistent entry), so a malformed cache cannot turn the
/// lookup into a working-directory open.
pub fn find<'a>(name: &[u8], cache: Option<&crate::cache::Cache<'a>>, flags: u32, hwcaps: &[&[u8]]) -> Found<'a> {
    if name.contains(&b'/') {
        return Found::AsGiven;
    }
    cache.and_then(|c| c.lookup(name, flags, hwcaps)).map_or(Found::NotFound, Found::Cache)
}

/// The `--version` text, in the order it is written: libniji's name and version, the ABI it
/// implements and the interpreter's own name ([`Arch::INTERP`]).
pub fn version<A: Arch>(niji_version: &'static str) -> [&'static str; 5] {
    ["ld.so (libniji) ", niji_version, "\nImplements the GNU C Library 2.39 ABI.\nProgram interpreter: ", A::INTERP, "\n"]
}

/// The `--help` text, in the order it is written.
pub fn help<A: Arch>() -> [&'static str; 3] {
    [
        "Usage: ",
        A::INTERP,
        " [OPTION]... EXECUTABLE-FILE [ARGS-FOR-PROGRAM...]\n\
         libniji's program interpreter. Run as a program, it loads and runs EXECUTABLE-FILE.\n\
         \n\
         \x20 --list                list the program's dependencies and where they were found\n\
         \x20 --verify              exit 0 if the file is a dynamically linked program, 2 if it is\n\
         \x20                       a shared object or a static PIE, 1 otherwise\n\
         \x20 --inhibit-cache       do not read the loader cache\n\
         \x20 --library-path PATH   search PATH instead of LD_LIBRARY_PATH's directories\n\
         \x20 --preload LIST        preload the objects in LIST (after LD_PRELOAD's)\n\
         \x20 --argv0 STRING        run the program with STRING as argv[0]\n\
         \x20 --inhibit-rpath LIST  ignore RPATH and RUNPATH of the objects loaded by the paths in\n\
         \x20                       LIST (an empty first entry is the program)\n\
         \x20 --glibc-hwcaps-prepend LIST\n\
         \x20                       search the glibc-hwcaps subdirectories in LIST first\n\
         \x20 --glibc-hwcaps-mask LIST\n\
         \x20                       search only the supported subdirectories named in LIST\n\
         \x20 --audit LIST          accepted; this loader has no audit interface, so each object\n\
         \x20                       in LIST is reported and not loaded\n\
         \x20 --list-tunables       list libniji's tunables and their values, and exit\n\
         \x20 --list-diagnostics    list the loader's diagnostic information, and exit\n\
         \x20 --help                print this help and exit\n\
         \x20 --version             print version information and exit\n",
    ]
}

/// The message for an action that ends with [`USAGE`], in the order it is written (standard error);
/// `me` is the name `ld.so` was run by (its `argv[0]`).
pub fn refusal<'a>(action: Action<'a>, me: &'a [u8]) -> [&'a [u8]; 6] {
    const HINT: &[u8] = b" --help' for more information.\n";
    match action {
        Action::Unknown(arg) => [me, b": unrecognized option '", arg, b"'\nTry '", me, HINT],
        Action::Missing | Action::Version | Action::Help | Action::Tunables | Action::Diagnostics | Action::Program { .. } =>[me, b": missing program name\nTry '", me, HINT, b"", b""],
    }
}

