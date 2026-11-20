//! `ld.so`'s start (phase 14 DL-5, plan/ph14/design.md §1 and §19, R14-2, D-27.13). Feature `ld`,
//! used only by the `crates/ld-so` product.
//!
//! `niji_machine_ld_entry` (`machine/entry.S`, `-DNIJI_LD`) calls [`niji_ld_start`] with the
//! initial stack. The steps, in order: (1) relocate the interpreter's own image with the DL-4
//! relocator ([`crate::bootstrap::relocate_image`]); (2) install the provisional thread block
//! (L14-2); (3) read argc/argv and the auxiliary vector; (4) as a program, answer `--version` and
//! `--help` ([`niji_ld::program`]); everything else, and a start by the kernel for a program's
//! `PT_INTERP`, ends with status 127 until DL-6 loads programs.
//!
//! Before step 1 returns nothing here may read a pointer stored in the image's data (design §1):
//! every reference is PC-relative (the product's version script makes every symbol local).

use core::alloc::{GlobalAlloc, Layout};
#[cfg(not(feature = "export"))]
use core::ffi::{c_int, c_void};
use core::sync::atomic::{AtomicBool, AtomicUsize};
use core::sync::atomic::Ordering::{Relaxed, Release};
use niji::config::Tunables;
use niji_kernel::auxv::Auxv;
use niji_kernel::uapi::{AT_ENTRY, AT_EXECFN, AT_RANDOM, AT_SYSINFO_EHDR};
use niji_ld::arch::{Arch, Native};
use niji_ld::program::{self, Action};
use niji_ld::dlopen::Refusal;
use niji_ld::search::{self, CacheRef, Ctx, PathBuf, Scope};
use niji_ld::{boot, elf, map, start};
use niji_ld::tls as ltls;
use niji_legacy::search_c::CWords;
use crate::conv;
use table::{PathStore, Table, INLINE};
use self::io::{fail_loading, lo32, refuse};
use self::state::{AUXV_WORDS_MAX, OWN_BASE, PHDR_WORDS, START_ARGS, START_INTERP, START_N, START_PAGE, start_obj};
use self::tls::{INITIAL_TAIL, NewTls, PROVISIONAL, THREAD_MODS_LEN, initial_place, install_tls, map_area, module_room, program_tls, publish_tls, release_tls, surplus, thread_mod, tls_generation};
use self::image::{AT_FDCWD, MAP_ANONYMOUS_I32, MAP_PRIVATE_I32, ProcImage, SysMapper, Unusable, dir_of, dyn_string, dynamic_of, image_view, load, map_view, open_candidate, open_whole, own_object, read_cache, read_image};
use self::initfini::{array_words, fn_addr0};
use self::link::{Origin, Preloads, link};

pub(crate) use self::io::{c_string, exit, stack_word, write_all};
pub use self::tls::{__tls_get_addr, _dl_get_tls_static_info};

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
fn report_nonshared(elf: &niji_ld::elf::Elf<'_>, name: &[u8]) {
    if REPORT_NONSHARED.load(Relaxed) && start::uses_nonshared_stubs(elf) {
        start::nonshared_report(name).iter().for_each(|p| write_all(2, p));
    }
}

/// Register the initial thread's rseq area at `tp + RSEQ_OFFSET` (inside the area [`map_area`]
/// mapped, zero-filled) and publish `__rseq_offset`/`__rseq_size`: before relocation, so a
/// program's copy of them reads the final values. The registered size (0: none).
fn register_rseq(tp: usize) -> usize {
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
fn read_auxv(auxv: usize, copy: &mut [u64; AUXV_WORDS_MAX]) -> Option<usize> {
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
fn env_entries(envp: usize) -> impl Iterator<Item = &'static [u8]> {
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
    match crate::bootstrap::relocate_image(ehdr) {
        // Its dynamic array as run-time addresses (DL-DYNAMIC-RELOC); its copies are read back
        // through `image_view`, which undoes this.
        Ok(r) if crate::bootstrap::publish_dynamic(&r).is_ok() => {}
        _ => refuse(b"ld.so: cannot relocate the interpreter\n"),
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
            list(&list_env, if execfn == 0 { b"" } else { c_string(execfn) }, false, None)
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
    // Run as a program: the path it was run by names the interpreter in a list.
    let invoked = aux.get(AT_EXECFN).and_then(|a| usize::try_from(a).ok()).filter(|&a| a != 0).map(c_string);
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
                list(&list_env, name, false, invoked)
            }
            // argv[1] names the program; it becomes argv[0] and the stack starts one word later.
            let (prog, bias) = load(arg(1), page);
            let mut copy = copy;
            start::rewrite(copy.get_mut(..words).unwrap_or(&mut []), &prog, ehdr);
            store_words(auxv, copy.get(..words.wrapping_add(2)).unwrap_or(&[]));
            store_words(argv, &[u64::try_from(argc.wrapping_sub(1)).unwrap_or(0)]);
            run(argv, &prog, bias, page, arg(1), &list_env, Auxv::from_words(&[]))
        }
        Action::List(name) => list(&list_env, name, true, invoked),
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
        let file = if search::prefixed(search::PRELOAD_FILE, &mut file_path) { file_path.c_str().and_then(open_whole) } else { None };
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
        // Its dynamic array as run-time addresses once relocated (DL-DYNAMIC-RELOC), as `link`
        // does for a program with objects.
        match crate::bootstrap::relocate_self(prog.phdr, prog.phnum, ehdr) {
            Ok(r) if crate::bootstrap::publish_dynamic(&r).is_ok() => {}
            _ => refuse(b"ld.so: cannot relocate the program (only RELATIVE relocations in this build)\n"),
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

/// `_r_debug` (spec s27_4_c_0): the debugger rendezvous `DT_DEBUG` points at ([`debug`]).
// UNSAFE(export): an exported data object of the ld.so product.
#[unsafe(no_mangle)]
#[allow(unsafe_code)]
#[allow(non_upper_case_globals)]
pub static _r_debug: debug::RDebug = debug::RDebug::new();

/// `_dl_debug_state` (spec s27_4_c_0): `r_brk`, called after each `r_state` change ([`debug`]); it
/// does nothing, and is never inlined, so a debugger's breakpoint on it stops at every change.
// UNSAFE(export): an exported function of the ld.so product.
#[unsafe(no_mangle)]
#[allow(unsafe_code)]
#[inline(never)]
pub extern "C" fn _dl_debug_state() {
    core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
}

/// What the dependency list needs from the start (DL-8).
struct List<'a> {
    ld: &'a start::LdEnv<'static>,
    secure: bool,
    /// The `LD_PRELOAD` value program start uses (in secure mode the one read before the clear).
    preload: Option<&'static [u8]>,
    features: &'a niji_rt::cpu::Features,
    page: u64,
    vdso: Option<u64>,
    base: usize,
}

impl List<'_> {
    /// The search context of program start and of the dependency list, with `cache`.
    fn ctx<'c>(&'c self, cache: CacheRef<'c>) -> Ctx<'c> {
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

/// One object of the dependency list: the name it was needed as, its `DT_SONAME`, its file (empty
/// when not found), its resolved path (a span of the path store) and the object that needed it.
#[derive(Clone, Copy, Default)]
struct Obj {
    name: &'static [u8],
    soname: &'static [u8],
    file: &'static [u8],
    /// The path it was found by (the program's: as given; none: not found, or the interpreter).
    path: &'static [u8],
    parent: usize,
    bias: u64,
    /// The interpreter's place in a list (it is never mapped again).
    interp: bool,
}

/// `v` as 16 lower-case hexadecimal digits.
fn hex16(v: u64) -> [u8; 16] {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digit = |n: u8| HEX.get(usize::from(n & 0xf)).copied().unwrap_or(b'0');
    let mut out = [b'0'; 16];
    for (pair, byte) in out.chunks_exact_mut(2).zip(v.to_be_bytes()) {
        if let [hi, lo] = pair {
            *hi = digit(byte.wrapping_shr(4));
            *lo = digit(byte);
        }
    }
    out
}

/// `ld.so --list PROGRAM` and `LD_TRACE_LOADED_OBJECTS` (ld.so(8), design §19, DL-8): find and map
/// (through [`map::load`]) every object the program needs, breadth first in `DT_NEEDED` order, each
/// once, and write one line per object as `ldd` shows them; nothing is relocated or run. The lines
/// are written once the walk is complete. A dependency that is not found is a `NAME => not found`
/// line (status 0) when tracing; with `strict` (`--list`) it is the loader's error on stderr, nothing
/// on stdout, status 127, as the reference's `ld.so --list` does (DL-8 probe). The interpreter's
/// line is listed only when an object names the interpreter in `DT_NEEDED` (libniji's own
/// libraries name it last), at its breadth-first place; a program with no `DT_NEEDED` is
/// `statically linked`, status 0 (DL-TRACE-INTERP probe, spec/behaviour/dynlink.md). `invoked` is
/// the loader's own path when it was run as a program, `None` when the kernel started it.
fn list(env: &List<'_>, prog: &'static [u8], strict: bool, invoked: Option<&'static [u8]>) -> ! {
    let mut path = PathBuf::default();
    let fail = |what: &[u8], msg: &[u8]| -> ! {
        for part in [&b"ld.so: "[..], what, b": ", msg] {
            write_all(2, part);
        }
        exit(program::FAILURE)
    };
    let Some((file, fd)) = (if path.push(prog) { path.c_str().and_then(open_whole) } else { None }) else {
        fail(prog, b"cannot open shared object file\n")
    };
    let _ = crate::ffi_sys::niji_sys_close(fd);
    let interp = elf::Elf::parse::<Native>(file).ok().and_then(|e| e.interp().ok().flatten());
    // The interpreter's line: the program's PT_INTERP, else the path the loader was run by; the
    // path is the loader as run (`NAME => PATH` when the two differ), as the reference prints it.
    let interp_path = invoked.or(interp).unwrap_or(Native::INTERP.as_bytes());
    let interp_name = interp.unwrap_or(interp_path);
    let Some(top) = dynamic_of(file) else {
        write_all(1, b"\tnot a dynamic executable\n");
        exit(1)
    };
    if top.needed().flatten().next().is_none() {
        // No DT_NEEDED (a static PIE, a library that needs nothing): as the reference, status 0.
        write_all(1, b"\tstatically linked\n");
        exit(0)
    }
    // The cache, once (R14-3 (a): `<prefix>/etc/ld.so.cache`); unreadable or inconsistent = none.
    let cache = read_cache();
    let ctx = env.ctx(cache.as_ref().map(|(_, c)| c).into());
    let mut objs: Table<Obj, INLINE> = Table::new(Obj::default());
    let mut store = PathStore::default();
    objs.reserve(1).unwrap_or_else(|_| fail(prog, b"cannot allocate memory\n"));
    if let Some(o) = objs.first_mut() {
        *o = Obj { name: prog, file, path: prog, ..Obj::default() };
    }
    let mut count = 1usize;
    let mut i = 0usize;
    while i < count {
        let me = i;
        i = i.wrapping_add(1);
        let Some(d) = objs.get(me).and_then(|o| dynamic_of(o.file)) else { continue };
        // The requester chain: this object, then each object that loaded it, up to the program.
        let mut chain: Table<Scope<'_>, INLINE> = Table::new(Scope::default());
        let mut depth = 0usize;
        let mut k = me;
        while let Some(o) = objs.get(k) {
            chain.reserve(depth.wrapping_add(1)).unwrap_or_else(|_| fail(o.name, b"cannot allocate memory\n"));
            let Some(slot) = chain.get_mut(depth) else { break };
            let od = dynamic_of(o.file);
            let origin = Some(dir_of(o.path));
            *slot = Scope { rpath: od.as_ref().and_then(|x| dyn_string(x, elf::dt::RPATH)), runpath: od.as_ref().and_then(|x| dyn_string(x, elf::dt::RUNPATH)), origin };
            depth = depth.wrapping_add(1);
            if k == 0 {
                break;
            }
            k = o.parent;
        }
        let chain = chain.get(..depth).unwrap_or(&[]);
        // libniji's own libraries run only under this loader: each names it after its own
        // DT_NEEDED entries, where the reference's libc and libm name theirs (DL-TRACE-INTERP probe).
        let own = me != 0 && objs.get(me).is_some_and(|o| !o.file.is_empty() && niji_ld::search::pinned(o.name));
        for name in d.needed().flatten().chain(core::iter::once(Native::INTERP.as_bytes()).filter(|_| own)) {
            objs.reserve(count.wrapping_add(1)).unwrap_or_else(|_| fail(name, b"cannot allocate memory\n"));
            if name == Native::INTERP.as_bytes() || name == interp_name || name == interp_path {
                // The interpreter: listed once, at its breadth-first place, only when an object needs it.
                if !objs.get(..count).unwrap_or(&[]).iter().any(|o| o.interp) {
                    if let Some(o) = objs.get_mut(count) {
                        *o = Obj { name, interp: true, parent: me, ..Obj::default() };
                    }
                    count = count.wrapping_add(1);
                }
                continue;
            }
            let known = objs.get(..count).unwrap_or(&[]).iter().any(|o| o.name == name || (!o.soname.is_empty() && o.soname == name));
            if known {
                continue;
            }
            let mut hit: Option<(&'static [u8], i64)> = None;
            let mut refused: Option<Refusal> = None;
            let mut hit_path = PathBuf::default();
            search::search::<Native>(name, chain, &ctx, &mut |p: &mut PathBuf| {
                // The program-start rule: another machine's object or class is skipped, the search
                // goes on; any other unusable file ends the list as it ends a start.
                let Some(c) = p.c_str() else { return false };
                let found = match open_candidate(c, env.page) {
                    Err(Unusable::Skip) => return false,
                    Err(Unusable::Refused(r)) => {
                        refused = Some(r);
                        None
                    }
                    Ok((fd, _, f, _)) => Some((f, i64::from(fd))),
                };
                hit_path.clear();
                hit_path.push(p.as_bytes());
                hit = found;
                true
            });
            if let Some(r) = refused {
                let (text, by_path) = r.at_start();
                fail_loading(prog, if by_path { hit_path.as_bytes() } else { name }, text)
            }
            let Some((f, fd)) = hit else {
                if strict {
                    fail_loading(prog, name, niji_ld::dlopen::FILE_MISSING)
                }
                if let Some(o) = objs.get_mut(count) {
                    *o = Obj { name, parent: me, ..Obj::default() };
                }
                count = count.wrapping_add(1);
                continue;
            };
            let found = hit_path.as_bytes();
            let loaded = map::load::<Native, _>(&mut SysMapper { fd: i32::try_from(fd).unwrap_or(-1) }, f, env.page);
            let _ = crate::ffi_sys::niji_sys_close(fd);
            let loaded = loaded.unwrap_or_else(|e| fail(found, e.message()));
            let path = store.keep(found).unwrap_or_else(|_| fail(found, b"cannot allocate memory\n"));
            let soname = dynamic_of(f).and_then(|x| dyn_string(&x, elf::dt::SONAME)).unwrap_or(b"");
            if let Some(o) = objs.get_mut(count) {
                *o = Obj { name, soname, file: f, path, parent: me, bias: loaded.bias, interp: false };
            }
            count = count.wrapping_add(1);
        }
    }
    let vdso = || {
        if let Some(v) = env.vdso.filter(|&v| v != 0) {
            for part in [&b"\tlinux-vdso.so.1 (0x"[..], &hex16(v), b")\n"] {
                write_all(1, part);
            }
        }
    };
    // The vDSO's line comes first, after the interpreter's only when the interpreter is the first
    // object listed (the program's first need), as the reference orders them.
    let mut vdso_done = false;
    let base = u64::try_from(env.base).unwrap_or(0);
    for o in objs.get(1..count).unwrap_or(&[]) {
        if !o.interp && !vdso_done {
            vdso();
            vdso_done = true;
        }
        if o.interp {
            write_all(1, b"\t");
            write_all(1, interp_name);
            if interp_name != interp_path {
                write_all(1, b" => ");
                write_all(1, interp_path);
            }
            for part in [&b" (0x"[..], &hex16(base), b")\n"] {
                write_all(1, part);
            }
        } else if o.file.is_empty() {
            for part in [&b"\t"[..], o.name, b" => not found\n"] {
                write_all(1, part);
            }
        } else {
            for part in [&b"\t"[..], o.name, b" => ", o.path, b" (0x", &hex16(o.bias), b")\n"] {
                write_all(1, part);
            }
        }
    }
    if !vdso_done {
        vdso();
    }
    exit(0)
}

mod io;
mod state;
mod tls;
mod image;
mod initfini;
mod link;
pub mod debug;
mod dlopen;
mod readers;
mod table;

/// The panic handler of `ld.so` (a panic is a loader bug): a fixed message, status 127. It formats
/// nothing, so it works whatever state the image is in.
pub fn panicked(_info: &core::panic::PanicInfo<'_>) -> ! {
    write_all(2, b"ld.so: internal error\n");
    exit(program::FAILURE)
}

/// The global allocator of `ld.so` until the loader has a heap: every request fails (DL-5 code
/// does not allocate; a request reaches `handle_alloc_error`, a panic, status 127).
pub struct NoHeap;

// UNSAFE(allocator): an allocator that never hands out memory.
#[allow(unsafe_code)]
// SAFETY: `alloc` always returns null (allowed: allocation failure) and `dealloc` is only ever
// called with pointers `alloc` returned, of which there are none.
unsafe impl GlobalAlloc for NoHeap {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        core::ptr::null_mut()
    }
    // UNSAFE(allocator): nothing to free (see the impl's SAFETY).
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

// The C runtime's memory symbols that compiled Rust code calls, local to `ld.so` (its version
// script). They are the library's own twins (`posix::s04_strings`), over the same machine kernels
// libc uses. Not in a build with `export`, where libc's exports define these names.
macro_rules! ld_mem {
    ($($name:ident($($arg:ident: $ty:ty),*) -> $ret:ty;)*) => {$(
        #[cfg(not(feature = "export"))]
        // UNSAFE(export): a runtime symbol of the ld.so product (local, never exported).
        #[unsafe(no_mangle)]
        #[allow(unsafe_code)]
        #[allow(clippy::not_unsafe_ptr_arg_deref)]
        #[doc = concat!("`", stringify!($name), "` inside `ld.so` (local).")]
        pub extern "C" fn $name($($arg: $ty),*) -> $ret {
            crate::posix::s04_strings::$name($($arg),*)
        }
    )*};
}

ld_mem! {
    memcpy(d: *mut c_void, s: *const c_void, n: usize) -> *mut c_void;
    memmove(d: *mut c_void, s: *const c_void, n: usize) -> *mut c_void;
    memset(d: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;
    bcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;
}
