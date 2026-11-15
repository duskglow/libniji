//! The run: directories collected and scanned, soname links, the cache (module documentation of
//! [`super`] lists the probe-settled behaviour and the Liella rules).

use alloc::vec::Vec;

use super::conf::{Fatal, Reader};
use super::{extend, push, Fs, Kind, Oom, Options, Outcome, Stage, Text, CACHE, CONF};
use crate::arch::Arch;
use crate::cache::{self, compare, Entry};

/// The largest cache `-p` reads.
pub const CACHE_READ_MAX: usize = 1 << 28;
/// The most program headers read from one file.
pub const PHNUM_MAX: usize = 4096;
/// The largest dynamic segment read from one file.
pub const DYNAMIC_MAX: usize = 1 << 20;

const ENOTDIR: i32 = 20;

/// What one file is to ldconfig.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Info {
    /// Not an ELF file (no magic), or one ldconfig does not record (not a shared object, no
    /// dynamic segment, inconsistent headers): skipped silently.
    NotElf,
    /// The ELF magic, but the file ends before the headers it names.
    Truncated,
    /// An ELF file for another class, byte order or machine: skipped silently.
    Foreign,
    /// A shared object for this machine and its `DT_SONAME`, if any.
    Lib(Option<Vec<u8>>),
}

fn le<const N: usize>(b: &[u8], off: usize) -> Option<[u8; N]> {
    b.get(off..off.checked_add(N)?)?.try_into().ok()
}
fn u16le(b: &[u8], off: usize) -> Option<u16> {
    le::<2>(b, off).map(u16::from_le_bytes)
}
fn u32le(b: &[u8], off: usize) -> Option<u32> {
    le::<4>(b, off).map(u32::from_le_bytes)
}
fn u64le(b: &[u8], off: usize) -> Option<u64> {
    le::<8>(b, off).map(u64::from_le_bytes)
}

/// Reads exactly `len` bytes at `off` into a new buffer; `Ok(None)` when the file is shorter.
fn read_exact<F: Fs>(fs: &mut F, path: &[u8], off: u64, len: usize) -> Result<Option<Vec<u8>>, Oom> {
    let mut v = Vec::new();
    v.try_reserve_exact(len).map_err(|_| Oom)?;
    #[allow(clippy::disallowed_methods)] // capacity reserved on the line above, so this cannot allocate
    v.resize(len, 0);
    let mut got = 0usize;
    while got < len {
        let at = off.checked_add(u64::try_from(got).map_err(|_| Oom)?);
        let (Some(at), Some(buf)) = (at, v.get_mut(got..)) else { return Ok(None) };
        match fs.read_at(path, at, buf) {
            Ok(0) | Err(_) => return Ok(None),
            Ok(n) => got = got.saturating_add(n),
        }
    }
    Ok(Some(v))
}

/// Reads what ldconfig needs from the ELF headers of `path`: positioned reads of the file
/// header, the program headers, the dynamic segment and the soname string, each bounds-checked
/// (never mapped). 64-bit little-endian objects of `A`'s machine only.
pub fn elf_info<A: Arch, F: Fs>(fs: &mut F, path: &[u8]) -> Result<Info, Oom> {
    let mut hdr = [0u8; 64];
    let n = match fs.read_at(path, 0, &mut hdr) {
        Ok(n) => n,
        Err(_) => return Ok(Info::NotElf),
    };
    if n < 6 || hdr.get(..4) != Some(b"\x7fELF".as_slice()) {
        return Ok(Info::NotElf);
    }
    let (class, data) = (hdr.get(4).copied().unwrap_or(0), hdr.get(5).copied().unwrap_or(0));
    let need = match class {
        1 => 52,
        2 => 64,
        _ => return Ok(Info::NotElf),
    };
    if n < need {
        return Ok(Info::Truncated);
    }
    let machine = u16le(&hdr, 18).unwrap_or(0);
    if class != 2 || class != A::EI_CLASS || data != A::EI_DATA || machine != A::E_MACHINE {
        return Ok(Info::Foreign);
    }
    if u16le(&hdr, 16) != Some(3) {
        return Ok(Info::NotElf); // not ET_DYN
    }
    let (Some(phoff), Some(phentsize), Some(phnum)) = (u64le(&hdr, 32), u16le(&hdr, 54), u16le(&hdr, 56)) else { return Ok(Info::NotElf) };
    let phnum = usize::from(phnum);
    if phentsize != 56 || phnum > PHNUM_MAX {
        return Ok(Info::NotElf);
    }
    let Some(ph) = read_exact(fs, path, phoff, phnum.saturating_mul(56))? else { return Ok(Info::Truncated) };
    let mut loads: Vec<(u64, u64, u64)> = Vec::new(); // (vaddr, offset, filesz)
    let mut dynamic = None;
    for p in ph.chunks_exact(56) {
        let (Some(ty), Some(off), Some(vaddr), Some(filesz)) = (u32le(p, 0), u64le(p, 8), u64le(p, 16), u64le(p, 32)) else { continue };
        match ty {
            1 => push(&mut loads, (vaddr, off, filesz))?,
            2 => dynamic = Some((off, filesz)),
            _ => {}
        }
    }
    let Some((doff, dsize)) = dynamic else { return Ok(Info::NotElf) };
    let dsize = match usize::try_from(dsize) {
        Ok(s) if s <= DYNAMIC_MAX => s.saturating_sub(s % 16),
        _ => return Ok(Info::NotElf),
    };
    let Some(dynbytes) = read_exact(fs, path, doff, dsize)? else { return Ok(Info::Truncated) };
    let (mut soname, mut strtab, mut strsz) = (None, None, None);
    for d in dynbytes.chunks_exact(16) {
        let (Some(tag), Some(val)) = (u64le(d, 0), u64le(d, 8)) else { break };
        match tag {
            0 => break,
            14 => soname = Some(val),
            5 => strtab = Some(val),
            10 => strsz = Some(val),
            _ => {}
        }
    }
    let Some(so) = soname else { return Ok(Info::Lib(None)) };
    let (Some(strtab), Some(strsz)) = (strtab, strsz) else { return Ok(Info::NotElf) };
    if so >= strsz {
        return Ok(Info::NotElf);
    }
    // The string table's file offset through the PT_LOAD segment that holds it.
    let file_off = loads.iter().find_map(|&(va, off, sz)| {
        let rel = strtab.checked_sub(va)?;
        (rel < sz).then(|| off.checked_add(rel)?.checked_add(so)).flatten()
    });
    let Some(at) = file_off else { return Ok(Info::NotElf) };
    let room = usize::try_from(strsz.saturating_sub(so)).unwrap_or(usize::MAX).min(cache::NAME_MAX.saturating_add(1));
    let mut name = [0u8; 256];
    let got = match name.get_mut(..room) {
        Some(buf) => fs.read_at(path, at, buf).unwrap_or(0),
        None => 0,
    };
    let Some(nul) = name.get(..got).and_then(|b| b.iter().position(|&c| c == 0)) else {
        return Ok(if got < room { Info::Truncated } else { Info::NotElf });
    };
    let mut v = Vec::new();
    extend(&mut v, name.get(..nul).unwrap_or_default())?;
    Ok(Info::Lib(Some(v)))
}

/// A name the cache accepts as a key or file name.
fn name_ok(s: &[u8]) -> bool {
    !s.is_empty() && s.len() <= cache::NAME_MAX && s != b"." && s != b".." && !s.iter().any(|&c| c == 0 || c == b'/')
}

/// The names ldconfig reads (ldconfig(8)): `lib*.so*` and `ld-*.so*`.
fn wanted(name: &[u8]) -> bool {
    (name.starts_with(b"lib") || name.starts_with(b"ld-")) && name.windows(3).any(|w| w == b".so")
}

/// `dir` as the cache needs it (absolute, no empty, `.` or `..` component, no trailing `/`),
/// or `None` when it is not that way (its libraries are then linked but not recorded).
fn canonical(dir: &[u8]) -> Result<Option<Vec<u8>>, Oom> {
    if dir.first() != Some(&b'/') {
        return Ok(None);
    }
    let mut v = Vec::new();
    for c in dir.split(|&c| c == b'/').filter(|c| !c.is_empty()) {
        if c == b"." || c == b".." {
            return Ok(None);
        }
        push(&mut v, b'/')?;
        extend(&mut v, c)?;
    }
    if v.is_empty() {
        push(&mut v, b'/')?;
    }
    Ok(Some(v))
}

fn join(dir: &[u8], name: &[u8]) -> Result<Vec<u8>, Oom> {
    let mut p = Vec::new();
    extend(&mut p, dir)?;
    if p.last() != Some(&b'/') {
        push(&mut p, b'/')?;
    }
    extend(&mut p, name)?;
    Ok(p)
}

/// One cache entry, owned until the cache is written.
#[derive(Debug)]
struct Rec {
    key: Vec<u8>,
    path: Vec<u8>,
    hwcaps: Option<Vec<u8>>,
    order: u32,
}

/// A directory to scan and where it came from.
struct Src<'c> {
    path: &'c [u8],
    from: &'c [u8],
    line: u32,
}

struct Run<'r, A: Arch, F: Fs> {
    fs: &'r mut F,
    me: &'r [u8],
    o: &'r Options<'r>,
    recs: Vec<Rec>,
    _arch: core::marker::PhantomData<A>,
}

fn errno_line<F: Fs>(fs: &F, err: &mut Text, e: i32) {
    err.bytes(b": ");
    fs.error_text(e, err);
    err.bytes(b"\n");
}

impl<A: Arch, F: Fs> Run<'_, A, F> {
    /// Scans one directory (or one `glibc-hwcaps/<name>` subdirectory of it).
    fn dir(&mut self, dir: &[u8], hwcaps: Option<&[u8]>, order: u32, r: &mut Outcome) -> Result<(), Oom> {
        let mut names = Vec::new();
        if let Err(e) = self.fs.list(dir, &mut names) {
            r.err.bytes(self.me).bytes(b": Can't open directory ").bytes(dir);
            errno_line(self.fs, &mut r.err, e);
            return Ok(());
        }
        names.sort_unstable();
        names.dedup();
        let canon = canonical(dir)?;
        // (soname, file name) of each candidate; the soname links that already exist.
        let mut cands: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
        for name in names.iter().filter(|n| wanted(n) && name_ok(n)) {
            let path = join(dir, name)?;
            let is_link = match self.fs.lstat(&path) {
                Ok(m) if m.kind == Kind::Link => true,
                Ok(m) if m.kind == Kind::File => false,
                _ => continue,
            };
            if is_link {
                match self.fs.stat(&path) {
                    Ok(m) if m.kind == Kind::File => {}
                    Ok(_) => continue,
                    Err(e) => {
                        r.err.bytes(self.me).bytes(b": Cannot stat ").bytes(&path);
                        errno_line(self.fs, &mut r.err, e);
                        continue;
                    }
                }
            }
            let soname = match elf_info::<A, F>(self.fs, &path)? {
                Info::Lib(Some(s)) => s,
                Info::Lib(None) => {
                    let mut v = Vec::new();
                    extend(&mut v, name)?;
                    v
                }
                Info::Truncated => {
                    r.err.bytes(self.me).bytes(b": file ").bytes(&path).bytes(b" is truncated\n\n");
                    continue;
                }
                Info::NotElf | Info::Foreign => continue,
            };
            if !name_ok(&soname) {
                continue;
            }
            if soname == A::INTERP.as_bytes() {
                r.err.bytes(self.me).bytes(b": ").bytes(&path).bytes(b" is the dynamic linker, ignoring\n\n");
            }
            if is_link && name.as_slice() == soname.as_slice() {
                continue; // the soname link itself
            }
            if is_link && name.ends_with(b".so") && soname.starts_with(name) {
                // A development link: its own entry, no link, no -v line (probe).
                self.record(canon.as_deref(), name, hwcaps, order)?;
                continue;
            }
            let mut n = Vec::new();
            extend(&mut n, name)?;
            push(&mut cands, (soname, n))?;
        }
        // Group by soname (bytes), highest file name first under the D.3.3 comparison, then bytes.
        cands.sort_unstable_by(|a, b| a.0.cmp(&b.0).then_with(|| compare(&b.1, &a.1)).then_with(|| b.1.cmp(&a.1)));
        let mut i = 0usize;
        while let Some((soname, winner)) = cands.get(i) {
            let next = cands.iter().skip(i).position(|c| c.0 != *soname).map_or(cands.len(), |k| i.saturating_add(k));
            let changed = if soname != winner && !self.o.no_links { self.link(dir, soname, winner, r)? } else { false };
            if self.o.verbose {
                r.out.bytes(b"\t").bytes(soname).bytes(b" -> ").bytes(winner);
                if changed {
                    r.out.bytes(b" (changed)");
                }
                r.out.bytes(b"\n");
            }
            self.record(canon.as_deref(), soname, hwcaps, order)?;
            i = next;
        }
        Ok(())
    }

    /// Points `<dir>/<soname>` at `winner` unless it already does; true when it changed.
    fn link(&mut self, dir: &[u8], soname: &[u8], winner: &[u8], r: &mut Outcome) -> Result<bool, Oom> {
        let path = join(dir, soname)?;
        match self.fs.lstat(&path) {
            Ok(m) if m.kind == Kind::Link => {
                let mut cur = Vec::new();
                if self.fs.read_link(&path, &mut cur).is_ok() && cur == winner {
                    return Ok(false);
                }
            }
            Ok(_) => {
                r.err.bytes(self.me).bytes(b": ").bytes(&path).bytes(b" is not a symbolic link, not replaced\n");
                return Ok(false);
            }
            Err(_) => {}
        }
        match self.fs.replace_link(winner, &path) {
            Ok(()) => Ok(true),
            Err(e) => {
                r.err.bytes(self.me).bytes(b": Cannot link ").bytes(&path).bytes(b" to ").bytes(winner);
                errno_line(self.fs, &mut r.err, e);
                Ok(false)
            }
        }
    }

    fn record(&mut self, canon: Option<&[u8]>, key: &[u8], hwcaps: Option<&[u8]>, order: u32) -> Result<(), Oom> {
        let (Some(dir), false) = (canon, self.o.no_cache) else { return Ok(()) };
        let mut path = Vec::new();
        extend(&mut path, dir)?;
        if let Some(h) = hwcaps {
            extend(&mut path, b"/glibc-hwcaps/")?;
            extend(&mut path, h)?;
        }
        if path.as_slice() != b"/" {
            push(&mut path, b'/')?;
        }
        extend(&mut path, key)?;
        let mut k = Vec::new();
        extend(&mut k, key)?;
        let h = match hwcaps {
            Some(h) => {
                let mut v = Vec::new();
                extend(&mut v, h)?;
                Some(v)
            }
            None => None,
        };
        push(&mut self.recs, Rec { key: k, path, hwcaps: h, order })
    }
}

/// The whole run after option parsing; `None` = out of memory.
pub(super) fn main<A: Arch, F: Fs>(me: &[u8], o: &Options<'_>, fs: &mut F, generator: &[u8], r: &mut Outcome) -> Option<u8> {
    go::<A, F>(me, o, fs, generator, r).ok()
}

fn go<A: Arch, F: Fs>(me: &[u8], o: &Options<'_>, fs: &mut F, generator: &[u8], r: &mut Outcome) -> Result<u8, Oom> {
    if let Some(f) = o.format {
        if f == b"old" || f == b"compat" {
            r.err.bytes(me).bytes(b": cache format `").bytes(f).bytes(b"' is not supported: Liella writes only the new format (-c new)\n");
            return Ok(1);
        }
    }
    if o.library_mode {
        r.err.bytes(me).bytes(b": -l (linking individual libraries) is not supported yet\n");
        return Ok(1);
    }
    let cache_path = o.cache.unwrap_or(CACHE);
    let mut shown_cache = Vec::new();
    extend(&mut shown_cache, fs.root())?;
    extend(&mut shown_cache, cache_path)?;
    if o.print {
        let mut bytes = Vec::new();
        if let Err(e) = fs.read_file(cache_path, CACHE_READ_MAX, &mut bytes) {
            r.err.bytes(me).bytes(b": Can't open cache file ").bytes(&shown_cache).bytes(b"\n");
            errno_line(fs, &mut r.err, e);
            return Ok(1);
        }
        if !super::print_cache(&bytes, &shown_cache, &mut r.out) {
            r.out.buf.clear();
            r.err.bytes(me).bytes(b": ").bytes(&shown_cache).bytes(b" is not a cache in the new format\n");
            return Ok(1);
        }
        return Ok(0);
    }
    // 1. The directories, in order: operands, configuration, trusted.
    let mut dirs: Vec<conf::Dir> = Vec::new();
    for &op in &o.operands {
        let mut d = conf::Dir { path: Vec::new(), from: Vec::new(), line: 0 };
        extend(&mut d.path, op)?;
        extend(&mut d.from, b"<cmdline>")?;
        push(&mut dirs, d)?;
    }
    if !o.only_operands {
        let conf_path = o.conf.unwrap_or(CONF);
        let mut reader = Reader::new(fs, o.root.is_some());
        match reader.read(conf_path, conf_path, me, &mut r.err) {
            Ok(()) => {}
            Err(Fatal::Oom) => return Err(Oom),
            Err(Fatal::RelativeUnderRoot) => {
                r.err.bytes(me).bytes(b": need absolute file name for configuration file when using -r\n");
                return Ok(1);
            }
        }
        let found = core::mem::take(&mut reader.dirs);
        for d in found {
            push(&mut dirs, d)?;
        }
        for &t in A::SYSTEM_DIRS {
            let mut d = conf::Dir { path: Vec::new(), from: Vec::new(), line: 0 };
            extend(&mut d.path, t.as_bytes())?;
            extend(&mut d.from, b"<builtin>")?;
            push(&mut dirs, d)?;
        }
    }
    // 2. Each directory once, by identity.
    let mut keep: Vec<(Src<'_>, (u64, u64))> = Vec::new();
    for d in &dirs {
        let m = match fs.stat(&d.path) {
            Ok(m) if m.kind == Kind::Dir => m,
            other => {
                r.err.bytes(me).bytes(b": Can't stat ").bytes(&d.path);
                errno_line(fs, &mut r.err, other.err().unwrap_or(ENOTDIR));
                continue;
            }
        };
        let id = (m.dev, m.ino);
        if let Some((first, _)) = keep.iter().find(|k| k.1 == id) {
            r.err.bytes(me).bytes(b": Path `").bytes(&d.path).bytes(b"' given more than once\n(from ").bytes(&d.from).bytes(b":").num(u64::from(d.line));
            r.err.bytes(b" and ").bytes(first.from).bytes(b":").num(u64::from(first.line)).bytes(b")\n");
            continue;
        }
        push(&mut keep, (Src { path: &d.path, from: &d.from, line: d.line }, id))?;
    }
    // 3. Scan.
    let mut run = Run::<A, F> { fs, me, o, recs: Vec::new(), _arch: core::marker::PhantomData };
    for (idx, (s, _)) in keep.iter().enumerate() {
        if o.verbose {
            r.out.bytes(s.path).bytes(b": (from ").bytes(s.from).bytes(b":").num(u64::from(s.line)).bytes(b")\n");
        }
        if o.no_cache && o.no_links {
            continue;
        }
        let order = u32::try_from(idx).unwrap_or(u32::MAX);
        let hw = join(s.path, b"glibc-hwcaps")?;
        if matches!(run.fs.stat(&hw), Ok(m) if m.kind == Kind::Dir) {
            let mut subs = Vec::new();
            if run.fs.list(&hw, &mut subs).is_ok() {
                subs.sort_unstable();
                for sub in subs.iter().filter(|n| name_ok(n) && n.first() != Some(&b'.')) {
                    let p = join(&hw, sub)?;
                    if matches!(run.fs.stat(&p), Ok(m) if m.kind == Kind::Dir) {
                        run.dir(&p, Some(sub), order, r)?;
                    }
                }
            }
        }
        run.dir(s.path, None, order, r)?;
    }
    if o.no_cache || o.only_operands {
        return Ok(0);
    }
    // 4. The cache.
    let mut entries: Vec<Entry<'_>> = Vec::new();
    for rec in &run.recs {
        push(&mut entries, Entry { flags: A::CACHE_FLAGS, key: &rec.key, path: &rec.path, hwcaps: rec.hwcaps.as_deref(), order: rec.order })?;
    }
    let bytes = match cache::encoded_len(&entries, generator).and_then(|len| {
        let mut out = Vec::new();
        out.try_reserve_exact(len).map_err(|_| cache::WriteError::Buffer(len))?;
        #[allow(clippy::disallowed_methods)] // capacity reserved on the line above, so this cannot allocate
        out.resize(len, 0);
        cache::write(&mut entries, generator, &mut out).map(|n| {
            out.truncate(n);
            out
        })
    }) {
        Ok(b) => b,
        Err(e) => {
            r.err.bytes(me).bytes(b": the cache was not written: ");
            let why: &[u8] = match e {
                cache::WriteError::TooManyEntries => b"too many libraries",
                cache::WriteError::TooManyHwcaps => b"too many glibc-hwcaps subdirectories",
                cache::WriteError::TooLarge | cache::WriteError::Buffer(_) => b"too large",
                _ => b"an entry the format cannot hold",
            };
            r.err.bytes(why).bytes(b"\n");
            return Ok(1);
        }
    };
    match run.fs.replace_file(cache_path, &bytes) {
        Ok(()) => Ok(0),
        Err((stage, e)) => {
            match stage {
                Stage::Create => r.err.bytes(me).bytes(b": Can't create temporary cache file ").bytes(&shown_cache).bytes(b"~"),
                Stage::Write => r.err.bytes(me).bytes(b": Writing of cache data to ").bytes(&shown_cache).bytes(b"~ failed"),
                Stage::Rename => r.err.bytes(me).bytes(b": Renaming of ").bytes(&shown_cache).bytes(b"~ to ").bytes(&shown_cache).bytes(b" failed"),
            };
            errno_line(run.fs, &mut r.err, e);
            Ok(1)
        }
    }
}

use super::conf;
