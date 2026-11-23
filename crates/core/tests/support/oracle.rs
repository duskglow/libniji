//! Test programs compiled into the shared `target/test-oracles` directory (the C oracles, the
//! programs linked against the products). Every test process of every test binary may want the
//! same program at the same moment (nextest runs each test in its own process), so a build must
//! never expose a half-written file under the final name: one process saw the compiler's output
//! before it was executable (EACCES) or while it was still open for writing (ETXTBSY).
//!
//! [`build`] therefore compiles to a name of its own in the same directory and renames it onto
//! the final name (rename(2) replaces the name atomically; a process already running the old file
//! keeps it). A per-name lock file (flock) makes concurrent processes wait for one build instead of
//! all compiling; correctness does not depend on it. The build is skipped when the program exists and
//! `<name>.stamp` records the same command line and the same content hash of every input on it (the
//! files it names, plus the headers beside a `.c` source and under each include directory). Content,
//! not modification time, so file-system timestamp resolution does not matter.
//! Included with `#[path]` by the test files that compile programs.
#![allow(dead_code)]

use std::ffi::OsString;
use std::fs::File;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use niji_boundary::sys::CSys;
use niji_kernel::fd::Fd;
use niji_kernel::sys::Sys;
use niji_kernel::uapi::LOCK_EX;

/// Containment for harnesses that start processes in namespaces (`contain.rs`).
#[path = "contain.rs"]
pub(crate) mod contain;

static SEQ: AtomicU32 = AtomicU32::new(0);

/// The repository root: every crate that includes this file sits at `crates/<name>`.
pub(crate) fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A differential test program: one C source compiled against the host C library (the
/// reference) and linked statically against libniji's products in `target/niji`, with the
/// test's own flags and libraries. The one place the products' link line is written, so a
/// change to it (the start file, a new library) is one edit. Command lines:
///
/// ```text
/// reference: cc -std=gnu17 -O1 <flags> <host_flags> <src> <host_libs>
/// libniji:   cc -std=gnu17 -O1 <flags> -static -nostdlib -fno-builtin <niji_flags>
///               target/niji/<crt> <src> target/niji/<archives>... target/niji/libc.a
///               [-L target/niji <niji_libs>] -lgcc
/// ```
#[derive(Clone, Copy)]
pub(crate) struct Pair<'a> {
    src: &'a str,
    flags: &'a [&'a str],
    host_flags: &'a [&'a str],
    host_libs: &'a [&'a str],
    niji_flags: &'a [&'a str],
    archives: &'a [&'a str],
    niji_libs: &'a [&'a str],
    crt: &'a str,
    quiet: bool,
}

/// The pair for `src` (a path under the repository root) with no extra flags or libraries.
pub(crate) const fn build_pair(src: &str) -> Pair<'_> {
    Pair { src, flags: &[], host_flags: &[], host_libs: &[], niji_flags: &[], archives: &[], niji_libs: &[], crt: "niji-crt1.o", quiet: false }
}

impl<'a> Pair<'a> {
    /// Flags for both sides, right after `-O1` (`-w`, `-pg`).
    pub(crate) const fn flags(self, flags: &'a [&'a str]) -> Self {
        Pair { flags, ..self }
    }
    /// Flags for the reference side only, before the source (`-fno-builtin`, `-static`).
    pub(crate) const fn host_flags(self, host_flags: &'a [&'a str]) -> Self {
        Pair { host_flags, ..self }
    }
    /// Libraries for the reference side, after the source (`-lm`, `-lresolv`).
    pub(crate) const fn host_libs(self, host_libs: &'a [&'a str]) -> Self {
        Pair { host_libs, ..self }
    }
    /// Flags for the libniji side only, after `-fno-builtin` (`-DNIJI_PRODUCTS`).
    pub(crate) const fn niji_flags(self, niji_flags: &'a [&'a str]) -> Self {
        Pair { niji_flags, ..self }
    }
    /// Product archives linked before `libc.a` (`libm.a`).
    pub(crate) const fn archives(self, archives: &'a [&'a str]) -> Self {
        Pair { archives, ..self }
    }
    /// Libraries searched in `target/niji` after `libc.a` (`-lresolv`).
    pub(crate) const fn niji_libs(self, niji_libs: &'a [&'a str]) -> Self {
        Pair { niji_libs, ..self }
    }
    /// The products' start file (`niji-gcrt1.o` for a profiled program).
    pub(crate) const fn crt(self, crt: &'a str) -> Self {
        Pair { crt, ..self }
    }
    /// The math tests' pair: `libm.a` before `libc.a` on the libniji side, `-lm` and no compiler
    /// builtins on the reference side (so both sides call the library).
    pub(crate) const fn libm(self) -> Self {
        Pair { host_flags: &["-fno-builtin"], host_libs: &["-lm"], archives: &["libm.a"], ..self }
    }
    /// Capture the compiler's messages ([`build_quiet`]): for programs that may fail to build by design.
    pub(crate) const fn quiet(self) -> Self {
        Pair { quiet: true, ..self }
    }

    /// The compiler command line for one side, without `-o`.
    pub(crate) fn command(&self, niji: bool) -> Command {
        let r = root();
        let src = r.join(self.src);
        let mut cmd = Command::new("cc");
        cmd.args(["-std=gnu17", "-O1"]).args(self.flags);
        if niji {
            let p = r.join("target/niji");
            cmd.args(["-static", "-nostdlib", "-fno-builtin"]).args(self.niji_flags).arg(p.join(self.crt)).arg(&src);
            cmd.args(self.archives.iter().map(|a| p.join(a))).arg(p.join("libc.a"));
            if !self.niji_libs.is_empty() {
                cmd.arg("-L").arg(&p).args(self.niji_libs);
            }
            cmd.arg("-lgcc");
        } else {
            cmd.args(self.host_flags).arg(&src).args(self.host_libs);
        }
        cmd
    }

    /// Build one side into `out` ([`build`] or [`build_quiet`]): the libniji side if `niji`.
    pub(crate) fn build(&self, out: &Path, niji: bool) -> bool {
        build_with(self.command(niji), out, self.quiet)
    }
}

/// Each line of `text` with the last header line before it (a line starting with one of
/// `markers`, itself included, `\n`-terminated; empty before the first header).
pub(crate) fn with_headers(text: &str, markers: &[&str]) -> Vec<(String, String)> {
    let mut header = String::new();
    text.lines()
        .map(|l| {
            if markers.iter().any(|m| l.starts_with(m)) {
                header = format!("{l}\n");
            }
            (header.clone(), l.to_string())
        })
        .collect()
}

/// The known-differences filter: `lines` without the lines recorded in `list` as printed by
/// `who` only. An entry is `(case, line, why)`: it takes the first line not yet taken whose
/// header starts with `case` and whose text starts with `line`, so each entry takes exactly one
/// line; an entry that takes none is no longer printed and fails the test.
pub(crate) fn without(lines: Vec<(String, String)>, list: &[(&str, &str, &str)], who: &str) -> Vec<(String, String)> {
    let mut seen = vec![false; list.len()];
    let mut out = Vec::new();
    for (h, l) in lines {
        match list.iter().zip(seen.iter_mut()).find(|((case, needle, _), taken)| !**taken && h.starts_with(case) && l.starts_with(needle)) {
            Some(((_, _, why), taken)) => {
                *taken = true;
                eprintln!("recorded line only {who} prints ({why}):\n  {l}");
            }
            None => out.push((h, l)),
        }
    }
    for (entry, taken) in list.iter().zip(&seen) {
        assert!(*taken, "recorded line no longer printed by {who}: {entry:?}");
    }
    out
}

/// Compile `cmd` (a compiler command line without `-o`) into `out`; the compiler's messages go
/// to the test's stderr. True if `out` is there and current.
pub(crate) fn build(cmd: Command, out: &Path) -> bool {
    build_with(cmd, out, false)
}

/// [`build`] with the compiler's output captured (for builds that may fail by design, for
/// example on a host without a header).
pub(crate) fn build_quiet(cmd: Command, out: &Path) -> bool {
    build_with(cmd, out, true)
}

/// Compile locale `lang`.`charset` into the directory `dest` (in the shared `target/test-locales`)
/// with the host's `localedef`, the same way: built in a private sibling directory and renamed into
/// place, so no test sees (or writes into) a half-built locale. True when `dest/<probe>` exists
/// after. localedef reports warnings with status 1 and still writes the locale.
pub(crate) fn locale(dest: &Path, lang: &str, charset: &str, probe: &str) -> bool {
    if dest.join(probe).exists() {
        return true;
    }
    let Some(parent) = dest.parent() else { return false };
    if std::fs::create_dir_all(parent).is_err() {
        return false;
    }
    let lock = File::options().create(true).truncate(false).write(true).open(sibling(dest, ".lock"));
    if let Ok(f) = &lock {
        let _ = CSys.flock(Fd::new(f.as_raw_fd()), LOCK_EX);
    }
    if dest.join(probe).exists() {
        return true;
    }
    let tmp = sibling(dest, &format!(".tmp-{}-{}", std::process::id(), SEQ.fetch_add(1, Ordering::Relaxed)));
    let _ = std::fs::remove_dir_all(&tmp);
    // Absent (cannot be started) is a skip for the caller; present but failing is a failure (TEST-SILENT-SKIPS).
    let made = match Command::new("localedef").args(["-i", lang, "-f", charset]).arg(&tmp).output() {
        Err(_) => false,
        Ok(o) => {
            assert!(o.status.code().is_some_and(|c| c <= 1) && tmp.join(probe).exists(), "localedef is present but failed for {lang}.{charset} ({}): {}", o.status, String::from_utf8_lossy(&o.stderr));
            true
        }
    };
    if made && tmp.join(probe).exists() {
        // Losing a race with a builder that holds no lock is fine: its locale is complete too.
        let _ = std::fs::rename(&tmp, dest);
    }
    let _ = std::fs::remove_dir_all(&tmp);
    dest.join(probe).exists()
}

fn sibling(out: &Path, suffix: &str) -> PathBuf {
    let mut name = out.file_name().map(OsString::from).unwrap_or_default();
    name.push(suffix);
    out.with_file_name(name)
}

/// The command line as recorded in the stamp.
fn key(cmd: &Command) -> Vec<u8> {
    let mut k = cmd.get_program().as_bytes().to_vec();
    for a in cmd.get_args() {
        k.push(0);
        k.extend_from_slice(a.as_bytes());
    }
    k
}

/// FNV-1a over `bytes`, continuing from `h`. Fixed and portable, so a stamp stays valid across
/// toolchains (a changed algorithm could only force one rebuild).
fn fnv(mut h: u64, bytes: &[u8]) -> u64 {
    for b in bytes {
        h = (h ^ u64::from(*b)).wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// The files the command line depends on: the files it names, the `.h` files beside each `.c`
/// source, and every header under each include directory (`-I dir`).
fn input_files(cmd: &Command) -> Option<Vec<PathBuf>> {
    let mut files = Vec::new();
    for a in cmd.get_args() {
        let p = Path::new(a);
        if p.is_dir() {
            headers_under(p, &mut files)?;
        } else if p.is_file() {
            files.push(p.to_path_buf());
            if p.extension().is_some_and(|e| e == "c") {
                headers_under_flat(p.parent()?, &mut files);
            }
        }
    }
    // Directory order differs between file systems; the hash must not.
    files.sort();
    files.dedup();
    Some(files)
}

fn headers_under_flat(dir: &Path, files: &mut Vec<PathBuf>) {
    for p in std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()) {
        if p.extension().is_some_and(|x| x == "h") {
            files.push(p);
        }
    }
    files.sort();
}

fn headers_under(dir: &Path, files: &mut Vec<PathBuf>) -> Option<()> {
    for p in std::fs::read_dir(dir).ok()?.flatten().map(|e| e.path()) {
        if p.is_dir() {
            headers_under(&p, files)?;
        } else if p.extension().is_some_and(|x| x == "h") {
            files.push(p);
        }
    }
    files.sort();
    Some(())
}

/// A hash of the names and contents of the input files. Content, not modification time: a
/// file system with 1-2 s (or no) timestamps cannot make an edited input look old.
fn inputs_hash(cmd: &Command) -> Option<u64> {
    let mut h = 0xcbf2_9ce4_8422_2325_u64;
    for f in input_files(cmd)? {
        h = fnv(h, f.as_os_str().as_bytes());
        h = fnv(h, &[0]);
        h = fnv(h, &std::fs::read(&f).ok()?);
        h = fnv(h, &[0]);
    }
    Some(h)
}

/// The stamp: the command line, then the hash of the inputs it was built from.
fn stamp_text(key: &[u8], inputs: u64) -> Vec<u8> {
    let mut t = key.to_vec();
    t.extend_from_slice(format!("\n{inputs:016x}").as_bytes());
    t
}

fn build_with(mut cmd: Command, out: &Path, quiet: bool) -> bool {
    let Some(dir) = out.parent() else { return false };
    if std::fs::create_dir_all(dir).is_err() {
        return false;
    }
    // A static link against the products drops the library sections the program never reaches
    // (the archive's functions and data are in sections of their own): ~5x smaller programs,
    // so far less written per test run (CONF-DIET). Part of the command line, so of the stamp.
    let args: Vec<&std::ffi::OsStr> = cmd.get_args().collect();
    if args.iter().any(|a| *a == "-static") && args.iter().any(|a| *a == "-nostdlib") && !args.iter().any(|a| *a == "-c" || *a == "-r") {
        cmd.arg("-Wl,--gc-sections");
    }
    let stamp = sibling(out, ".stamp");
    let key = key(&cmd);
    // Held until the end of the function; without it (no flock on this file system) concurrent
    // processes each build and rename, which is still correct.
    let lock = File::options().create(true).truncate(false).write(true).open(sibling(out, ".lock"));
    if let Ok(f) = &lock {
        let _ = CSys.flock(Fd::new(f.as_raw_fd()), LOCK_EX);
    }
    // Taken before the build: an input edited during it makes the stamp stale, never wrong.
    let want = inputs_hash(&cmd).map(|h| stamp_text(&key, h));
    if let Some(want) = &want
        && out.is_file()
        && std::fs::read(&stamp).is_ok_and(|s| s == *want)
    {
        return true;
    }
    let tag = format!(".tmp-{}-{}", std::process::id(), SEQ.fetch_add(1, Ordering::Relaxed));
    let tmp = sibling(out, &tag);
    cmd.arg("-o").arg(&tmp);
    let ok = if quiet {
        cmd.output().is_ok_and(|o| o.status.success())
    } else {
        // Not startable = the tool is absent (a skip for the caller); started and failed = a failure (TEST-SILENT-SKIPS).
        match cmd.status() {
            Err(_) => false,
            Ok(s) => {
                assert!(s.success(), "{:?} is present but failed ({s})", cmd.get_program());
                true
            }
        }
    };
    if !ok || std::fs::rename(&tmp, out).is_err() {
        let _ = std::fs::remove_file(&tmp);
        return false;
    }
    // After the program: a crash in between leaves the old stamp, which can only force a rebuild.
    let stamp_tmp = sibling(out, &format!("{tag}.stamp"));
    let wrote = want.as_ref().is_some_and(|w| std::fs::write(&stamp_tmp, w).is_ok() && std::fs::rename(&stamp_tmp, &stamp).is_ok());
    if !wrote {
        let _ = std::fs::remove_file(&stamp_tmp);
        let _ = std::fs::remove_file(&stamp);
    }
    true
}
