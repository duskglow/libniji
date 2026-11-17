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
use niji_ld::{boot, elf, init, map, start};
use niji_ld::tls::{self as ltls, Module, Placed};
use niji_legacy::search_c::CWords;
use crate::{conv, raw};
use self::io::{fail_loading, fail_with, lo32, refuse};
use self::state::{AUXV_WORDS_MAX, OBJECTS_MAX, OWN_BASE, PHDR_WORDS, START_ARGS, START_INTERP, START_N, START_OBJS, START_PAGE};
use self::tls::{INITIAL_TAIL, NewTls, PROVISIONAL, THREAD_MODS, THREAD_MODS_LEN, initial_place, install_tls, map_area, module_room, program_tls, publish_tls, release_tls, surplus, tls_generation};
use self::image::{AT_FDCWD, FileId, MAP_ANONYMOUS_I32, MAP_PRIVATE_I32, ProcImage, SysMapper, Unusable, dir_of, dyn_string, dynamic_of, file_mode, image_view, load, open_candidate, open_file, open_whole, own_object, parse, read_cache, read_image, scratch, word};
use self::initfini::{array_words, run_init};

pub(crate) use self::io::{c_string, exit, stack_word, write_all};
pub use self::tls::__tls_get_addr;

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

/// Load the program's dependencies, check their versions, relocate everything and protect the RELRO
/// pages (DL-10, design §6, D-27.17). A `DT_NEEDED` name with `/` is opened as given; one without is
/// found by [`search::search`], the search `--list` and `dlopen` use (DL-START-SEARCH): the requester
/// chain is each object's first requester up to the program, `$ORIGIN` the directory of the path an
/// object was opened by (the program's: from `origin`, looked up at the first search), and the
/// cache is read only when a search reaches it. The program is read from its memory image
/// ([`image_view`], ruling #4229, D-27.18), never from its file; `name` names it in messages (its
/// path for `ld.so PROGRAM`, else `argv[0]`).
// Indices stay below `n`, which never passes OBJECTS_MAX (checked before each store).
#[allow(clippy::indexing_slicing, clippy::arithmetic_side_effects, clippy::too_many_arguments)]
fn link(prog: &start::Program, bias: usize, page: u64, name: &'static [u8], sp: usize, env: &List<'_>, origin: Origin<'_>, preloads: &Preloads<'_>) -> usize {
    use niji_ld::reloc::{self, Obj, Objects};
    let prog_name = name;
    let mut origin_at: Option<usize> = None;
    // The search state, used only for a name without `/`: each object's first requester and the
    // path it was opened by (empty: unknown, or the program's, from `origin`); the store for the
    // paths found (address space mapped at the first search hit, so it never runs out); the cache.
    let mut parent = [0u8; OBJECTS_MAX];
    let mut paths: [&'static [u8]; OBJECTS_MAX] = [&[]; OBJECTS_MAX];
    let mut store: &'static mut [u8] = &mut [];
    let mut store_map = (0usize, 0usize);
    let cache: search::LazyCache<'_> = core::cell::OnceCell::new();
    let mut maps = [(0usize, 0usize); OBJECTS_MAX];
    // The scratch mappings of the two image views: the program's, then the interpreter's own.
    let mut views = [(0usize, 0usize); 6];
    let (prog_views, own_views) = views.split_at_mut(3);
    let (head, view) = image_view(prog, bias, name, prog_views);
    // The program's own file is not known (no file is read): (dev, ino) that no file has.
    let mut ids: [FileId; OBJECTS_MAX] = [(u64::MAX, u64::MAX); OBJECTS_MAX];
    // Each object's run-time program header table (address, count), for `dlopen` (DL-16a).
    let mut phdrs = [(prog.phdr, prog.phnum); OBJECTS_MAX];
    let image = niji_ld::elf::Elf::from_image::<Native>(head, view).unwrap_or_else(|_| fail_with(&[name, b": malformed program headers\n"], program::FAILURE));
    let dynm = image.dynamic().ok().flatten().unwrap_or_else(|| fail_with(&[name, b": no valid dynamic section\n"], program::FAILURE));
    let first = Obj { elf: image, dynm, bias: u64::try_from(bias).unwrap_or(0), name, tls: None };
    let mut objs = [first; OBJECTS_MAX];
    let mut n = 1usize;
    let mut q = 0usize;
    // The interpreter itself, when an object names it (`__tls_get_addr`): already relocated, in the
    // scope with its exports, never relocated again (D-27.20); read from its memory image.
    let mut interp: Option<usize> = None;
    // The DT_NEEDED links (dependent, dependency) in the order each object names them (DL-11).
    let mut edges = [(0u8, 0u8); EDGES_MAX];
    let mut ne = 0usize;
    let mut edge = |from: usize, to: usize, name: &[u8]| {
        let (Ok(a), Ok(b)) = (u8::try_from(from), u8::try_from(to)) else { fail_with(&[name, b": too many objects\n"], program::FAILURE) };
        let Some(slot) = edges.get_mut(ne) else { fail_with(&[name, b": too many DT_NEEDED entries\n"], program::FAILURE) };
        *slot = (a, b);
        ne += 1;
    };
    // The preloads (DL-LD-PRELOAD) come first, as the program's (q == 0) names before its own
    // DT_NEEDED: after the program and before its dependencies in the scope, linked by no edge.
    let mut pending = preloads.names();
    while q < n {
        let needed = objs[q].dynm;
        let pre_names = core::iter::from_fn(|| if q == 0 { pending.next() } else { None }).map(|(p, from_env)| Ok((p, Some(from_env))));
        for name in pre_names.chain(needed.needed().map(|r| r.map(|p| (p, None)))) {
            let (name, pre) = name.unwrap_or_else(|_| fail_with(&[objs[q].name, b": malformed DT_NEEDED\n"], program::FAILURE));
            let rule = pre.map(|from_env| start::preload_rule(name, from_env, env.secure));
            if rule == Some(start::PreloadRule::Skip) {
                continue;
            }
            let bare = !name.contains(&b'/');
            // A name already loaded; a bare name also matches a loaded object's DT_SONAME (as the
            // dependency list and `dlopen` match it).
            if let Some(j) = objs[..n].iter().position(|o| o.name == name || (bare && dyn_string(&o.dynm, elf::dt::SONAME) == Some(name))) {
                if pre.is_none() {
                    edge(q, j, name);
                }
                continue;
            }
            let mut is_interp = name == Native::INTERP.as_bytes();
            if is_interp && pre.is_some() {
                continue;
            }
            if n == OBJECTS_MAX {
                fail_with(&[name, b": too many objects\n"], program::FAILURE);
            }
            let mut found = None;
            if (bare || pre.is_some()) && !is_interp {
                // The requester chain: this object, then each first requester up to the program.
                let mut chain = [Scope::default(); OBJECTS_MAX];
                let (mut depth, mut k) = (0usize, q);
                loop {
                    let at = if k == 0 {
                        let o = *origin_at.get_or_insert_with(|| origin.at());
                        if o == 0 { &[][..] } else { c_string(o) }
                    } else {
                        paths[k]
                    };
                    let d = &objs[k].dynm;
                    chain[depth] = Scope { rpath: dyn_string(d, elf::dt::RPATH), runpath: dyn_string(d, elf::dt::RUNPATH), origin: (!at.is_empty()).then(|| dir_of(at)) };
                    depth += 1;
                    if k == 0 || depth == OBJECTS_MAX {
                        break;
                    }
                    k = usize::from(parent[k]);
                }
                let ctx = env.ctx(CacheRef::Lazy(&cache, read_cache));
                let mut refused: Option<Refusal> = None;
                let mut at_path = PathBuf::default();
                let set_uid_only = rule == Some(start::PreloadRule::TrustedSetUid);
                let mut try_open = |p: &mut PathBuf| {
                    let Some(c) = p.c_str() else { return false };
                    match open_candidate(c, page) {
                        Err(Unusable::Skip) => return false,
                        Err(Unusable::Refused(r)) => refused = Some(r),
                        Ok((fd, at, file, _)) if set_uid_only && !start::preload_mode_admits(file_mode(fd)) => {
                            let _ = crate::ffi_sys::niji_sys_close(i64::from(fd));
                            let _ = raw::munmap(at, file.len());
                            return false;
                        }
                        Ok(hit) => found = Some(hit),
                    }
                    at_path.clear();
                    at_path.push(p.as_bytes());
                    true
                };
                if set_uid_only {
                    // Secure mode, an LD_PRELOAD name: each trusted directory itself, nothing else.
                    let mut cand = PathBuf::default();
                    search::trusted_dirs::<Native>(&mut |dir| {
                        cand.clear();
                        cand.push(dir) && cand.push(b"/") && cand.push(name) && try_open(&mut cand)
                    });
                } else {
                    search::search::<Native>(name, &chain[..depth], &ctx, &mut try_open);
                }
                if let Some(r) = refused {
                    if let Some(from_env) = pre {
                        preloads.ignored(name, from_env, r.at_preload());
                        continue;
                    }
                    let (text, by_path) = r.at_start();
                    fail_loading(prog_name, if by_path { at_path.as_bytes() } else { name }, text)
                }
                if found.is_none() {
                    if let Some(from_env) = pre {
                        preloads.ignored(name, from_env, start::PRELOAD_MISSING);
                        continue;
                    }
                    fail_loading(prog_name, name, niji_ld::dlopen::FILE_MISSING)
                }
                // Kept for its `$ORIGIN` (the store is address space for OBJECTS_MAX full paths).
                if store_map.0 == 0 {
                    let len = OBJECTS_MAX * search::PATH_MAX;
                    let (at, buf) = scratch(len, prog_name);
                    (store_map, store) = ((at, len), buf);
                }
                let p = at_path.as_bytes();
                if p.len() > store.len() {
                    fail_with(&[p, b": too many objects\n"], program::FAILURE);
                }
                let (kept, rest) = core::mem::take(&mut store).split_at_mut(p.len());
                kept.copy_from_slice(p);
                store = rest;
                paths[n] = kept;
            }
            let id = if is_interp {
                (u64::MAX, u64::MAX)
            } else {
                let (fd, at, file, id) = found.unwrap_or_else(|| {
                    // The string table entry is NUL-terminated in the mapped file.
                    open_file(name.as_ptr().expose_provenance()).unwrap_or_else(|| fail_loading(prog_name, name, niji_ld::dlopen::FILE_MISSING))
                });
                found = Some((fd, at, file, id));
                // `ld.so` by any name it was found by is the start-time `ld.so` (ruling #5126).
                is_interp = bare && dynamic_of(file).and_then(|d| dyn_string(&d, elf::dt::SONAME)) == Some(Native::INTERP.as_bytes());
                id
            };
            // Another path to an object already loaded (L14-4): the same object, loaded once.
            let again = if is_interp { interp } else { ids[..n].iter().position(|x| *x == id) };
            if is_interp || again.is_some() {
                if let Some((fd, at, file, _)) = found {
                    let _ = crate::ffi_sys::niji_sys_close(i64::from(fd));
                    let _ = raw::munmap(at, file.len());
                }
            }
            if pre.is_some() && (is_interp || again.is_some()) {
                continue;
            }
            if let Some(j) = again {
                edge(q, j, name);
                continue;
            }
            if is_interp {
                objs[n] = own_object(name, own_views);
                paths[n] = &[];
                parent[n] = u8::try_from(q).unwrap_or(0);
                edge(q, n, name);
                interp = Some(n);
                n += 1;
                continue;
            }
            let Some((fd, at, file, id)) = found else { continue };
            if !bare && pre.is_none() {
                paths[n] = name;
            }
            let loaded = map::load::<Native, _>(&mut SysMapper { fd }, file, page);
            let _ = crate::ffi_sys::niji_sys_close(i64::from(fd));
            let loaded = loaded.unwrap_or_else(|e| fail_with(&[name, b": ", e.message()], program::FAILURE));
            maps[n] = (at, file.len());
            ids[n] = id;
            phdrs[n] = (usize::try_from(loaded.phdr).unwrap_or(0), usize::from(loaded.phnum));
            objs[n] = parse(file, name, loaded.bias);
            parent[n] = u8::try_from(q).unwrap_or(0);
            if pre.is_none() {
                edge(q, n, name);
            }
            n += 1;
        }
        q += 1;
    }
    for (k, o) in objs[..n].iter().enumerate() {
        if Some(k) != interp {
            report_nonshared(&o.elf, o.name);
        }
    }
    // Static TLS of the initially loaded objects (design §7, R14-4, D-27.20).
    let mut mods: [Option<Module>; OBJECTS_MAX] = [None; OBJECTS_MAX];
    for (k, (m, o)) in mods.iter_mut().zip(&objs[..n]).enumerate() {
        if Some(k) == interp {
            continue;
        }
        *m = o.elf.find(niji_ld::elf::pt::TLS).map(|ph| Module::of(&ph));
        if m.is_some_and(|m| !ltls::image_in_load(&o.elf, &m)) {
            fail_with(&[o.name, b": the TLS image lies outside the object's segments\n"], program::FAILURE);
        }
    }
    let mut placed: [Option<Placed>; OBJECTS_MAX] = [None; OBJECTS_MAX];
    let area = ltls::layout::<Native>(&mods[..n], &mut placed[..n], INITIAL_TAIL, ltls::SURPLUS).unwrap_or_else(|e| {
        let at = match e {
            ltls::TlsError::Align(k) | ltls::TlsError::Size(k) => k,
            _ => 0,
        };
        fail_with(&[objs.get(at).map_or(&b"?"[..], |o| o.name), b": malformed or too large TLS segment\n"], program::FAILURE)
    });
    // Room in every thread's dtv for the modules `dlopen` may add (DL-16d): address space only.
    let area = u64::try_from(ltls::MODULES_MAX).ok().and_then(|s| area.with_dtv_slots(s)).unwrap_or_else(|| fail_with(&[b"ld.so: the TLS area is too large\n"], program::FAILURE));
    // The objects ld.so exports (R14-12, D-27.26): published before any object is relocated, so
    // a program's copy relocation of one reads its final value.
    let area_base = map_area(&area);
    __libc_stack_end.store(sp, Release);
    let rseq_size = register_rseq(area_base.wrapping_add(usize::try_from(area.tp).unwrap_or(0)));
    for (o, p) in objs[..n].iter_mut().zip(&placed[..n]) {
        o.tls = *p;
    }
    let objs = &objs[..n];
    if let Err(e) = reloc::check_versions(objs) {
        let by = |i: usize| objs.get(i).map_or(&b"?"[..], |o| o.name);
        match e {
            reloc::VersionError::NoVersions { by: b, file, version } => fail_with(&[file, b": version `", version, b"' not found (the object has no version information; required by ", by(b), b")\n"], program::FAILURE),
            reloc::VersionError::Missing { by: b, file, version } => fail_with(&[file, b": version `", version, b"' not found (required by ", by(b), b")\n"], 1),
            reloc::VersionError::Elf(b, _) => fail_with(&[by(b), b": malformed version tables\n"], program::FAILURE),
        }
    }
    let first_sym = niji_ld::symbols::Object::new(objs[0].dynm).unwrap_or_else(|_| fail_with(&[objs[0].name, b": malformed symbol tables\n"], program::FAILURE));
    let mut syms = [first_sym; OBJECTS_MAX];
    for (s, o) in syms.iter_mut().zip(objs).skip(1) {
        *s = niji_ld::symbols::Object::new(o.dynm).unwrap_or_else(|_| fail_with(&[o.name, b": malformed symbol tables\n"], program::FAILURE));
    }
    // The C library's private interface (NIJI_PRIVATE, L14-10, D-27.22): a library from another
    // build is refused before any of its code runs or any relocation is applied.
    let early = start::private::find(objs, &syms[..n], interp, |at, out| read_image(usize::try_from(at).unwrap_or(0), out)).unwrap_or_else(|e| {
        let (start::private::PrivateError::Mismatch(k) | start::private::PrivateError::Malformed(k)) = e;
        fail_with(&[objs.get(k).map_or(&b"?"[..], |o| o.name), e.message()], program::FAILURE)
    });
    let global: [usize; OBJECTS_MAX] = core::array::from_fn(|i| i);
    let set = Objects { objs, syms: &syms[..n], global: &global[..n] };
    // The start's lookups all search the global scope, whose order decides each unique name
    // (`ScopeOrder`): no table to fill, so no count of unique names can make the start fail.
    let mut unique = niji_ld::symbols::ScopeOrder;
    // Reverse load order (L14-8), the interpreter left out: it relocated itself at its start.
    let relocated = (0..n).rev().filter(|&k| Some(k) != interp).try_for_each(|k| reloc::relocate::<Native, _>(&set, k, &mut ProcImage, page, &mut unique).map_err(|error| reloc::Failure { obj: k, error }));
    if let Err(f) = relocated {
        let name = objs.get(f.obj).map_or(&b"?"[..], |o| o.name);
        match f.error {
            reloc::RelocError::Undefined(sym) => fail_with(&[name, b": ", f.error.message(), sym, b"\n"], program::FAILURE),
            e => fail_with(&[name, b": ", e.message()], program::FAILURE),
        }
    }
    if let Err(i) = reloc::protect_relro(objs, &mut ProcImage, page) {
        fail_with(&[objs.get(i).map_or(&b"?"[..], |o| o.name), b": cannot protect the RELRO segment\n"], program::FAILURE);
    }
    // The images are copied after relocation: a `.tdata` word may hold a relocated address.
    let biases: [u64; OBJECTS_MAX] = core::array::from_fn(|k| objs.get(k).map_or(0, |o| o.bias));
    install_tls(&mods[..n], &biases[..n], &placed[..n], &area, area_base);
    // The interpreter, when in the list, ran its own start already: no hooks of its own here.
    let hooks: [init::Hooks; OBJECTS_MAX] = core::array::from_fn(|i| objs.get(i).filter(|_| Some(i) != interp).map_or_else(init::Hooks::default, |o| init::hooks(&o.dynm, o.bias, i == 0)));
    // The cache, when a search read it, and the path store: no longer needed.
    let cache_map = cache.get().and_then(Option::as_ref).map_or((0, 0), |(f, _)| (f.as_ptr().expose_provenance(), f.len()));
    let preload_map = (preloads.file.as_ptr().expose_provenance(), preloads.file.len());
    for &(at, len) in maps[1..n].iter().chain(&views).chain([&store_map, &cache_map, &preload_map]).filter(|m| m.1 != 0) {
        let _ = raw::munmap(at, len);
    }
    for (k, (slot, o)) in START_OBJS.iter().zip(objs).enumerate() {
        let (phdr, phnum) = phdrs[k];
        let words = [u64::try_from(phdr).unwrap_or(0), u64::try_from(phnum).unwrap_or(0), o.bias, ids[k].0, ids[k].1];
        slot.iter().zip(words).for_each(|(w, v)| w.store(v, Relaxed));
    }
    START_INTERP.store(interp.unwrap_or(usize::MAX), Relaxed);
    START_PAGE.store(page, Relaxed);
    START_N.store(n, Release);
    run_init(&hooks[..n], &edges[..ne], sp, early.map(|f| f.early_init), rseq_size)
}

/// Most `DT_NEEDED` links over all objects of one start.
const EDGES_MAX: usize = 1024;
/// Objects a dependency list (`--list`) holds (the program included); more is refused (D-27.16).
const LIST_OBJECTS_MAX: usize = 256;
/// Bytes of resolved paths a dependency list holds; more is refused (D-27.16).
const PATHS_MAX: usize = 64 * 1024;

/// Where a kernel-started program's `$ORIGIN` comes from: the path it was run by (`ld.so PROGRAM`,
/// nonzero), else `AT_EXECFN` of `aux`, read only when a search needs it (a program whose
/// dependencies are all named by path never looks it up).
#[derive(Clone, Copy)]
struct Origin<'a> {
    path: usize,
    aux: Auxv<'a>,
}

impl Origin<'_> {
    /// The NUL-terminated path, or 0 when unknown.
    fn at(self) -> usize {
        if self.path != 0 { self.path } else { self.aux.get(AT_EXECFN).and_then(|a| usize::try_from(a).ok()).unwrap_or(0) }
    }
}

/// The objects program start preloads (DL-LD-PRELOAD): the `LD_PRELOAD` names, then the preload
/// file's (ld.so(8) order; probed), each with whether it came from `LD_PRELOAD`.
struct Preloads<'a> {
    env: &'static [u8],
    file: &'static [u8],
    file_path: &'a [u8],
}

impl Preloads<'_> {
    fn names(&self) -> impl Iterator<Item = (&'static [u8], bool)> {
        start::preload_names(self.env).map(|n| (n, true)).chain(start::preload_file_names(self.file).map(|n| (n, false)))
    }

    /// The "cannot be preloaded" diagnostic for `name` (from `LD_PRELOAD` when `from_env`).
    fn ignored(&self, name: &[u8], from_env: bool, reason: &[u8]) {
        for part in start::preload_ignored(name, if from_env { start::PRELOAD_ENV } else { self.file_path }, reason) {
            write_all(2, part);
        }
    }
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
    path: (usize, usize),
    parent: usize,
    bias: u64,
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
/// on stdout, status 127, as the reference's `ld.so --list` does (DL-8 probe).
fn list(env: &List<'_>, prog: &'static [u8], strict: bool) -> ! {
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
    if dynamic_of(file).is_none() {
        write_all(1, b"\tnot a dynamic executable\n");
        exit(1)
    }
    // The cache, once (R14-3 (a): `<prefix>/etc/ld.so.cache`); unreadable or inconsistent = none.
    let cache = read_cache();
    let ctx = env.ctx(cache.as_ref().map(|(_, c)| c).into());
    let mut objs = [Obj::default(); LIST_OBJECTS_MAX];
    let mut store = [0u8; PATHS_MAX];
    let first = store.get_mut(..prog.len()).unwrap_or_else(|| fail(prog, b"path too long\n"));
    first.copy_from_slice(prog);
    let mut used = prog.len();
    if let Some(o) = objs.first_mut() {
        *o = Obj { name: prog, file, path: (0, prog.len()), ..Obj::default() };
    }
    let mut count = 1usize;
    let mut i = 0usize;
    while i < count {
        let me = i;
        i = i.wrapping_add(1);
        let Some(d) = objs.get(me).and_then(|o| dynamic_of(o.file)) else { continue };
        let (old, free) = store.split_at_mut(used);
        let old: &[u8] = old;
        let mut wrote = 0usize;
        // The requester chain: this object, then each object that loaded it, up to the program.
        let mut chain = [Scope::default(); LIST_OBJECTS_MAX];
        let mut depth = 0usize;
        let mut k = me;
        while let (Some(slot), Some(o)) = (chain.get_mut(depth), objs.get(k)) {
            let od = dynamic_of(o.file);
            let origin = old.get(o.path.0..o.path.0.saturating_add(o.path.1)).map(dir_of);
            *slot = Scope { rpath: od.as_ref().and_then(|x| dyn_string(x, elf::dt::RPATH)), runpath: od.as_ref().and_then(|x| dyn_string(x, elf::dt::RUNPATH)), origin };
            depth = depth.wrapping_add(1);
            if k == 0 {
                break;
            }
            k = o.parent;
        }
        let chain = chain.get(..depth).unwrap_or(&[]);
        for name in d.needed().flatten() {
            let known = objs.get(..count).unwrap_or(&[]).iter().any(|o| o.name == name || (!o.soname.is_empty() && o.soname == name));
            if name == Native::INTERP.as_bytes() || known {
                continue;
            }
            if count >= LIST_OBJECTS_MAX {
                fail(name, b"too many shared objects\n");
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
            let at = wrote;
            let dst = free.get_mut(at..at.saturating_add(found.len())).unwrap_or_else(|| fail(found, b"too many shared objects\n"));
            dst.copy_from_slice(found);
            wrote = wrote.saturating_add(found.len());
            let soname = dynamic_of(f).and_then(|x| dyn_string(&x, elf::dt::SONAME)).unwrap_or(b"");
            if let Some(o) = objs.get_mut(count) {
                *o = Obj { name, soname, file: f, path: (used.saturating_add(at), found.len()), parent: me, bias: loaded.bias };
            }
            count = count.wrapping_add(1);
        }
        used = used.saturating_add(wrote);
    }
    if let Some(v) = env.vdso.filter(|&v| v != 0) {
        for part in [&b"\tlinux-vdso.so.1 (0x"[..], &hex16(v), b")\n"] {
            write_all(1, part);
        }
    }
    for o in objs.get(1..count).unwrap_or(&[]) {
        if o.file.is_empty() {
            for part in [&b"\t"[..], o.name, b" => not found\n"] {
                write_all(1, part);
            }
        } else {
            let found = store.get(o.path.0..o.path.0.saturating_add(o.path.1)).unwrap_or(&[]);
            for part in [&b"\t"[..], o.name, b" => ", found, b" (0x", &hex16(o.bias), b")\n"] {
                write_all(1, part);
            }
        }
    }
    let interp = interp.unwrap_or(Native::INTERP.as_bytes());
    let base = u64::try_from(env.base).unwrap_or(0);
    for part in [&b"\t"[..], interp, b" (0x", &hex16(base), b")\n"] {
        write_all(1, part);
    }
    exit(0)
}

mod io;
mod state;
mod tls;
mod image;
mod initfini;
mod dlopen;
mod readers;

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
