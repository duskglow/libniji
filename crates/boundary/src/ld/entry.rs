//! Program start (DL-5, DL-6): `niji_ld_start`, the auxiliary vector and the environment, the
//! secure-mode clear, rseq registration, the libc_nonshared report, and `run`, the start-up
//! sequence up to entering the program.

use core::sync::atomic::{AtomicBool, AtomicUsize};
use core::sync::atomic::Ordering::{Relaxed, Release};
use niji::config::Tunables;
use niji_kernel::auxv::Auxv;
use niji_kernel::uapi::{AT_ENTRY, AT_EXECFN, AT_RANDOM, AT_SYSINFO_EHDR};
use niji_ld::arch::{Arch, Native};
use niji_ld::program::{self, Action};
use niji_ld::search::{self, CacheRef, Ctx, PathBuf};
use niji_ld::{boot, map, start};
use niji_ld::tls as ltls;
use niji_legacy::search_c::CWords;
use crate::conv;
use super::image::{load, open_whole, read_image};
use super::io::{c_string, exit, lo32, refuse, stack_word, write_all};
use super::link::{Origin, Preloads, link};
use super::state::{AUXV_WORDS_MAX, OWN_BASE, PHDR_WORDS};
use super::tls::{PROVISIONAL, install_tls, map_area, program_tls};
use super::trace::list;

/// `__rseq_offset` (§22.9, `GLIBC_2.35` of `ld.so`): the rseq area's offset from the thread pointer
/// (`niji_rt::tls::RSEQ_OFFSET`, the same in every thread), published with [`__rseq_size`].
// UNSAFE(export): an exported data object of the ld.so product.
#[unsafe(no_mangle)]
#[allow(unsafe_code)]
#[allow(non_upper_case_globals)]
pub static __rseq_offset: core::sync::atomic::AtomicIsize = core::sync::atomic::AtomicIsize::new(0);

/// `__rseq_size` (§22.9): the size of the area the loader registered for the initial thread, 0 when
/// registration is off (`glibc.pthread.rseq=0`) or the kernel refused it.
// UNSAFE(export): an exported data object of the ld.so product.
#[unsafe(no_mangle)]
#[allow(unsafe_code)]
#[allow(non_upper_case_globals)]
pub static __rseq_size: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

/// `__rseq_flags` (§22.9): always 0.
// UNSAFE(export): an exported data object of the ld.so product.
#[unsafe(no_mangle)]
#[allow(unsafe_code)]
#[allow(non_upper_case_globals)]
pub static __rseq_flags: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

/// Whether the tunables leave rseq registration on (`glibc.pthread.rseq`, D-22.7), read at start.
static RSEQ_WANTED: AtomicBool = AtomicBool::new(true);

/// `NIJI_LD_REPORT_NONSHARED` was set outside secure mode (D-27.29, ruling #4861): each object
/// loaded at start or by `dlopen` that carries the libc_nonshared.a notice is named on standard error.
static REPORT_NONSHARED: AtomicBool = AtomicBool::new(false);

/// Name `name` on standard error when the report is on and `elf` carries the libc_nonshared.a notice.
pub(super) fn report_nonshared(elf: &niji_ld::elf::Elf<'_>, name: &[u8]) {
    if REPORT_NONSHARED.load(Relaxed) && start::uses_nonshared_stubs(elf) {
        start::nonshared_report(name).iter().for_each(|p| write_all(2, p));
    }
}

/// Register the initial thread's rseq area at `tp + RSEQ_OFFSET` (inside the area [`map_area`]
/// mapped, zero-filled) and publish `__rseq_offset`/`__rseq_size`: before relocation, so a
/// program's copy of them reads the final values. The registered size (0: none).
pub(super) fn register_rseq(tp: usize) -> usize {
    if !RSEQ_WANTED.load(Relaxed) {
        return 0;
    }
    let at = tp.wrapping_add(niji_rt::tls::RSEQ_OFFSET);
    let size = niji_rt::thread::RSEQ_SIZE;
    let ok = crate::ffi_sys::niji_sys_rseq(core::ptr::with_exposed_provenance_mut(at), i64::from(size), 0, i64::from(niji_kernel::uapi::RSEQ_SIG)) == 0;
    if !ok {
        return 0;
    }
    __rseq_offset.store(isize::try_from(niji_rt::tls::RSEQ_OFFSET).unwrap_or(0), Release);
    __rseq_size.store(size, Release);
    usize::try_from(size).unwrap_or(0)
}

/// Copy the auxiliary vector at `auxv` (key/value pairs up to `AT_NULL`) into `copy`; the number of
/// words before the `AT_NULL` pair, or `None` when it does not fit with its `AT_NULL` pair (a
/// malformed vector: the kernel passes about 30 pairs).
pub(super) fn read_auxv(auxv: usize, copy: &mut [u64; AUXV_WORDS_MAX]) -> Option<usize> {
    let mut words = 0usize;
    loop {
        let key = stack_word(auxv.wrapping_add(words.wrapping_mul(8)));
        if key == 0 {
            return Some(words);
        }
        let value = stack_word(auxv.wrapping_add(words.wrapping_add(1).wrapping_mul(8)));
        let pair = copy.get_mut(words..words.wrapping_add(2)).filter(|_| words.wrapping_add(4) <= AUXV_WORDS_MAX)?;
        pair.copy_from_slice(&[key, value]);
        words = words.wrapping_add(2);
    }
}

/// Write `words` to the initial stack at `at`, one word each.
fn store_words(at: usize, words: &[u64]) {
    // UNSAFE(slice): the initial stack's words (writable).
    #[allow(unsafe_code)]
    // SAFETY: callers write inside the kernel's argv/envp/auxv area: back over the slots they read
    // (or, after secure-mode clearing, a shorter span starting at the same environment array).
    let mut mem = unsafe { conv::ctext() };
    for (i, &w) in words.iter().enumerate() {
        mem.set_word(at, i, usize::try_from(w).unwrap_or(0));
    }
}

/// Secure mode: remove every `rt::start::UNSECURE` entry (all occurrences) from the environment
/// array at `envp`, in place, keeping the order of the rest; the number of entries kept.
fn clear_unsecure(envp: usize) -> usize {
    // UNSAFE(slice): the initial environment array and its strings (writable, on the stack).
    #[allow(unsafe_code)]
    // SAFETY: `envp` is the kernel's NULL-terminated array of NUL-terminated strings.
    let mut mem = unsafe { conv::ctext() };
    let mut kept = 0usize;
    for i in 0.. {
        let p = mem.word(envp, i);
        if p == 0 {
            break;
        }
        if !niji_rt::start::is_unsecure(mem.text(p)) {
            mem.set_word(envp, kept, p);
            kept = kept.wrapping_add(1);
        }
    }
    mem.set_word(envp, kept, 0);
    kept
}

/// The environment entries at `envp` (each without its NUL, at most [`ARG_MAX_READ`] bytes).
pub(super) fn env_entries(envp: usize) -> impl Iterator<Item = &'static [u8]> {
    (0usize..).map(move |i| stack_word(envp.wrapping_add(i.wrapping_mul(8)))).take_while(|&p| p != 0).map(|p| c_string(usize::try_from(p).unwrap_or(0)))
}

/// The entry of `ld.so` (called by `niji_machine_ld_entry` with the initial stack pointer
/// `sp` and the run-time address `ehdr` of the image's own ELF header, which the entry takes
/// PC-relatively: no call and no GOT slot is needed to find the image before it is relocated).
// UNSAFE(export): the Rust side of the ld entry primitive; local to `ld.so` (version script).
#[unsafe(no_mangle)]
#[allow(unsafe_code)]
pub extern "C" fn niji_ld_start(sp: usize, ehdr: usize) -> ! {
    // 1. The interpreter's own relocations; nothing before this reads a stored address.
    if crate::bootstrap::relocate_image(ehdr).is_err() {
        refuse(b"ld.so: cannot relocate the interpreter\n");
    }
    // 2. The provisional thread block (L14-2).
    let tb = core::ptr::from_ref(&PROVISIONAL).expose_provenance();
    if let Some(first) = PROVISIONAL.0.first() {
        first.store(tb, Relaxed);
    }
    if crate::arch::set_thread_pointer(tb).is_err() {
        refuse(b"ld.so: cannot set the thread pointer\n");
    }
    // 3. argc, argv, envp, auxv (a copy: the vector moves when the environment shrinks).
    let argc = usize::try_from(stack_word(sp)).unwrap_or(0);
    let argv = sp.wrapping_add(8);
    let envp = argv.wrapping_add(argc.wrapping_add(1).wrapping_mul(8));
    let mut envc = 0usize;
    while stack_word(envp.wrapping_add(envc.wrapping_mul(8))) != 0 {
        envc = envc.wrapping_add(1);
    }
    let mut auxv = envp.wrapping_add(envc.wrapping_add(1).wrapping_mul(8));
    let mut copy = [0u64; AUXV_WORDS_MAX];
    let Some(words) = read_auxv(auxv, &mut copy) else { refuse(b"ld.so: the auxiliary vector is too long\n") };
    OWN_BASE.store(ehdr, Relaxed);
    // The canary and the pointer guard from AT_RANDOM (spec §1.1), kept in the provisional block's
    // slots; the final thread block takes them from there (DL-12).
    if let Some(r) = Auxv::from_words(copy.get(..words).unwrap_or(&[])).get(AT_RANDOM).and_then(|a| usize::try_from(a).ok()).filter(|&a| a != 0) {
        // UNSAFE(slice): the 16 AT_RANDOM bytes.
        #[allow(unsafe_code)]
        // SAFETY: the kernel put 16 random bytes at AT_RANDOM on the initial stack (⚑ KERN, execve),
        // live for the process's lifetime.
        let bytes = unsafe { conv::bytes_at(r, 16) };
        let mut random = [0u8; 16];
        random.copy_from_slice(bytes);
        let (canary, pointer) = niji_rt::tls::guards(&random);
        for (slot, v) in [(Native::STACK_GUARD_SLOT, canary), (Native::POINTER_GUARD_SLOT, pointer)] {
            if let Some(w) = slot.and_then(|s| PROVISIONAL.0.get(usize::try_from(s / 8).unwrap_or(usize::MAX))) {
                w.store(v, Relaxed);
            }
        }
    }
    // 4. Secure mode, decided once (AT_SECURE only); the UNSECURE names leave the environment before
    // anything below reads an LD_* name (threat model rule 1). The vector then moves down to follow
    // the shorter environment, so the program finds it where the kernel's layout puts it.
    let secure = start::secure(&Auxv::from_words(copy.get(..words).unwrap_or(&[])));
    // The one LD_* name a secure process still reads (threat model rule 1's exception): its
    // LD_PRELOAD, taken before the clear and used only under `start::preload_rule`.
    let secure_preload = if secure { env_entries(envp).find_map(|e| start::value_of(e, b"LD_PRELOAD")) } else { None };
    if secure {
        let kept = clear_unsecure(envp);
        auxv = envp.wrapping_add(kept.wrapping_add(1).wrapping_mul(8));
        store_words(auxv, copy.get(..words.wrapping_add(2)).unwrap_or(&[]));
    }
    // 5. The LD_* settings, the tunables (D-1.8, the secure filter) and the CPU features: the
    // features come from the arch seam only (design §0a); the hwcaps search (DL-8) reads them.
    let ld = start::ld_env(env_entries(envp));
    let tunables = ld.tunables.map(|t| Tunables::parse(t, secure));
    RSEQ_WANTED.store(tunables.is_none_or(|t| t.rseq != Some(false)), Relaxed);
    REPORT_NONSHARED.store(ld.report_nonshared && !secure, Relaxed);
    let features = Native::features(&Auxv::from_words(copy.get(..words).unwrap_or(&[])), &crate::sys::CSys);
    // 6. The program: the kernel's (started for its PT_INTERP) or, run as a program, the one named.
    #[allow(clippy::as_conversions)] // a function's address has no conversion other than `as`
    let own_entry = (crate::arch::niji_machine_ld_entry as *const ()).expose_provenance();
    let aux = Auxv::from_words(copy.get(..words).unwrap_or(&[]));
    // The page size, from AT_PAGESZ through the arch seam only (DL-7, D-27.15).
    let page = Native::page_size(&aux);
    // LD_TRACE_LOADED_OBJECTS lists the dependencies instead of running (ldd); never in secure mode
    // (threat model rule 11).
    let preload = if secure { secure_preload } else { ld.preload };
    let list_env = List { ld: &ld, secure, preload, features: &features, page, vdso: aux.get(AT_SYSINFO_EHDR), base: ehdr };
    if aux.get(AT_ENTRY).and_then(|e| usize::try_from(e).ok()) != Some(own_entry) {
        if ld.trace && !secure {
            let execfn = aux.get(AT_EXECFN).and_then(|a| usize::try_from(a).ok()).unwrap_or(0);
            list(&list_env, if execfn == 0 { b"" } else { c_string(execfn) }, false)
        }
        let prog = start::program(&aux).unwrap_or_else(|e| refuse(e.message()));
        // The bias from PT_PHDR or, without one, the ELF header in the page holding the program
        // headers (ruling #4371, D-27.18).
        let page_bytes = usize::try_from(page).unwrap_or(usize::MAX);
        let header = |at: usize| {
            let mut out = [0u8; 64];
            read_image(at, &mut out);
            out
        };
        let bias = start::bias(program_headers(&prog), prog.phnum, prog.phdr, page_bytes, header)
            .unwrap_or_else(|| refuse(b"ld.so: cannot find the program's load bias (no PT_PHDR, and no ELF header before its program headers)\n"));
        run(sp, &prog, bias, page, 0, &list_env, aux)
    }
    let arg = |i: usize| usize::try_from(stack_word(argv.wrapping_add(i.wrapping_mul(8)))).unwrap_or(0);
    match program::action((1..argc).map(|i| c_string(arg(i)))) {
        Action::Version => {
            for part in program::version::<Native>(env!("CARGO_PKG_VERSION")) {
                write_all(1, part.as_bytes());
            }
            exit(0)
        }
        Action::Help => {
            for part in program::help::<Native>() {
                write_all(1, part.as_bytes());
            }
            exit(0)
        }
        Action::Program(name) => {
            if ld.trace && !secure {
                list(&list_env, name, false)
            }
            // argv[1] names the program; it becomes argv[0] and the stack starts one word later.
            let (prog, bias) = load(arg(1), page);
            let mut copy = copy;
            start::rewrite(copy.get_mut(..words).unwrap_or(&mut []), &prog, ehdr);
            store_words(auxv, copy.get(..words.wrapping_add(2)).unwrap_or(&[]));
            store_words(argv, &[u64::try_from(argc.wrapping_sub(1)).unwrap_or(0)]);
            run(argv, &prog, bias, page, arg(1), &list_env, Auxv::from_words(&[]))
        }
        Action::List(name) => list(&list_env, name, true),
        other => {
            for part in program::refusal(other) {
                write_all(2, part);
            }
            exit(program::FAILURE)
        }
    }
}

/// The program headers of `prog` (`AT_PHDR`, `AT_PHNUM` checked by [`start::program`] or set by [`load`]).
fn program_headers(prog: &start::Program) -> &'static [u64] {
    // UNSAFE(slice): the program's headers, mapped with the program.
    #[allow(unsafe_code)]
    // SAFETY: the kernel (`AT_PHDR`/`AT_PHNUM`, ⚑ KERN) or `load` mapped `phnum` 56-byte headers
    // there, readable for the process's lifetime.
    unsafe { conv::words_at(prog.phdr, prog.phnum.saturating_mul(PHDR_WORDS)) }
}

/// Relocate the mapped program `prog` (load bias `bias`), protect its RELRO pages (pages of `page`
/// bytes) and enter it with the initial stack at `sp`. A program with dependencies or with
/// relocations other than RELATIVE/RELR is refused (DL-8 and DL-10 add them); a program without a
/// dynamic section has neither, so nothing is relocated.
fn run(sp: usize, prog: &start::Program, bias: usize, page: u64, path: usize, env: &List<'_>, aux: Auxv<'_>) -> ! {
    let headers = program_headers(prog);
    // R14-13: a program whose PT_GNU_STACK asks for an executable stack is refused (word 0 of a header
    // holds p_type in its low and p_flags in its high half).
    let types = headers.chunks_exact(PHDR_WORDS).take(prog.phnum).filter_map(|h| h.first()).map(|&w| (lo32(w), lo32(w >> 32)));
    if map::executable_stack(types) {
        refuse(b"ld.so: the program asks for an executable stack (PT_GNU_STACK with PF_X); refused\n");
    }
    let has_dynamic = headers.chunks_exact(PHDR_WORDS).take(prog.phnum).any(|h| h.first().is_some_and(|w| w & 0xffff_ffff == u64::from(niji_ld::elf::pt::DYNAMIC)));
    if has_dynamic {
        let img = boot::image(headers, prog.phnum).unwrap_or_else(|_| refuse(b"ld.so: the program's segments are not relocatable\n"));
        let dynamic = bias.wrapping_add(usize::try_from(img.dynamic.0).unwrap_or(0));
        // UNSAFE(slice): the program's dynamic array (`PT_DYNAMIC`), read only.
        #[allow(unsafe_code)]
        // SAFETY: `dynamic` is the run-time address (bias + `p_vaddr`) of `PT_DYNAMIC`'s `p_memsz` bytes
        // in the mapped program.
        let dyn_words = unsafe { conv::words_at(dynamic, usize::try_from(img.dynamic.1).unwrap_or(0) / 8) };
        // The preload file: one open per start when it is absent (the reference's one `access`).
        let mut file_path = PathBuf::default();
        let file = if search::prefixed(b"etc/ld.so.preload", &mut file_path) { file_path.c_str().and_then(open_whole) } else { None };
        let file = file.map_or(&[][..], |(f, fd)| {
            let _ = crate::ffi_sys::niji_sys_close(fd);
            f
        });
        let preloads = Preloads { env: env.preload.unwrap_or(&[]), file, file_path: file_path.as_bytes() };
        if start::needs_objects(dyn_words) || preloads.names().next().is_some() {
            // The program's name in messages: its path (`ld.so PROGRAM`), else argv[0].
            let at = if path != 0 { path } else { usize::try_from(stack_word(sp.wrapping_add(8))).unwrap_or(0) };
            // Its `$ORIGIN`: the path it was run by (`ld.so PROGRAM`), else `AT_EXECFN` (threat model
            // rule 2), looked up only when a search needs it.
            let origin = Origin { path, aux };
            let fini = link(prog, bias, page, if at != 0 { c_string(at) } else { b"the program" }, sp, env, origin, &preloads);
            crate::arch::enter(sp, prog.entry, fini)
        }
        let ehdr = bias.wrapping_add(usize::try_from(img.header).unwrap_or(0));
        if crate::bootstrap::relocate_self(prog.phdr, prog.phnum, ehdr).is_err() {
            refuse(b"ld.so: cannot relocate the program (only RELATIVE relocations in this build)\n");
        }
    }
    let page = usize::try_from(page).unwrap_or(usize::MAX);
    // Segment by segment (D-27.25): a reservation page between two writable segments stays as mapped.
    if niji_rt::start::relro_pieces(headers, prog.phnum, bias, page, |addr, len| crate::raw::mprotect(addr, len, map::PROT_READ)).is_err() {
        refuse(b"ld.so: cannot protect the program's RELRO segment\n");
    }
    // The program alone: its own TLS block (if any) and the final thread block (DL-12).
    let one = [program_tls(headers, prog.phnum)];
    let mut placed = [None];
    let area = ltls::layout::<Native>(&one, &mut placed, Native::TCB_SIZE, ltls::SURPLUS).unwrap_or_else(|_| refuse(b"ld.so: the program's TLS segment is malformed\n"));
    install_tls(&one, &[u64::try_from(bias).unwrap_or(0)], &placed, &area, map_area(&area));
    crate::arch::enter(sp, prog.entry, 0)
}

/// `__libc_stack_end` (spec §1.1, §27.4; exported by `ld.so` at `GLIBC_2.2.5`, R14-12, D-27.20):
/// the initial stack pointer, stored by [`link`] with the rseq objects before any object is
/// relocated, so a program's COPY relocation of it carries the value (D-27.26). `libc.so.6` keeps its
/// own local copy, which its start publishes from the same stack pointer (`StartInfo::stack_end`);
/// `libc.a` keeps its own definition for static programs. Unmangled only in the `ld.so` product: a
/// build with `runtime` takes the C library's definition (`adapter/runtime.c`).
// UNSAFE(export): a C data symbol needs an unmangled name; the loader defines it once.
#[cfg_attr(not(feature = "runtime"), unsafe(no_mangle))]
#[allow(unsafe_code)]
#[allow(non_upper_case_globals)]
pub static __libc_stack_end: AtomicUsize = AtomicUsize::new(0);

/// What the dependency list needs from the start (DL-8).
pub(super) struct List<'a> {
    ld: &'a start::LdEnv<'static>,
    pub(super) secure: bool,
    /// The `LD_PRELOAD` value program start uses (in secure mode the one read before the clear).
    preload: Option<&'static [u8]>,
    features: &'a niji_rt::cpu::Features,
    pub(super) page: u64,
    pub(super) vdso: Option<u64>,
    pub(super) base: usize,
}

impl List<'_> {
    /// The search context of program start and of the dependency list, with `cache`.
    pub(super) fn ctx<'c>(&'c self, cache: CacheRef<'c>) -> Ctx<'c> {
        Ctx {
            secure: self.secure,
            library_path: if self.secure { None } else { self.ld.library_path },
            origin_path: if self.secure { None } else { self.ld.origin_path },
            platform: Native::platform(self.features),
            hwcaps: Native::hwcaps(self.features),
            cache,
        }
    }
}
