//! Reading and mapping objects: opening files and identifying them, the views of mapped images
//! the relocator reads, the program's own image, the system-call `Mapper` and relocation
//! `Image`, the cache file, and the candidates of a search.

use core::ffi::c_void;
use core::sync::atomic::Ordering::Relaxed;
use niji_kernel::uapi::{O_CLOEXEC, O_RDONLY, SEEK_END};
use niji_ld::arch::{Arch, Native};
use niji_ld::program;
use niji_ld::cache::Cache;
use niji_ld::dlopen::{Checked, Refusal};
use niji_ld::search::{self, PathBuf};
use niji_ld::{boot, elf, map, start};
use crate::{conv, raw};
use super::debug;
use super::entry::List;
use super::exec_stack::{exec_stack_accepted, exec_stack_candidate, refuse_exec_stack_program};
use super::io::{c_string, exit, fail_loading, fail_with, write_all};
use super::state::OWN_BASE;
use super::table::PathStore;

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

/// The size of the open file `fd`, whose `fstat` result is `st`: a regular file's `st_size` (no
/// further system call; DYN-START-QUICK), any other kind's `lseek(fd, 0, SEEK_END)` (a block device
/// reports its size only there). Negative when unknown.
fn file_size(fd: i64, st: &niji_kernel::uapi::Stat) -> i64 {
    if st.mode & niji_kernel::uapi::S_IFMT == niji_kernel::uapi::S_IFREG {
        st.size
    } else {
        crate::ffi_sys::niji_sys_lseek(fd, 0, i64::from(SEEK_END))
    }
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
    let size = file_size(fd, &st);
    let Some(len) = usize::try_from(size).ok().filter(|&n| n >= niji_ld::elf::EHDR_SIZE) else {
        let _ = crate::ffi_sys::niji_sys_close(fd);
        return None;
    };
    let Ok(at) = map_view(fd32, len) else {
        let _ = crate::ffi_sys::niji_sys_close(fd);
        return None;
    };
    // UNSAFE(slice): an object file, mapped read-only above.
    #[allow(unsafe_code)]
    // SAFETY: `len` bytes from `at` are a private read-only mapping of the file, unmapped only after
    // the last use of the slice (the end of `link`, or `load`).
    Some((fd32, at, unsafe { conv::bytes_at(at, len) }, (st.dev, st.ino)))
}

/// Bytes of the scratch arena ([`scratch`]): one page, which holds a start's image views (the
/// program's header table, span list and dynamic array: about 1.4 KiB for a typical program).
const ARENA_LEN: usize = 4096;

/// The scratch arena: zero-filled loader data, page-aligned so it costs at most one page. Atomic
/// words, so the bytes may be written through the address ([`scratch`] hands each range out once).
#[repr(C, align(4096))]
struct Arena([core::sync::atomic::AtomicU64; ARENA_LEN / 8]);

static ARENA: Arena = Arena([const { core::sync::atomic::AtomicU64::new(0) }; ARENA_LEN / 8]);

/// Bytes of [`ARENA`] handed out: only grows, so no range is handed out twice.
static ARENA_USED: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);

/// `len` bytes of [`ARENA`] not handed out before (16-byte aligned), or `None` when they do not fit.
fn arena_take(len: usize) -> Option<usize> {
    let need = len.checked_next_multiple_of(16)?;
    let mut from = ARENA_USED.load(Relaxed);
    loop {
        let end = from.checked_add(need).filter(|&end| end <= ARENA_LEN)?;
        match ARENA_USED.compare_exchange_weak(from, end, Relaxed, Relaxed) {
            Ok(_) => return Some(ARENA.0.as_ptr().expose_provenance().wrapping_add(from)),
            Err(now) => from = now,
        }
    }
}

/// `len` (> 0) zero-filled read-write bytes, never used before: (address, the length to unmap after
/// the last use, bytes). With `arena` (only `link`'s views of the program, which end with `link`),
/// from the loader's scratch arena while it has room (length 0: nothing to unmap; the start's views
/// then cost no system call, DYN-START-QUICK); otherwise, or when it is full, a new private
/// anonymous mapping. The views read after the start (`adopt_start`, `own_object`) are always
/// mapped: taken from the arena, they made `dladdr` report `ld.so` for a `dlopen`ed object's
/// address and two `dlclose` tests fail, so the arena is kept to the one use that removes start-up
/// system calls.
fn scratch(len: usize, name: &[u8], arena: bool) -> (usize, usize, &'static mut [u8]) {
    if let Some(at) = arena.then(|| arena_take(len)).flatten() {
        // UNSAFE(slice): a range of the scratch arena, handed out once.
        #[allow(unsafe_code)]
        // SAFETY: [at, at + len) lies inside `ARENA` (`arena_take` checked the end), a static of atomic
        // words (interior mutability: writes through its address are allowed) that nothing reads or
        // writes as words; `ARENA_USED` only grows, so no other slice covers these bytes; they are
        // zero (never handed out before) and live for the process.
        return (at, 0, unsafe { conv::bytes_at_mut(at, len) });
    }
    let Ok(at) = raw::mmap(0, len, map::PROT_READ | map::PROT_WRITE, MAP_PRIVATE_I32 | MAP_ANONYMOUS_I32, -1, 0) else {
        fail_with(&[name, b": out of memory for the program's tables\n"], program::FAILURE)
    };
    // UNSAFE(slice): a fresh anonymous mapping, owned by the caller.
    #[allow(unsafe_code)]
    // SAFETY: `len` bytes from `at` were just mapped private, readable and writable; nothing else
    // refers to them, and the caller unmaps them only after the last use of the slice.
    (at, len, unsafe { conv::bytes_at_mut(at, len) })
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
/// the table. Returns the table and the spans; each buffer's (address, length to unmap: 0 when it
/// is in the scratch arena, [`scratch`]) is stored in `maps`. `published`:
/// the object's dynamic array may already hold run-time addresses (DL-DYNAMIC-RELOC: `ld.so`'s own
/// from its start, every start-time object's once `link` ended); the copies are turned back into
/// link-time values, so the reader sees what the file holds.
// The span index stays below `spans.len()` (checked before each store).
#[allow(clippy::indexing_slicing)]
pub(super) fn image_view(
    prog: &start::Program,
    bias: usize,
    name: &[u8],
    maps: &mut [(usize, usize)],
    published: bool,
) -> (&'static [u8], &'static [niji_ld::elf::Span<'static>]) {
    use niji_ld::elf::{pt, Elf, Span, SpanKind, PHDR_SIZE};
    let bad = |what: &[u8]| -> ! { fail_with(&[name, what], program::FAILURE) };
    let table = prog.phnum.saturating_mul(PHDR_SIZE);
    // Pass 1: the header table alone, to find PT_DYNAMIC and size the first mapping. One span per
    // PT_LOAD and one for PT_DYNAMIC's copy: at most phnum (PT_DYNAMIC, required below, is a header
    // that is not a PT_LOAD), in the same mapping after the table (a multiple of 8 bytes from a page
    // boundary, so aligned for them). Any number of PT_LOADs (D-27.15).
    let whole = prog.phnum.saturating_mul(size_of::<Span<'static>>()).saturating_add(table);
    // Only `link`'s view of the program (`published` false) takes the scratch arena ([`scratch`]).
    let (at, unmap, buf) = scratch(whole, name, !published);
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
    maps[0] = (at, unmap);
    let (at, unmap, buf) = scratch(dlen, name, !published);
    read_image(run_at(dynph.vaddr), buf);
    // A published array (DL-DYNAMIC-RELOC) back to link-time values: the reader's one view.
    let unpublish = published && probe.published_dynamic().is_some();
    let link_time = u64::try_from(bias).unwrap_or(0).wrapping_neg();
    if unpublish {
        niji_ld::elf::shift_address_entries(buf, link_time);
    }
    maps[1] = (at, unmap);
    let dynamic: &'static [u8] = buf;
    // Pass 2: the PT_LOADs that change and hold a table, copied into one mapping.
    let copy_len = loads().filter(|p| probe.image_span_kind(p, dynamic) == SpanKind::Copy).fold(0usize, |a, p| a.saturating_add(usize::try_from(p.filesz).unwrap_or(usize::MAX)));
    let mut copies: &'static mut [u8] = &mut [];
    if copy_len != 0 {
        let (at, unmap, buf) = scratch(copy_len, name, !published);
        maps[2] = (at, unmap);
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
                // The array's bytes in a whole-segment copy (searched before PT_DYNAMIC's own) too.
                let inner = dynph.vaddr.checked_sub(p.vaddr).and_then(|o| usize::try_from(o).ok());
                if let Some(d) = inner.and_then(|o| now.get_mut(o..o.saturating_add(dlen))).filter(|_| unpublish) {
                    niji_ld::elf::shift_address_entries(d, link_time);
                }
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
    let (head, view) = image_view(&me, base, name, maps, true);
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
        debug::note_copy(to, from, len);
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

    fn warn(&mut self, parts: &[&[u8]]) {
        write_all(2, b"ld.so: ");
        parts.iter().for_each(|p| write_all(2, p));
    }
}

/// `AT_FDCWD` (kernel UAPI `linux/fcntl.h`): a relative path is taken from the current directory.
pub(super) const AT_FDCWD: i64 = -100;

/// The file `ld.so PROGRAM` (or `--list PROGRAM`) runs for the argument `name`, NUL-terminated as
/// `name` is: `name` itself when it holds a `/`, else the path the loader cache gives for it
/// ([`program::find`], D-27.69: never the working directory). A name the cache does not have ends
/// the start as the reference does (probed): `NAME: error while loading shared libraries: NAME:
/// cannot open shared object file`, status 127. Cold: only the loader run as a command comes here.
#[cold]
#[inline(never)]
pub(super) fn program_file(name: &'static [u8], env: &List<'_>) -> &'static [u8] {
    if name.contains(&b'/') {
        return name;
    }
    let cache = if env.inhibit_cache { None } else { read_cache() };
    let ctx = env.ctx(cache.as_ref().map(|(_, c)| c).into());
    let found = match program::find(name, cache.as_ref().map(|(_, c)| c), Native::CACHE_FLAGS, ctx.hwcaps) {
        // The store's copy ends in a NUL and stays mapped for the process, as `name` does.
        program::Found::Cache(path) => PathStore::default().keep(path).map_err(|_| b"cannot allocate memory".as_slice()),
        program::Found::NotFound => Err(b"cannot open shared object file".as_slice()),
        program::Found::AsGiven => Ok(name),
    };
    if let Some((f, _)) = cache {
        let _ = raw::munmap(f.as_ptr().expose_provenance(), f.len());
    }
    found.unwrap_or_else(|text| {
        if name.is_empty() {
            // An empty name: no name before the reason (probed).
            for part in [&b": error while loading shared libraries: "[..], text, b"\n"] {
                write_all(2, part);
            }
            exit(program::FAILURE)
        }
        fail_loading(name, name, text)
    })
}

/// Open, read and map the program file named by the NUL-terminated path at `path` (`ld.so PROGRAM`)
/// with pages of `page` bytes: [`map::load`] (one reservation, each `PT_LOAD` inside it, nothing left
/// mapped on failure). The program and its load bias.
pub(super) fn load(path: usize, page: u64, ld: &start::LdEnv<'_>, secure: bool) -> (start::Program, usize) {
    // `PROGRAM: error while loading shared libraries: PROGRAM: <reason>`, 127 (the reference's form
    // and reasons for a missing file, a directory and a short file, probed; LDSO-RUN-PROGRAM).
    let fail = |msg: &[u8]| -> ! {
        let name = c_string(path);
        fail_loading(name, name, msg.strip_suffix(b"\n").unwrap_or(msg))
    };
    let fd = crate::ffi_sys::niji_sys_openat(AT_FDCWD, core::ptr::with_exposed_provenance(path), i64::from(O_RDONLY | O_CLOEXEC), 0);
    if fd < 0 {
        fail(match fd {
            -2 => niji_ld::dlopen::FILE_MISSING,
            -13 => b"cannot open shared object file: Permission denied",
            _ => b"cannot open shared object file",
        });
    }
    let fd32 = i32::try_from(fd).unwrap_or(-1);
    if file_mode(fd32) & niji_kernel::uapi::S_IFMT == niji_kernel::uapi::S_IFDIR {
        fail(b"cannot read file data: Error 21");
    }
    let size = crate::ffi_sys::niji_sys_lseek(fd, 0, i64::from(SEEK_END));
    let len = usize::try_from(size).ok().filter(|&n| n >= niji_ld::elf::EHDR_SIZE).unwrap_or_else(|| fail(b"file too short"));
    let file_at = map_view(fd32, len).unwrap_or_else(|_| fail(b"cannot read the program\n"));
    // UNSAFE(slice): the program file, mapped read-only above.
    #[allow(unsafe_code)]
    // SAFETY: `len` bytes from `file_at` are a private read-only mapping of the file, unmapped only
    // after the last use of this slice below.
    let file = unsafe { conv::bytes_at(file_at, len) };
    let loaded = match map::load::<Native, _>(&mut StartMapper { fd: fd32 }, file, page) {
        // COMPAT-19: `run` checks the program's request again and refuses it there when not accepted.
        Err(map::MapError::ExecutableStack) if exec_stack_accepted(ld, secure) => map::load_accepting_exec_stack::<Native, _>(&mut StartMapper { fd: fd32 }, file, page),
        Err(map::MapError::ExecutableStack) => refuse_exec_stack_program(0, path),
        other => other,
    };
    unmap_view(file_at, len);
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

/// [`SysMapper`] for the objects of the start (the program, its dependencies): never unmapped, so
/// an aligned reservation's slack stays reserved, inaccessible, rather than being trimmed by two
/// `munmap`s ([`map::Mapper::keeps_slack`], DYN-START-RING).
pub(super) struct StartMapper {
    pub(super) fd: i32,
}

impl map::Mapper for StartMapper {
    fn reserve(&mut self, at: Option<u64>, len: u64) -> Result<u64, ()> {
        SysMapper { fd: self.fd }.reserve(at, len)
    }

    fn map_file(&mut self, at: u64, len: u64, prot: i32, offset: u64) -> Result<(), ()> {
        SysMapper { fd: self.fd }.map_file(at, len, prot, offset)
    }

    fn map_zero(&mut self, at: u64, len: u64, prot: i32) -> Result<(), ()> {
        SysMapper { fd: self.fd }.map_zero(at, len, prot)
    }

    fn store(&mut self, at: u64, len: u64, bytes: Option<&[u8]>) {
        SysMapper { fd: self.fd }.store(at, len, bytes);
    }

    fn protect(&mut self, at: u64, len: u64, prot: i32) -> Result<(), ()> {
        SysMapper { fd: self.fd }.protect(at, len, prot)
    }

    fn unmap(&mut self, at: u64, len: u64) {
        SysMapper { fd: self.fd }.unmap(at, len);
    }

    fn keeps_slack(&self) -> bool {
        true
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
const MAP_NORESERVE_I32: i32 = 0x4000;
/// `PROT_NONE` (kernel UAPI `asm-generic/mman-common.h`).
const PROT_NONE_I32: i32 = 0;

/// The alignment of a whole-file view ([`map_view`]): the kernel's default fault-around window
/// (`fault_around_bytes`, 64 KiB), the unit in which a read of a file-backed mapping maps pages in.
const VIEW_ALIGN: usize = 0x1_0000;

/// Map the first `len` bytes of the file `fd` private and read-only at an address the loader
/// chooses, a multiple of [`VIEW_ALIGN`]: the view's pages then share fault-around windows exactly
/// as their file offsets do, so the faults its reads cost do not depend on where the kernel would
/// have put the mapping (which varies with the file system holding the file: DYN-START-SPEED-2).
/// The address range is reserved oversized and inaccessible first, the file is mapped over its
/// aligned part, and both ends are given back, so the result is one mapping of exactly `len`
/// bytes (rounded to the page) that `munmap(at, len)` removes. Nothing stays mapped on failure.
pub(super) fn map_view(fd: i32, len: usize) -> Result<usize, ()> {
    if views_hidden() {
        return map_view_hidden(fd, len, |at, n| raw::mprotect(at, n, map::PROT_READ).map_err(|_| ()));
    }
    map_view_as(fd, len, map::PROT_READ)
}

/// [`map_view`] with the protection `prot`: from the start's view arena when it has room, else on
/// its own.
#[inline(always)]
fn map_view_as(fd: i32, len: usize, prot: i32) -> Result<usize, ()> {
    if VIEWS_STATE.load(Relaxed) != VIEWS_NONE {
        if let Some(at) = views_take(len) {
            return raw::mmap(at, len, prot, MAP_PRIVATE_I32 | MAP_FIXED_I32, fd, 0).map_err(|_| ());
        }
    }
    map_view_alone(fd, len, prot)
}

/// [`VIEWS_HIDDEN`]: not asked yet.
const HIDDEN_UNKNOWN: u8 = 0;
/// [`VIEWS_HIDDEN`]: views are mapped readable (the bare machine).
const HIDDEN_NO: u8 = 1;
/// [`VIEWS_HIDDEN`]: views are mapped inaccessible, then made readable ([`map_view_hidden`]).
const HIDDEN_YES: u8 = 2;

/// Whether whole-file views are hidden from valgrind ([`views_hidden`]): asked once per process.
static VIEWS_HIDDEN: core::sync::atomic::AtomicU8 = core::sync::atomic::AtomicU8::new(HIDDEN_UNKNOWN);

/// Whether this process runs under valgrind, whose debug-information reader records every readable
/// mapping of an object file made before the object's segments, as a mapping of the object, and keeps
/// it after `munmap` (valgrind 3.24 `di_notify_mmap`, ruling #7301): a whole-file view covers every
/// read-only segment's file range, so valgrind would take those segments' load bias (`.eh_frame`,
/// `.rodata`) from the view, and every stack walk through the object would use unwind data read from
/// the wrong address. A mapping made inaccessible is not recorded, and a later `mprotect` is not
/// either. On the bare machine the views stay as they were (one `mmap` each): the client request
/// costs a few instructions once per process, never a system call.
#[inline]
fn views_hidden() -> bool {
    match VIEWS_HIDDEN.load(Relaxed) {
        HIDDEN_UNKNOWN => views_hidden_ask(),
        v => v == HIDDEN_YES,
    }
}

/// [`views_hidden`]'s first call: valgrind's `RUNNING_ON_VALGRIND` client request.
#[cold]
#[inline(never)]
fn views_hidden_ask() -> bool {
    let hidden = crate::ffi::niji_machine_running_on_valgrind() != 0;
    VIEWS_HIDDEN.store(if hidden { HIDDEN_YES } else { HIDDEN_NO }, Relaxed);
    hidden
}

/// [`map_view`] under valgrind ([`views_hidden`]): the view is mapped inaccessible, so the tool does
/// not take it for a mapping of the object, then made readable by `reveal(at, len)` (an `mprotect`).
/// Nothing stays mapped on failure. One system call per view more than [`map_view_as`].
#[cold]
#[inline(never)]
fn map_view_hidden(fd: i32, len: usize, reveal: impl FnOnce(usize, usize) -> Result<(), ()>) -> Result<usize, ()> {
    let at = map_view_as(fd, len, PROT_NONE_I32)?;
    if reveal(at, len).is_err() {
        unmap_view(at, len);
        return Err(());
    }
    Ok(at)
}

/// [`map_view`] outside the view arena: its own aligned reservation, trimmed to the view.
fn map_view_alone(fd: i32, len: usize, prot: i32) -> Result<usize, ()> {
    const PAGE: usize = 0x1000;
    let mapped = len.checked_add(PAGE - 1).ok_or(())? & !(PAGE - 1);
    let span = mapped.checked_add(VIEW_ALIGN).ok_or(())?;
    let raw = raw::mmap(0, span, 0, MAP_PRIVATE_I32 | MAP_ANONYMOUS_I32, -1, 0).map_err(|_| ())?;
    // `raw` is page-aligned, so `at` is within the first VIEW_ALIGN - PAGE bytes and the view fits.
    let at = raw.wrapping_add(VIEW_ALIGN - 1) & !(VIEW_ALIGN - 1);
    if raw::mmap(at, len, prot, MAP_PRIVATE_I32 | MAP_FIXED_I32, fd, 0).is_err() {
        let _ = raw::munmap(raw, span);
        return Err(());
    }
    // User addresses (below 2^47) plus lengths the kernel accepted: none of these wrap.
    if at != raw {
        let _ = raw::munmap(raw, at.wrapping_sub(raw));
    }
    let end = at.wrapping_add(mapped);
    let _ = raw::munmap(end, raw.wrapping_add(span).wrapping_sub(end));
    Ok(at)
}

/// Address space of the start's view arena ([`views_arm`]): inaccessible until a view is mapped
/// into it, so it commits no memory; a start whose views do not fit maps the rest on their own.
/// 64 MiB (libc.so.6's view is about 6 MiB): every mapping made while it is open lands below it, and
/// at 1 GiB that moved them into another 1 GiB page-table range than the loader's. (The 1 GiB build
/// measured +11.6k kernel instructions on an empty main on the Zen 1 host, together with a text
/// layout split fixed at the same time; the two were not measured apart.)
const VIEWS_LEN: usize = 64 << 20;

/// [`VIEWS_STATE`]: no arena (every view is mapped on its own).
const VIEWS_NONE: u8 = 0;
/// [`VIEWS_STATE`]: armed by `link`, reserved at the first view.
const VIEWS_ARMED: u8 = 1;
/// [`VIEWS_STATE`]: reserved at [`VIEWS_AT`].
const VIEWS_OPEN: u8 = 2;

/// Whether the start's views come from the arena ([`VIEWS_NONE`], [`VIEWS_ARMED`], [`VIEWS_OPEN`]).
static VIEWS_STATE: core::sync::atomic::AtomicU8 = core::sync::atomic::AtomicU8::new(VIEWS_NONE);
/// The arena's reservation (`VIEWS_LEN + VIEW_ALIGN` bytes from here) while [`VIEWS_OPEN`].
static VIEWS_AT: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);
/// Bytes of the arena handed out, from its first [`VIEW_ALIGN`] boundary; only grows.
static VIEWS_USED: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);

/// Take the start's whole-file views (`link`, which runs before any other thread exists) from one
/// arena (DYN-START-RING): one inaccessible reservation at the first view, each view then mapped
/// over its next [`VIEW_ALIGN`]-aligned slot with one `mmap` (same alignment, so the same faults,
/// as [`map_view_alone`]'s), and the whole arena unmapped by one `munmap` ([`views_close`]). Four
/// system calls fewer per loaded object than a reservation and two trims per view and an unmap
/// each. A start with no dependency reserves nothing.
pub(super) fn views_arm() {
    let _ = VIEWS_STATE.compare_exchange(VIEWS_NONE, VIEWS_ARMED, Relaxed, Relaxed);
}

/// The next arena slot for a view of `len` bytes, reserving the arena first when armed; `None`
/// without an arena or when the view does not fit (the caller maps it on its own).
fn views_take(len: usize) -> Option<usize> {
    if VIEWS_STATE.load(Relaxed) == VIEWS_ARMED {
        let span = VIEWS_LEN.checked_add(VIEW_ALIGN)?;
        let Ok(at) = raw::mmap(0, span, 0, MAP_PRIVATE_I32 | MAP_ANONYMOUS_I32 | MAP_NORESERVE_I32, -1, 0) else {
            // No address space for it (an RLIMIT_AS): every view on its own.
            VIEWS_STATE.store(VIEWS_NONE, Relaxed);
            return None;
        };
        VIEWS_AT.store(at, Relaxed);
        VIEWS_STATE.store(VIEWS_OPEN, Relaxed);
    }
    if VIEWS_STATE.load(Relaxed) != VIEWS_OPEN {
        return None;
    }
    // A page-aligned user address plus a length the kernel accepted: no wrap.
    let first = VIEWS_AT.load(Relaxed).wrapping_add(VIEW_ALIGN - 1) & !(VIEW_ALIGN - 1);
    let from = VIEWS_USED.load(Relaxed);
    let end = from.checked_add(len.checked_next_multiple_of(VIEW_ALIGN)?).filter(|&e| e <= VIEWS_LEN)?;
    VIEWS_USED.store(end, Relaxed);
    Some(first.wrapping_add(from))
}

/// Give back the view [`map_view`] made at `at` (`len` bytes). A view inside the arena stays until
/// [`views_close`] unmaps the arena whole: unmapping it alone would open a hole another mapping
/// could take, which the arena's own unmap would then remove.
pub(super) fn unmap_view(at: usize, len: usize) {
    let base = VIEWS_AT.load(Relaxed);
    let inside = VIEWS_STATE.load(Relaxed) == VIEWS_OPEN && at >= base && at.wrapping_sub(base) < VIEWS_LEN.wrapping_add(VIEW_ALIGN);
    if !inside {
        let _ = raw::munmap(at, len);
    }
}

/// End the start's view arena: unmap it with every view in it, after the last use and the last
/// [`unmap_view`] of any of them. Later views (`dlopen`) are mapped on their own, and may reuse
/// the range.
pub(super) fn views_close() {
    if VIEWS_STATE.swap(VIEWS_NONE, Relaxed) == VIEWS_OPEN {
        let _ = raw::munmap(VIEWS_AT.swap(0, Relaxed), VIEWS_LEN.wrapping_add(VIEW_ALIGN));
    }
}

/// Open the NUL-terminated `path` and map the whole file read-only: the file and its descriptor
/// (list mode; [`open_file`] is the run path's opener, which also gives the file's identity).
pub(super) fn open_whole(path: &[u8]) -> Option<(&'static [u8], i64)> {
    let fd = crate::ffi_sys::niji_sys_openat(AT_FDCWD, path.as_ptr().cast(), i64::from(O_RDONLY | O_CLOEXEC), 0);
    if fd < 0 {
        return None;
    }
    let len = usize::try_from(crate::ffi_sys::niji_sys_lseek(fd, 0, i64::from(SEEK_END))).ok().filter(|&n| n > 0);
    let at = len.and_then(|len| map_view(i32::try_from(fd).unwrap_or(-1), len).ok());
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
    let (f, fd) = if search::prefixed(search::CACHE_FILE, &mut path) { path.c_str().and_then(open_whole) } else { None }?;
    let _ = crate::ffi_sys::niji_sys_close(fd);
    let parsed = Cache::parse(f);
    if parsed.is_none() {
        unmap_view(f.as_ptr().expose_provenance(), f.len());
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
/// probed rule [`Refusal::skipped_at_start`]. Pages of `page` bytes. An object asking for an
/// executable stack passes only when `env`'s tunable accepts it (COMPAT-19).
///
/// `checked` receives what the check read ([`Checked`], D-27.71) for an accepted candidate; it is
/// `None` after a refusal and for an object whose executable-stack request the tunable accepted
/// (its plan refused that request, so the caller maps it through the accepting path, which asks
/// the tunable again).
pub(super) fn open_candidate(path: &[u8], page: u64, env: &List<'_>, checked: &mut Option<Checked<'static>>) -> Result<(i32, usize, &'static [u8], FileId), Unusable> {
    *checked = None;
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
    let size = file_size(fd, &st);
    let Some(len) = usize::try_from(size).ok().filter(|&n| n >= niji_ld::elf::EHDR_SIZE) else { return Err(refuse(Refusal::TooShort)) };
    let Ok(at) = map_view(fd32, len) else { return Err(refuse(Refusal::Truncated)) };
    // UNSAFE(slice): a candidate file, mapped read-only above.
    #[allow(unsafe_code)]
    // SAFETY: `len` bytes from `at` are a private read-only mapping of the file, unmapped here when the
    // file is not used, else only after the caller's last use of the slice (the end of `link`; the
    // list mode keeps it until the process exits).
    let file = unsafe { conv::bytes_at(at, len) };
    let verdict = match niji_ld::dlopen::check_into::<Native>(file, page, checked) {
        Err(Refusal::ExecutableStack) => exec_stack_candidate(file, page, env),
        v => v,
    };
    if let Err(r) = verdict {
        unmap_view(at, len);
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]
mod tests {
    use super::{map_view_hidden, raw, unmap_view, views_hidden};
    use std::os::unix::io::AsRawFd;

    /// The permissions column of `/proc/self/maps` for the mapping that starts at `at`.
    fn perms_at(at: usize) -> String {
        let maps = std::fs::read_to_string("/proc/self/maps").unwrap();
        let start = format!("{at:x}-");
        let line = maps.lines().find(|l| l.starts_with(&start)).unwrap_or_else(|| panic!("no mapping at {at:#x}"));
        line.split_whitespace().nth(1).unwrap().to_owned()
    }

    /// VALGRIND-LOADER-VIEW (ruling #7301); no spec clause (a tool's view of the loader). The view
    /// valgrind must not record is inaccessible when it is mapped (a readable file mapping made before
    /// an object's segments is taken for one of the object's own) and readable once `reveal` has run;
    /// a failed `reveal` leaves nothing mapped. Run on the bare machine, where `map_view` itself never
    /// takes this path: the client request answers 0 there.
    #[test]
    fn hidden_view_is_inaccessible_until_revealed_and_the_bare_machine_does_not_hide() {
        assert!(!views_hidden(), "the bare machine answered RUNNING_ON_VALGRIND with a nonzero depth");
        let exe = std::fs::File::open(std::env::current_exe().unwrap()).unwrap();
        let len = usize::try_from(exe.metadata().unwrap().len()).unwrap();
        let mut seen = None;
        let at = map_view_hidden(exe.as_raw_fd(), len, |at, n| {
            seen = Some(perms_at(at));
            raw::mprotect(at, n, niji_ld::map::PROT_READ).map_err(|_| ())
        })
        .expect("the hidden view maps");
        assert_eq!(seen.as_deref(), Some("---p"), "the view was readable when it was mapped: valgrind records it as a mapping of the object");
        assert_eq!(perms_at(at), "r--p", "the revealed view is not readable");
        unmap_view(at, len);
        let mut failed_at = 0;
        let r = map_view_hidden(exe.as_raw_fd(), len, |at, _| {
            failed_at = at;
            Err(())
        });
        assert!(r.is_err(), "a failed reveal was not reported");
        let maps = std::fs::read_to_string("/proc/self/maps").unwrap();
        assert!(!maps.lines().any(|l| l.starts_with(&format!("{failed_at:x}-"))), "a failed reveal left the view mapped");
    }
}
