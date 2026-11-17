//! The run: directories collected and scanned, soname links, the cache (module documentation of
//! [`super`] lists the probe-settled behaviour and the libniji rules).

use alloc::vec::Vec;

use super::conf::{Fatal, Reader};
use super::{extend, push, Fs, Kind, Oom, Options, Outcome, Stage, Text, CACHE, CONF};
use crate::arch::{Arch, ELFCLASS64, ELFDATA2LSB};
use crate::cache::{self, compare, Entry};

/// The largest cache `-p` reads.
pub const CACHE_READ_MAX: usize = 1 << 28;
/// The most program headers read from one file.
pub const PHNUM_MAX: usize = 4096;
/// The largest dynamic segment read from one file.
pub const DYNAMIC_MAX: usize = 1 << 20;


/// What one file is to ldconfig.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Info {
    /// Not an ELF file (no magic), or one ldconfig does not record (not a shared object, no
    /// dynamic segment, inconsistent headers): skipped silently.
    NotElf,
    /// The ELF magic, but the file ends before the headers it names.
    Truncated,
    /// An ELF file of a class, byte order and machine not in `Arch::LDCONFIG_TYPES`: skipped silently.
    Foreign,
    /// A shared object of one of `Arch::LDCONFIG_TYPES`: its `DT_SONAME`, if any, and that type's cache flags.
    Lib(Option<Vec<u8>>, u32),
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
/// (never mapped). Little-endian objects of the types in `A::LDCONFIG_TYPES` only, ELF-64 or ELF-32 layout
/// by `EI_CLASS` (gABI: 64- and 52-byte file headers, 56- and 32-byte program headers, 16- and 8-byte
/// dynamic entries).
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
    let Some(&(_, _, _, flags)) = A::LDCONFIG_TYPES.iter().find(|t| t.0 == class && t.1 == data && t.2 == machine) else { return Ok(Info::Foreign) };
    if data != ELFDATA2LSB {
        return Ok(Info::Foreign); // this reader decodes little-endian fields only
    }
    if u16le(&hdr, 16) != Some(3) {
        return Ok(Info::NotElf); // not ET_DYN
    }
    let wide = class == ELFCLASS64;
    // A field of the class's natural width (ELF-64: 8 bytes; ELF-32: 4) at `off64` or `off32`.
    let word = |b: &[u8], off64: usize, off32: usize| if wide { u64le(b, off64) } else { u32le(b, off32).map(u64::from) };
    let (phent, dynent) = if wide { (56usize, 16usize) } else { (32, 8) };
    let (Some(phoff), Some(phentsize), Some(phnum)) = (word(&hdr, 32, 28), u16le(&hdr, if wide { 54 } else { 42 }), u16le(&hdr, if wide { 56 } else { 44 })) else { return Ok(Info::NotElf) };
    let phnum = usize::from(phnum);
    if usize::from(phentsize) != phent || phnum > PHNUM_MAX {
        return Ok(Info::NotElf);
    }
    let Some(ph) = read_exact(fs, path, phoff, phnum.saturating_mul(phent))? else { return Ok(Info::Truncated) };
    let mut loads: Vec<(u64, u64, u64)> = Vec::new(); // (vaddr, offset, filesz)
    let mut dynamic = None;
    for p in ph.chunks_exact(phent) {
        let (Some(ty), Some(off), Some(vaddr), Some(filesz)) = (u32le(p, 0), word(p, 8, 4), word(p, 16, 8), word(p, 32, 16)) else { continue };
        match ty {
            1 => push(&mut loads, (vaddr, off, filesz))?,
            2 => dynamic = Some((off, filesz)),
            _ => {}
        }
    }
    let Some((doff, dsize)) = dynamic else { return Ok(Info::NotElf) };
    let dsize = match usize::try_from(dsize) {
        Ok(s) if s <= DYNAMIC_MAX => s.saturating_sub(s.checked_rem(dynent).unwrap_or(0)),
        _ => return Ok(Info::NotElf),
    };
    let Some(dynbytes) = read_exact(fs, path, doff, dsize)? else { return Ok(Info::Truncated) };
    let (mut soname, mut strtab, mut strsz) = (None, None, None);
    for d in dynbytes.chunks_exact(dynent) {
        // ELF-32's `d_tag` is signed: a negative tag widens to no tag matched below.
        let (Some(tag), Some(val)) = (word(d, 0, 0), word(d, 8, 4)) else { break };
        match tag {
            0 => break,
            14 => soname = Some(val),
            5 => strtab = Some(val),
            10 => strsz = Some(val),
            _ => {}
        }
    }
    let Some(so) = soname else { return Ok(Info::Lib(None, flags)) };
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
    Ok(Info::Lib(Some(v), flags))
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
    /// The cache flags of the file's ELF type (`Arch::LDCONFIG_TYPES`).
    flags: u32,
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
        let mut cands: Vec<(Vec<u8>, Vec<u8>, u32)> = Vec::new();
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
                        // A link that cannot be followed (probe): reported with -v; unless links are left alone
                        // (-X) or it is a development link (`.so` at the end), it is removed, as a stale soname
                        // link; not in a glibc-hwcaps subdirectory, where ldconfig makes no links.
                        if self.o.verbose {
                            r.err.bytes(self.me).bytes(b": Cannot stat ").bytes(&path);
                            errno_line(self.fs, &mut r.err, e);
                        }
                        if !self.o.no_links && hwcaps.is_none() && !name.ends_with(b".so") {
                            let _ = self.fs.remove(&path);
                        }
                        continue;
                    }
                }
            }
            let (soname, flags) = match elf_info::<A, F>(self.fs, &path)? {
                Info::Lib(Some(s), f) => (s, f),
                Info::Lib(None, f) => {
                    let mut v = Vec::new();
                    extend(&mut v, name)?;
                    (v, f)
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
                self.record(canon.as_deref(), name, name, hwcaps, order, flags)?;
                continue;
            }
            let mut n = Vec::new();
            extend(&mut n, name)?;
            push(&mut cands, (soname, n, flags))?;
        }
        // Group by soname (bytes), highest file name first under the D.3.3 comparison, then bytes.
        cands.sort_unstable_by(|a, b| a.0.cmp(&b.0).then_with(|| compare(&b.1, &a.1)).then_with(|| b.1.cmp(&a.1)));
        let mut i = 0usize;
        while let Some((soname, winner, flags)) = cands.get(i) {
            let next = cands.iter().skip(i).position(|c| c.0 != *soname).map_or(cands.len(), |k| i.saturating_add(k));
            // Files of one soname but different ELF types (a native and a foreign-class object): the highest
            // version still wins and the entry carries its type; each other-type file is reported, whatever order
            // the directory lists them in (pending record UTIL-LDCONFIG-5-1).
            for (_, other, f) in cands.get(i.saturating_add(1)..next).unwrap_or_default() {
                if f != flags {
                    r.err.bytes(self.me).bytes(b": libraries ").bytes(other).bytes(b" and ").bytes(winner);
                    r.err.bytes(b" in directory ").bytes(dir).bytes(b" have same soname but different type.\n");
                }
            }
            let changed = if soname != winner && !self.o.no_links && hwcaps.is_none() { self.link(dir, soname, winner, r)? } else { false };
            if self.o.verbose {
                r.out.bytes(b"\t").bytes(soname).bytes(b" -> ").bytes(winner);
                if changed {
                    r.out.bytes(b" (changed)");
                }
                r.out.bytes(b"\n");
            }
            // A glibc-hwcaps subdirectory gets no links, so its entry names the file itself (probe).
            let file = if hwcaps.is_some() { winner } else { soname };
            self.record(canon.as_deref(), soname, file, hwcaps, order, *flags)?;
            i = next;
        }
        Ok(())
    }

    /// Points `<dir>/<soname>` at `winner` unless it already reaches the same file; true when it changed.
    ///
    /// A soname path that already resolves to the winner's file (same device and inode) is left as it is, silently,
    /// whatever its text, and also when it is that file itself (`libtcl8.6.so` with `libtcl8.6.so.0 ->
    /// libtcl8.6.so`) (probe: r-dev-alias-cache.transcript, GitHub issue #16). The winner can be a `.so` alias whose
    /// chain runs through the soname link itself (`libcmockery.so -> libcmocka.so -> libcmocka.so.0`); such a winner
    /// resolves to whatever the soname link resolves to, so the link is never pointed back into its own chain.
    fn link(&mut self, dir: &[u8], soname: &[u8], winner: &[u8], r: &mut Outcome) -> Result<bool, Oom> {
        let path = join(dir, soname)?;
        let target = join(dir, winner)?;
        if let (Ok(cur), Ok(win)) = (self.fs.stat(&path), self.fs.stat(&target)) {
            if (cur.dev, cur.ino) == (win.dev, win.ino) {
                return Ok(false);
            }
        }
        self.link_at(&path, winner, r)
    }

    /// Points the link `path` at `target` unless it already does; true when it changed. Anything other than a
    /// symbolic link at `path` is left alone and reported (probe: n-not-link.transcript, l-edge.transcript).
    fn link_at(&mut self, path: &[u8], target: &[u8], r: &mut Outcome) -> Result<bool, Oom> {
        match self.fs.lstat(path) {
            Ok(m) if m.kind == Kind::Link => {
                let mut cur = Vec::new();
                if self.fs.read_link(path, &mut cur).is_ok() && cur == target {
                    return Ok(false);
                }
            }
            Ok(_) => {
                r.err.bytes(self.me).bytes(b": ").bytes(path).bytes(b" is not a symbolic link\n\n");
                return Ok(false);
            }
            Err(_) => {}
        }
        match self.fs.replace_link(target, path) {
            Ok(()) => Ok(true),
            Err(e) => {
                r.err.bytes(self.me).bytes(b": Cannot link ").bytes(path).bytes(b" to ").bytes(target);
                errno_line(self.fs, &mut r.err, e);
                Ok(false)
            }
        }
    }

    /// `-l`: each operand, in argument order, is one library; `<its directory>/<soname>` is pointed at its file
    /// name. No directory is scanned and no cache is written, whatever `-N` says; a later operand of the same
    /// soname wins (probes: validation/transcripts/ldconfig/l-*.transcript). Every operand is reported on its own
    /// and the status stays 0.
    fn libs(&mut self, r: &mut Outcome) -> Result<(), Oom> {
        for &op in &self.o.operands {
            match self.fs.lstat(op) {
                Ok(m) if m.kind == Kind::File => {}
                Ok(_) => {
                    r.err.bytes(self.me).bytes(b": Ignored file ").bytes(op).bytes(b" since it is not a regular file.\n");
                    continue;
                }
                Err(e) => {
                    r.err.bytes(self.me).bytes(b": Cannot lstat ").bytes(op);
                    errno_line(self.fs, &mut r.err, e);
                    continue;
                }
            }
            // The operand's directory (`/` for `/name`; none for a bare `name`, which stays relative) and file name.
            let (dir, file) = match op.iter().rposition(|&c| c == b'/') {
                Some(0) => (Some(&b"/"[..]), op.get(1..).unwrap_or_default()),
                Some(k) => (op.get(..k), op.get(k.saturating_add(1)..).unwrap_or_default()),
                None => (None, op),
            };
            let soname = match elf_info::<A, F>(self.fs, op)? {
                Info::Lib(Some(s), _) => s,
                Info::Lib(None, _) => {
                    let mut v = Vec::new();
                    extend(&mut v, file)?;
                    v
                }
                found => {
                    if found == Info::Truncated {
                        r.err.bytes(self.me).bytes(b": file ").bytes(op).bytes(b" is truncated\n\n");
                    }
                    r.err.bytes(self.me).bytes(b": No link created since soname could not be found for ").bytes(op).bytes(b"\n");
                    continue;
                }
            };
            let changed = if soname.as_slice() != file && !self.o.no_links {
                let path = match dir {
                    Some(d) => join(d, &soname)?,
                    None => {
                        let mut p = Vec::new();
                        extend(&mut p, &soname)?;
                        p
                    }
                };
                self.link_at(&path, file, r)?
            } else {
                false
            };
            if self.o.verbose {
                r.out.bytes(b"\t").bytes(&soname).bytes(b" -> ").bytes(file);
                if changed {
                    r.out.bytes(b" (changed)");
                }
                r.out.bytes(b"\n");
            }
        }
        Ok(())
    }

    /// A cache entry `key` => `<canon>/<file>` (`canon`: the scanned directory, a hwcaps one included).
    fn record(&mut self, canon: Option<&[u8]>, key: &[u8], file: &[u8], hwcaps: Option<&[u8]>, order: u32, flags: u32) -> Result<(), Oom> {
        let (Some(dir), false) = (canon, self.o.no_cache) else { return Ok(()) };
        let mut path = Vec::new();
        extend(&mut path, dir)?;
        if path.as_slice() != b"/" {
            push(&mut path, b'/')?;
        }
        extend(&mut path, file)?;
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
        push(&mut self.recs, Rec { key: k, path, hwcaps: h, order, flags })
    }
}

/// The whole run after option parsing; `None` = out of memory.
pub(super) fn main<A: Arch, F: Fs>(me: &[u8], o: &Options<'_>, fs: &mut F, generator: &[u8], r: &mut Outcome) -> Option<u8> {
    go::<A, F>(me, o, fs, generator, r).ok()
}

fn go<A: Arch, F: Fs>(me: &[u8], o: &Options<'_>, fs: &mut F, generator: &[u8], r: &mut Outcome) -> Result<u8, Oom> {
    if let Some(f) = o.format {
        if f == b"old" || f == b"compat" {
            r.err.bytes(me).bytes(b": cache format `").bytes(f).bytes(b"' is not supported: libniji writes only the new format (-c new)\n");
            return Ok(1);
        }
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
    if o.library_mode {
        let mut run = Run::<A, F> { fs, me, o, recs: Vec::new(), _arch: core::marker::PhantomData };
        run.libs(r)?;
        return Ok(0);
    }
    // 1. The directories, in order: operands, configuration, trusted; each checked as it is named.
    let mut reader = Reader::new(fs, o.root.is_some());
    for &op in &o.operands {
        let mut d = conf::Dir { path: Vec::new(), from: Vec::new(), line: 0 };
        extend(&mut d.path, op)?;
        extend(&mut d.from, b"<cmdline>")?;
        reader.add(d, me, &mut r.err)?;
    }
    if !o.only_operands {
        let conf_path = o.conf.unwrap_or(CONF);
        match reader.read(conf_path, conf_path, me, &mut r.err) {
            Ok(()) => {}
            Err(Fatal::Oom) => return Err(Oom),
            Err(Fatal::RelativeUnderRoot) => {
                r.err.bytes(me).bytes(b": need absolute file name for configuration file when using -r\n");
                return Ok(1);
            }
        }
        for &t in A::SYSTEM_DIRS {
            let mut d = conf::Dir { path: Vec::new(), from: Vec::new(), line: 0 };
            extend(&mut d.path, t.as_bytes())?;
            extend(&mut d.from, b"<builtin>")?;
            reader.add(d, me, &mut r.err)?;
        }
    }
    let keep = core::mem::take(&mut reader.kept);
    drop(reader);
    // 2. Scan: each directory, then its glibc-hwcaps subdirectories (sorted) after it.
    let mut run = Run::<A, F> { fs, me, o, recs: Vec::new(), _arch: core::marker::PhantomData };
    for (idx, (s, _)) in keep.iter().enumerate() {
        if o.verbose {
            r.out.bytes(&s.path).bytes(b": (from ").bytes(&s.from).bytes(b":").num(u64::from(s.line)).bytes(b")\n");
        }
        let order = u32::try_from(idx).unwrap_or(u32::MAX);
        run.dir(&s.path, None, order, r)?;
        let hw = join(&s.path, b"glibc-hwcaps")?;
        if matches!(run.fs.stat(&hw), Ok(m) if m.kind == Kind::Dir) {
            let mut subs = Vec::new();
            if run.fs.list(&hw, &mut subs).is_ok() {
                subs.sort_unstable();
                for sub in subs.iter().filter(|n| name_ok(n) && n.first() != Some(&b'.')) {
                    let p = join(&hw, sub)?;
                    if matches!(run.fs.stat(&p), Ok(m) if m.kind == Kind::Dir) {
                        if o.verbose {
                            r.out.bytes(&p).bytes(b": (hwcap: \"").bytes(sub).bytes(b"\") (from ").bytes(&s.from).bytes(b":").num(u64::from(s.line)).bytes(b")\n");
                        }
                        run.dir(&p, Some(sub), order, r)?;
                    }
                }
            }
        }
    }
    if o.no_cache || o.only_operands {
        return Ok(0);
    }
    // 4. The cache.
    let mut entries: Vec<Entry<'_>> = Vec::new();
    for rec in &run.recs {
        push(&mut entries, Entry { flags: rec.flags, key: &rec.key, path: &rec.path, hwcaps: rec.hwcaps.as_deref(), order: rec.order })?;
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
