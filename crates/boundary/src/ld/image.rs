//! Reading and mapping objects: opening files and identifying them, the views of mapped images
//! the relocator reads, the program's own image, the system-call `Mapper` and relocation
//! `Image`, the cache file, and the candidates of a search.

use core::ffi::c_void;
use core::sync::atomic::Ordering::Relaxed;
use niji_kernel::uapi::{O_CLOEXEC, O_RDONLY, SEEK_END};
use niji_ld::arch::Native;
use niji_ld::program;
use niji_ld::cache::Cache;
use niji_ld::dlopen::Refusal;
use niji_ld::search::{self, PathBuf};
use niji_ld::{boot, elf, map, start};
use crate::{conv, raw};
use super::io::{c_string, fail_with, refuse, write_all};
use super::state::OWN_BASE;

/// The identity of an open file, (`st_dev`, `st_ino`): one object reached by two paths (a symlink, a
/// hard link) is the same object (L14-4).
pub(super) type FileId = (u64, u64);

/// The `st_mode` of the open descriptor `fd` (0 when `fstat` fails: no bit set, never admitted).
pub(super) fn file_mode(fd: i32) -> u32 {
    let mut st = niji_kernel::uapi::Stat::default();
    if crate::ffi_sys::niji_sys_fstat(i64::from(fd), core::ptr::from_mut(&mut st).cast::<c_void>()) != 0 {
        return 0;
    }
    st.mode
}

/// Open and map read-only the file at the NUL-terminated path `path`: (descriptor, mapping, bytes,
/// identity).
pub(super) fn open_file(path: usize) -> Option<(i32, usize, &'static [u8], FileId)> {
    let fd = crate::ffi_sys::niji_sys_openat(AT_FDCWD, core::ptr::with_exposed_provenance(path), i64::from(O_RDONLY | O_CLOEXEC), 0);
    let fd32 = i32::try_from(fd).ok().filter(|&f| f >= 0)?;
    let mut st = niji_kernel::uapi::Stat::default();
    if crate::ffi_sys::niji_sys_fstat(fd, core::ptr::from_mut(&mut st).cast::<c_void>()) != 0 {
        let _ = crate::ffi_sys::niji_sys_close(fd);
        return None;
    }
    let size = crate::ffi_sys::niji_sys_lseek(fd, 0, i64::from(SEEK_END));
    let Some(len) = usize::try_from(size).ok().filter(|&n| n >= niji_ld::elf::EHDR_SIZE) else {
        let _ = crate::ffi_sys::niji_sys_close(fd);
        return None;
    };
    let Ok(at) = raw::mmap(0, len, map::PROT_READ, MAP_PRIVATE_I32, fd32, 0) else {
        let _ = crate::ffi_sys::niji_sys_close(fd);
        return None;
    };
    // UNSAFE(slice): an object file, mapped read-only above.
    #[allow(unsafe_code)]
    // SAFETY: `len` bytes from `at` are a private read-only mapping of the file, unmapped only after
    // the last use of the slice (the end of `link`, or `load`).
    Some((fd32, at, unsafe { conv::bytes_at(at, len) }, (st.dev, st.ino)))
}

/// A new private anonymous read-write mapping of `len` (> 0) bytes: (address, bytes).
pub(super) fn scratch(len: usize, name: &[u8]) -> (usize, &'static mut [u8]) {
    let Ok(at) = raw::mmap(0, len, map::PROT_READ | map::PROT_WRITE, MAP_PRIVATE_I32 | MAP_ANONYMOUS_I32, -1, 0) else {
        fail_with(&[name, b": out of memory for the program's tables\n"], program::FAILURE)
    };
    // UNSAFE(slice): a fresh anonymous mapping, owned by the caller.
    #[allow(unsafe_code)]
    // SAFETY: `len` bytes from `at` were just mapped private, readable and writable; nothing else
    // refers to them, and the caller unmaps them only after the last use of the slice.
    (at, unsafe { conv::bytes_at_mut(at, len) })
}

/// Copy `out.len()` bytes of the mapped program at run-time address `at` into `out` (D-27.18).
pub(super) fn read_image(at: usize, out: &mut [u8]) {
    // UNSAFE(slice): bytes of the mapped program, copied before any relocation store.
    #[allow(unsafe_code)]
    // SAFETY: [at, at + len) is the program header table (`AT_PHDR`, ⚑ KERN), the start of its (whole, readable)
    // page, or the file-backed part of one readable `PT_LOAD` mapped by the kernel, `load`, or (ld.so's own ELF
    // header, D-27.23) its first PT_LOAD; the slice lives only for this copy, and nothing stores meanwhile.
    out.copy_from_slice(unsafe { conv::bytes_at(at, out.len()) });
}

/// The `len` file-backed bytes at run-time address `at` of a program `PT_LOAD` that
/// [`niji_ld::elf::Elf::image_span_kind`] calls [`niji_ld::elf::SpanKind::Live`], read in place.
fn live_segment(at: usize, len: usize) -> &'static [u8] {
    // UNSAFE(slice): a PT_LOAD of the program read in place (SpanKind::Live).
    #[allow(unsafe_code)]
    // SAFETY: the caller checked the range is the file-backed part of a readable PT_LOAD, mapped for the
    // process's lifetime. Live means: not writable, the program is not text-relocated, and no segment the
    // relocator may store into overlaps it; the relocator stores only inside writable segments of such an
    // object (reloc `inside`; ld.so's own image not at all), so the bytes do not change until `link` ends.
    unsafe { conv::bytes_at(at, len) }
}

/// `n` empty spans laid over `bytes` (the unique borrow of a fresh scratch mapping's tail, consumed
/// here): the image reader's span list, sized from the program's header count, since the number of
/// `PT_LOAD`s has no bound (D-27.15).
fn span_slots(bytes: &'static mut [u8], n: usize, name: &[u8]) -> &'static mut [niji_ld::elf::Span<'static>] {
    use niji_ld::elf::Span;
    let p = bytes.as_mut_ptr().cast::<Span<'static>>();
    if !p.is_aligned() || n.checked_mul(size_of::<Span<'static>>()).is_none_or(|need| need > bytes.len()) {
        fail_with(&[name, b": malformed program headers\n"], program::FAILURE)
    }
    for i in 0..n {
        let slot = p.wrapping_add(i);
        // UNSAFE(slice): one slot of the span list, initialised before the list is formed (D-27.18).
        #[allow(unsafe_code)]
        // SAFETY: `bytes` is the only reference to its memory and is consumed here; `slot` is aligned for `Span`
        // and inside it (`i < n`, and `n` spans fit: checked above).
        unsafe {
            slot.write(Span { vaddr: 0, bytes: &[] });
        }
    }
    // UNSAFE(slice): the span list, in a fresh anonymous mapping (D-27.18).
    #[allow(unsafe_code)]
    // SAFETY: `p` is aligned, `n` spans fit and every one holds a valid empty span (written above); nothing
    // else refers to the memory, and the mapping is unmapped only at the end of `link`, after the last use.
    unsafe {
        core::slice::from_raw_parts_mut(p, n)
    }
}

/// The program as the reader sees it, read from its memory image (ruling #4229, D-27.18): its program
/// header table and `PT_DYNAMIC` copied into one anonymous mapping, each `PT_LOAD` whose bytes cannot
/// change during loading sliced in place, and each that can change and holds a table copied into a
/// second one, all before the first relocation store. The span list lives in the first mapping, after
/// the table. Returns the table and the spans; each mapping made is stored in `maps`.
// The span index stays below `spans.len()` (checked before each store).
#[allow(clippy::indexing_slicing)]
pub(super) fn image_view(
    prog: &start::Program,
    bias: usize,
    name: &[u8],
    maps: &mut [(usize, usize)],
) -> (&'static [u8], &'static [niji_ld::elf::Span<'static>]) {
    use niji_ld::elf::{pt, Elf, Span, SpanKind, PHDR_SIZE};
    let bad = |what: &[u8]| -> ! { fail_with(&[name, what], program::FAILURE) };
    let table = prog.phnum.saturating_mul(PHDR_SIZE);
    // Pass 1: the header table alone, to find PT_DYNAMIC and size the first mapping. One span per
    // PT_LOAD and one for PT_DYNAMIC's copy: at most phnum (PT_DYNAMIC, required below, is a header
    // that is not a PT_LOAD), in the same mapping after the table (a multiple of 8 bytes from a page
    // boundary, so aligned for them). Any number of PT_LOADs (D-27.15).
    let whole = prog.phnum.saturating_mul(size_of::<Span<'static>>()).saturating_add(table);
    let (at, buf) = scratch(whole, name);
    let (buf, tail) = buf.split_at_mut(table);
    read_image(prog.phdr, buf);
    let head: &'static [u8] = buf;
    let spans = span_slots(tail, prog.phnum, name);
    let probe = Elf::from_image::<Native>(head, &[]).unwrap_or_else(|_| bad(b": malformed program headers\n"));
    let loads = || probe.program_headers().filter(|p| p.p_type == pt::LOAD);
    let run_at = |vaddr: u64| bias.wrapping_add(usize::try_from(vaddr).unwrap_or(usize::MAX));
    // A range is read from memory only when it lies in the file-backed part of one readable PT_LOAD.
    let mapped = |vaddr: u64, len: u64| {
        loads().any(|p| p.flags & 4 != 0 && p.vaddr <= vaddr && vaddr.checked_add(len).is_some_and(|e| p.vaddr.checked_add(p.filesz).is_some_and(|f| e <= f)))
    };
    let dynph = probe.find(pt::DYNAMIC).unwrap_or_else(|| bad(b": no valid dynamic section\n"));
    let dlen = usize::try_from(dynph.filesz).unwrap_or(usize::MAX);
    if !mapped(dynph.vaddr, dynph.filesz) || dlen == 0 {
        bad(b": no valid dynamic section\n");
    }
    maps[0] = (at, whole);
    let (at, buf) = scratch(dlen, name);
    read_image(run_at(dynph.vaddr), buf);
    maps[1] = (at, dlen);
    let dynamic: &'static [u8] = buf;
    // Pass 2: the PT_LOADs that change and hold a table, copied into one mapping.
    let copy_len = loads().filter(|p| probe.image_span_kind(p, dynamic) == SpanKind::Copy).fold(0usize, |a, p| a.saturating_add(usize::try_from(p.filesz).unwrap_or(usize::MAX)));
    let mut copies: &'static mut [u8] = &mut [];
    if copy_len != 0 {
        let (at, buf) = scratch(copy_len, name);
        maps[2] = (at, copy_len);
        copies = buf;
    }
    let mut k = 0usize;
    for kind in [SpanKind::Copy, SpanKind::Live] {
        for p in loads().filter(|p| probe.image_span_kind(p, dynamic) == kind) {
            let len = usize::try_from(p.filesz).unwrap_or(usize::MAX);
            if !mapped(p.vaddr, p.filesz) || k == spans.len() {
                bad(b": malformed program headers\n");
            }
            let bytes: &'static [u8] = if kind == SpanKind::Copy {
                let (now, rest) = core::mem::take(&mut copies).split_at_mut(len);
                read_image(run_at(p.vaddr), now);
                copies = rest;
                now
            } else {
                live_segment(run_at(p.vaddr), len)
            };
            spans[k] = Span { vaddr: p.vaddr, bytes };
            k = k.saturating_add(1);
        }
        if kind == SpanKind::Copy {
            // After the whole-segment copies, before the live slices: PT_DYNAMIC's own copy.
            if k == spans.len() {
                bad(b": malformed program headers\n");
            }
            spans[k] = Span { vaddr: dynph.vaddr, bytes: dynamic };
            k = k.saturating_add(1);
        }
    }
    let spans: &'static [Span<'static>] = spans;
    (head, &spans[..k])
}

/// A dependency's mapped `file` as the relocator sees it (load bias `bias`). Its `name` may come from
/// the program's string table (an image span), so any lifetime.
pub(super) fn parse<'a>(file: &'static [u8], name: &'a [u8], bias: u64) -> niji_ld::reloc::Obj<'a> {
    let elf = niji_ld::elf::Elf::parse::<Native>(file).unwrap_or_else(|_| fail_with(&[name, b": not a valid ELF file for this machine\n"], program::FAILURE));
    let dynm = elf.dynamic().ok().flatten().unwrap_or_else(|| fail_with(&[name, b": no valid dynamic section\n"], program::FAILURE));
    niji_ld::reloc::Obj { elf, dynm, bias, name, tls: None }
}

/// The interpreter as an object of the scope (D-27.20), read from its own memory image like the
/// program ([`image_view`], D-27.23): its ELF header at [`OWN_BASE`] (the first bytes of its first
/// `PT_LOAD`) gives its program header table; no file is opened. It relocated itself at its start,
/// so its writable segments already hold their final values and the reader copies those holding a
/// table before the relocator runs, as for the program. The scratch mappings go to `maps`.
pub(super) fn own_object(name: &'static [u8], maps: &mut [(usize, usize)]) -> niji_ld::reloc::Obj<'static> {
    let base = OWN_BASE.load(Relaxed);
    let mut header = [0u8; niji_ld::elf::EHDR_SIZE];
    read_image(base, &mut header);
    let Some((phoff, phnum)) = boot::own_headers(&header) else { fail_with(&[name, b": malformed program headers\n"], program::FAILURE) };
    let me = start::Program { phdr: base.wrapping_add(usize::try_from(phoff).unwrap_or(usize::MAX)), phnum, entry: 0 };
    let (head, view) = image_view(&me, base, name, maps);
    let elf = niji_ld::elf::Elf::from_image::<Native>(head, view).unwrap_or_else(|_| fail_with(&[name, b": malformed program headers\n"], program::FAILURE));
    let dynm = elf.dynamic().ok().flatten().unwrap_or_else(|| fail_with(&[name, b": no valid dynamic section\n"], program::FAILURE));
    niji_ld::reloc::Obj { elf, dynm, bias: u64::try_from(base).unwrap_or(0), name, tls: None }
}

/// The process's memory as the relocator writes it ([`niji_ld::reloc::Image`]).
pub(super) struct ProcImage;

impl niji_ld::reloc::Image for ProcImage {
    fn store(&mut self, at: u64, bytes: &[u8]) {
        let Ok(at) = word(at) else { return };
        // UNSAFE(slice): a relocated field of a loaded object.
        #[allow(unsafe_code)]
        // SAFETY: the relocator checked [at, at + len) lies in a segment of the object that is mapped
        // writable at this moment, and nothing else of the process runs yet.
        unsafe { conv::bytes_at_mut(at, bytes.len()) }.copy_from_slice(bytes);
    }

    fn load(&mut self, at: u64, out: &mut [u8]) {
        let Ok(at) = word(at) else { return };
        // UNSAFE(slice): a word of a loaded object's segment.
        #[allow(unsafe_code)]
        // SAFETY: the relocator checked the range lies in a mapped, readable segment of the object.
        out.copy_from_slice(unsafe { conv::bytes_at(at, out.len()) });
    }

    fn copy(&mut self, to: u64, from: u64, len: u64) {
        let (Ok(to), Ok(from), Ok(len)) = (word(to), word(from), word(len)) else { return };
        // UNSAFE(slice): a provider's data, the source of a COPY relocation.
        #[allow(unsafe_code)]
        // SAFETY: the relocator checked [from, from + len) lies in a mapped segment of the provider.
        let src = unsafe { conv::bytes_at(from, len) };
        // UNSAFE(slice): the program's copy of that data.
        #[allow(unsafe_code)]
        // SAFETY: the relocator checked [to, to + len) lies in a writable segment of the program, a
        // different object from the provider (so the ranges do not overlap).
        unsafe { conv::bytes_at_mut(to, len) }.copy_from_slice(src);
    }

    fn call(&mut self, resolver: u64) -> u64 {
        let v = crate::ffi::niji_machine_call0(word(resolver).unwrap_or(0));
        u64::from_ne_bytes(v.to_ne_bytes())
    }

    fn protect(&mut self, at: u64, len: u64, prot: i32) -> Result<(), ()> {
        raw::mprotect(word(at)?, word(len)?, prot).map_err(|_| ())
    }

    fn tlsdesc_static(&mut self) -> Option<u64> {
        Some(crate::arch::tlsdesc_static())
    }
}

/// `AT_FDCWD` (kernel UAPI `linux/fcntl.h`): a relative path is taken from the current directory.
pub(super) const AT_FDCWD: i64 = -100;

/// Open, read and map the program file named by the NUL-terminated path at `path` (`ld.so PROGRAM`)
/// with pages of `page` bytes: [`map::load`] (one reservation, each `PT_LOAD` inside it, nothing left
/// mapped on failure). The program and its load bias.
pub(super) fn load(path: usize, page: u64) -> (start::Program, usize) {
    let fail = |msg: &[u8]| -> ! {
        write_all(2, b"ld.so: ");
        write_all(2, c_string(path));
        write_all(2, b": ");
        refuse(msg)
    };
    let fd = crate::ffi_sys::niji_sys_openat(AT_FDCWD, core::ptr::with_exposed_provenance(path), i64::from(O_RDONLY | O_CLOEXEC), 0);
    if fd < 0 {
        fail(b"cannot open the program\n");
    }
    let size = crate::ffi_sys::niji_sys_lseek(fd, 0, i64::from(SEEK_END));
    let len = usize::try_from(size).ok().filter(|&n| n >= niji_ld::elf::EHDR_SIZE).unwrap_or_else(|| fail(b"cannot read the program\n"));
    let fd32 = i32::try_from(fd).unwrap_or(-1);
    let file_at = raw::mmap(0, len, map::PROT_READ, MAP_PRIVATE_I32, fd32, 0).unwrap_or_else(|_| fail(b"cannot read the program\n"));
    // UNSAFE(slice): the program file, mapped read-only above.
    #[allow(unsafe_code)]
    // SAFETY: `len` bytes from `file_at` are a private read-only mapping of the file, unmapped only
    // after the last use of this slice below.
    let file = unsafe { conv::bytes_at(file_at, len) };
    let loaded = map::load::<Native, _>(&mut SysMapper { fd: fd32 }, file, page);
    let _ = raw::munmap(file_at, len);
    let _ = crate::ffi_sys::niji_sys_close(fd);
    let loaded = loaded.unwrap_or_else(|e| fail(e.message()));
    let at = |v: u64| usize::try_from(v).unwrap_or(0);
    (start::Program { phdr: at(loaded.phdr), phnum: usize::from(loaded.phnum), entry: at(loaded.entry) }, at(loaded.bias))
}

/// The mapper's system calls ([`map::Mapper`]) on one object's file descriptor.
pub(super) struct SysMapper {
    pub(super) fd: i32,
}

impl map::Mapper for SysMapper {
    fn reserve(&mut self, at: Option<u64>, len: u64) -> Result<u64, ()> {
        let (hint, flags) = match at {
            Some(a) => (a, MAP_PRIVATE_I32 | MAP_ANONYMOUS_I32 | MAP_FIXED_NOREPLACE_I32),
            None => (0, MAP_PRIVATE_I32 | MAP_ANONYMOUS_I32),
        };
        let r = raw::mmap(word(hint)?, word(len)?, 0, flags, -1, 0).map_err(|_| ())?;
        u64::try_from(r).map_err(|_| ())
    }

    fn map_file(&mut self, at: u64, len: u64, prot: i32, offset: u64) -> Result<(), ()> {
        let off = i64::try_from(offset).map_err(|_| ())?;
        raw::mmap(word(at)?, word(len)?, prot, MAP_PRIVATE_I32 | MAP_FIXED_I32, self.fd, off).map(|_| ()).map_err(|_| ())
    }

    fn map_zero(&mut self, at: u64, len: u64, prot: i32) -> Result<(), ()> {
        raw::mmap(word(at)?, word(len)?, prot, MAP_PRIVATE_I32 | MAP_FIXED_I32 | MAP_ANONYMOUS_I32, -1, 0).map(|_| ()).map_err(|_| ())
    }

    fn store(&mut self, at: u64, len: u64, bytes: Option<&[u8]>) {
        let (Ok(at), Ok(len)) = (word(at), word(len)) else { return };
        // UNSAFE(slice): bytes of a segment page the mapper mapped writable just before this call.
        #[allow(unsafe_code)]
        // SAFETY: `map::load` calls `store` only on [at, at + len) inside a page it mapped private and
        // writable through this mapper, before anything else can see the object.
        let dst = unsafe { conv::bytes_at_mut(at, len) };
        match bytes {
            Some(b) => dst.iter_mut().zip(b).for_each(|(d, s)| *d = *s),
            None => dst.fill(0),
        }
    }

    fn protect(&mut self, at: u64, len: u64, prot: i32) -> Result<(), ()> {
        raw::mprotect(word(at)?, word(len)?, prot).map_err(|_| ())
    }

    fn unmap(&mut self, at: u64, len: u64) {
        if let (Ok(at), Ok(len)) = (word(at), word(len)) {
            let _ = raw::munmap(at, len);
        }
    }
}

/// An address or length as a machine word.
fn word(v: u64) -> Result<usize, ()> {
    usize::try_from(v).map_err(|_| ())
}

/// `mmap` flags as `i32` (kernel UAPI `asm-generic/mman*.h`, `linux/mman.h`).
pub(super) const MAP_PRIVATE_I32: i32 = 0x02;
const MAP_FIXED_I32: i32 = 0x10;
pub(super) const MAP_ANONYMOUS_I32: i32 = 0x20;
const MAP_FIXED_NOREPLACE_I32: i32 = 0x10_0000;

/// Open the NUL-terminated `path` and map the whole file read-only: the file and its descriptor
/// (list mode; [`open_file`] is the run path's opener, which also gives the file's identity).
pub(super) fn open_whole(path: &[u8]) -> Option<(&'static [u8], i64)> {
    let fd = crate::ffi_sys::niji_sys_openat(AT_FDCWD, path.as_ptr().cast(), i64::from(O_RDONLY | O_CLOEXEC), 0);
    if fd < 0 {
        return None;
    }
    let len = usize::try_from(crate::ffi_sys::niji_sys_lseek(fd, 0, i64::from(SEEK_END))).ok().filter(|&n| n > 0);
    let at = len.and_then(|len| raw::mmap(0, len, map::PROT_READ, MAP_PRIVATE_I32, i32::try_from(fd).unwrap_or(-1), 0).ok());
    let (Some(len), Some(at)) = (len, at) else {
        let _ = crate::ffi_sys::niji_sys_close(fd);
        return None;
    };
    // UNSAFE(slice): a file mapped read-only above, never unmapped while the slice is in use.
    #[allow(unsafe_code)]
    // SAFETY: `len` bytes from `at` are a private read-only mapping of the file; the list mode keeps
    // every such mapping until the process exits (the caller unmaps only a file it rejects, after its
    // last use).
    let file = unsafe { conv::bytes_at(at, len) };
    Some((file, fd))
}

/// The cache (R14-3 (a): `<prefix>/etc/ld.so.cache`), mapped read-only and parsed: the mapping and
/// the reader, or `None` when it is missing, unreadable or inconsistent ("no cache", threat model
/// rule 5; an inconsistent file is unmapped). One reader for program start, `--list` and `dlopen`.
pub(super) fn read_cache() -> Option<(&'static [u8], Cache<'static>)> {
    let mut path = PathBuf::default();
    let (f, fd) = if search::prefixed(b"etc/ld.so.cache", &mut path) { path.c_str().and_then(open_whole) } else { None }?;
    let _ = crate::ffi_sys::niji_sys_close(fd);
    let parsed = Cache::parse(f);
    if parsed.is_none() {
        let _ = raw::munmap(f.as_ptr().expose_provenance(), f.len());
    }
    parsed.map(|c| (f, c))
}

/// Why a program-start candidate is not used: passed over (not openable, another machine's object
/// or another ELF class) or refused (the start ends).
#[derive(Clone, Copy)]
pub(super) enum Unusable {
    Skip,
    Refused(Refusal),
}

/// Open, map read-only and check the NUL-terminated candidate `path` a program-start search (or the
/// dependency list) found: (descriptor, mapping, bytes, identity), or why it is not used, by the
/// probed rule [`Refusal::skipped_at_start`]. Pages of `page` bytes.
pub(super) fn open_candidate(path: &[u8], page: u64) -> Result<(i32, usize, &'static [u8], FileId), Unusable> {
    let fd = crate::ffi_sys::niji_sys_openat(AT_FDCWD, path.as_ptr().cast(), i64::from(O_RDONLY | O_CLOEXEC), 0);
    let fd32 = i32::try_from(fd).ok().filter(|&f| f >= 0).ok_or(Unusable::Skip)?;
    let refuse = |r: Refusal| {
        let _ = crate::ffi_sys::niji_sys_close(fd);
        if r.skipped_at_start() { Unusable::Skip } else { Unusable::Refused(r) }
    };
    let mut st = niji_kernel::uapi::Stat::default();
    if crate::ffi_sys::niji_sys_fstat(fd, core::ptr::from_mut(&mut st).cast::<c_void>()) != 0 {
        let _ = crate::ffi_sys::niji_sys_close(fd);
        return Err(Unusable::Skip);
    }
    if st.mode & niji_kernel::uapi::S_IFMT == niji_kernel::uapi::S_IFDIR {
        return Err(refuse(Refusal::Directory));
    }
    let size = crate::ffi_sys::niji_sys_lseek(fd, 0, i64::from(SEEK_END));
    let Some(len) = usize::try_from(size).ok().filter(|&n| n >= niji_ld::elf::EHDR_SIZE) else { return Err(refuse(Refusal::TooShort)) };
    let Ok(at) = raw::mmap(0, len, map::PROT_READ, MAP_PRIVATE_I32, fd32, 0) else { return Err(refuse(Refusal::Truncated)) };
    // UNSAFE(slice): a candidate file, mapped read-only above.
    #[allow(unsafe_code)]
    // SAFETY: `len` bytes from `at` are a private read-only mapping of the file, unmapped here when the
    // file is not used, else only after the caller's last use of the slice (the end of `link`; the
    // list mode keeps it until the process exits).
    let file = unsafe { conv::bytes_at(at, len) };
    if let Err(r) = niji_ld::dlopen::check::<Native>(file, page) {
        let _ = raw::munmap(at, len);
        return Err(refuse(r));
    }
    Ok((fd32, at, file, (st.dev, st.ino)))
}

/// The dynamic section of the object file `file`, when it is one of this architecture's.
pub(super) fn dynamic_of(file: &'static [u8]) -> Option<elf::Dynamic<'static>> {
    elf::Elf::parse::<Native>(file).ok()?.dynamic().ok().flatten()
}

/// The string of dynamic tag `tag` in `d`.
pub(super) fn dyn_string(d: &elf::Dynamic<'static>, tag: u64) -> Option<&'static [u8]> {
    d.get(tag).and_then(|o| d.string(o).ok())
}

/// The directory part of `path` (`$ORIGIN`): `.` for a bare name, `/` for a name in the root.
pub(super) fn dir_of(path: &[u8]) -> &[u8] {
    match path.iter().rposition(|&b| b == b'/') {
        None => b".",
        Some(0) => b"/",
        Some(i) => path.get(..i).unwrap_or(b"."),
    }
}
