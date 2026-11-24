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
use super::exec_stack::{exec_stack_accepted, refuse_exec_stack_program};
use super::image::{load, open_whole, program_file, read_image};
use super::io::{c_string, exit, fail_loading, lo32, refuse, stack_word, write_all};
use super::link::{Origin, Preloads, link};
use super::state::{AUXV_WORDS_MAX, OWN_BASE, PHDR_WORDS, START_RUN_PATH};
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

/// The entries of [`env_entries`] whose name starts with `LD_` (the loader's own settings a
/// `dlopen` reads), in order: every other entry costs a three-byte test, never a measurement.
pub(super) fn ld_entries(envp: usize) -> impl Iterator<Item = &'static [u8]> {
    (0usize..)
        .map(move |i| usize::try_from(stack_word(envp.wrapping_add(i.wrapping_mul(8)))).unwrap_or(0))
        .take_while(|&p| p != 0)
        .filter(|&p| super::io::starts_with(p, b"LD_"))
        .map(c_string)
}

/// D-21.6: the shadow-stack mode the start's tunables asked for (`niji_ld::shstk::code`; 0: not asked,
/// the default), read by `link` (whether to enable) and by `dlopen` (an unmarked object).
pub(super) static SHSTK_WANTED: core::sync::atomic::AtomicU8 = core::sync::atomic::AtomicU8::new(0);

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
    // D-21.6: the shadow stack, only when the tunables ask for it (`link` decides, `dlopen` applies).
    if let Some(m) = tunables.and_then(|t| t.shstk_wanted()) {
        let m = match m {
            niji::config::ShstkMode::On => niji_ld::shstk::ShstkMode::On,
            niji::config::ShstkMode::Permissive => niji_ld::shstk::ShstkMode::Permissive,
            _ => niji_ld::shstk::ShstkMode::Default,
        };
        SHSTK_WANTED.store(niji_ld::shstk::code(m), Relaxed);
    }
    // D-27.39: a larger static surplus asked (raise-only), before the TLS layout.
    if let Some(bytes) = tunables.and_then(|t| t.optional_static_tls) {
        super::tls::ask_surplus(bytes);
    }
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
    let list_env = List { ld: &ld, secure, preload, platform: Native::platform(&features), hwcaps: Native::hwcaps(&features), page, vdso: aux.get(AT_SYSINFO_EHDR), base: ehdr };
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
        run(sp, &prog, bias, page, 0, &list_env)
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
        Action::Tunables => super::report::tunables(ld.tunables, secure),
        Action::Diagnostics => super::report::diagnostics(list_env.hwcaps, page, list_env.platform, envp, copy.get(..words).unwrap_or(&[])),
        Action::Program { mode, name, at, options } => {
            let start = Start { argv, argc, auxv, copy, words, invoked };
            program_mode(&start, mode, name, at, &options, &list_env)
        }
        other => {
            let me = if argc == 0 { &b"ld.so"[..] } else { c_string(arg(0)) };
            for part in program::refusal(other, me) {
                write_all(2, part);
            }
            exit(program::USAGE)
        }
    }
}

/// The initial stack of `ld.so` run as a program, as `niji_ld_start` read it.
struct Start {
    argv: usize,
    argc: usize,
    /// Where the auxiliary vector is now (after any secure-mode move), its copy and its words.
    auxv: usize,
    copy: [u64; AUXV_WORDS_MAX],
    words: usize,
    /// The path `ld.so` was run by (`AT_EXECFN`), for `--list`.
    invoked: Option<&'static [u8]>,
}

/// `ld.so [OPTION]... PROGRAM [ARGS]` (LDSO-RUN-PROGRAM): apply the options, then list, verify or run
/// the program. Running maps the program as the kernel would have and goes on through the same start
/// as an exec'd program ([`run`]); a program with no `PT_INTERP` (static, or a static PIE) is entered
/// as the kernel enters it, with nothing of the loader's start. Cold: an exec'd program never comes here.
#[cold]
#[inline(never)]
fn program_mode(st: &Start, mode: program::Mode, name: &'static [u8], at: usize, options: &program::Options<'static>, env: &List<'_>) -> ! {
    // `--library-path` replaces LD_LIBRARY_PATH for the search (the environment keeps its value);
    // dlopen reads the same choices later (`state::program_options`).
    let mut ld = *env.ld;
    if options.library_path.is_some() {
        ld.library_path = options.library_path;
    }
    super::state::keep_program_options(options.library_path, options.inhibit_cache);
    // `--preload`'s names follow `LD_PRELOAD`'s under its rule: folded into its value here, so the
    // start's preload list is the exec'd start's, unchanged.
    let preload = options.preload.map_or(env.preload, |p| Some(fold_preload(env.preload, p)));
    if let Some(list) = options.inhibit_rpath {
        super::state::keep_inhibit_rpath(list);
    }
    // The hwcaps options give the subdirectory list every search of the process uses.
    let hwcaps = if options.hwcaps_prepend.is_some() || options.hwcaps_mask.is_some() { hwcaps_list(env.hwcaps, options.hwcaps_prepend, options.hwcaps_mask) } else { env.hwcaps };
    let env = List { ld: &ld, preload, hwcaps, ..*env };
    // A name without `/` is found through the loader cache only (D-27.69); argv[0] stays the name as
    // given (or `--argv0`'s), the file's path names it in messages and gives its `$ORIGIN`.
    if mode == program::Mode::Verify {
        // Only a path is opened (no search, never the working directory for a bare name, D-27.69):
        // a name without `/` verifies as no program, status 1.
        let file = Some(name).filter(|n| n.contains(&b'/')).and_then(open_whole).map(|(f, fd)| {
            let _ = crate::ffi_sys::niji_sys_close(fd);
            f
        });
        exit(program::verify_status::<Native>(file))
    }
    let file = program_file(name, &env);
    if mode == program::Mode::List {
        list(&env, file, true, st.invoked)
    }
    if ld.trace && !env.secure {
        list(&env, file, false, st.invoked)
    }
    for a in options.audit.into_iter().flat_map(program::audit_names) {
        program::audit_ignored(a).iter().for_each(|p| write_all(2, p));
    }
    let path = file.as_ptr().expose_provenance();
    let (prog, bias) = load(path, env.page, &ld, env.secure);
    let headers = program_headers(&prog);
    let interp = headers.chunks_exact(PHDR_WORDS).take(prog.phnum).any(|h| h.first().is_some_and(|w| w & 0xffff_ffff == u64::from(niji_ld::elf::pt::INTERP)));
    let mut copy = st.copy;
    // AT_BASE: the interpreter's base, 0 for a program the kernel would start without one.
    start::rewrite(copy.get_mut(..st.words).unwrap_or(&mut []), &prog, if interp { env.base } else { 0 });
    store_words(st.auxv, copy.get(..st.words.wrapping_add(2)).unwrap_or(&[]));
    // argv[at + 1] names the program: it becomes argv[0], and the stack starts at the word before it,
    // which takes the new argc.
    let top = st.argv.wrapping_add(at.wrapping_mul(8));
    store_words(top, &[u64::try_from(st.argc.wrapping_sub(at).wrapping_sub(1)).unwrap_or(0)]);
    if let Some(a) = options.argv0 {
        // The string is the loader's own argument, on the stack for the process's lifetime; an empty
        // one may not point there (an empty slice's address is arbitrary), so it is the loader's "".
        let s = if a.is_empty() { b"\0".as_slice() } else { a };
        store_words(top.wrapping_add(8), &[u64::try_from(s.as_ptr().expose_provenance()).unwrap_or(0)]);
    }
    if !interp {
        // A static program or static PIE starts itself, as from the kernel. An object with no entry
        // point (`e_entry` 0: a shared library) is refused, never entered.
        if prog.entry == bias {
            fail_loading(file, file, b"the object has no entry point (a shared object, not a program)")
        }
        crate::arch::enter(top, prog.entry, 0)
    }
    run(top, &prog, bias, env.page, path, &env)
}

/// The `LD_PRELOAD` value with `ld.so --preload LIST`'s names after its own (`env`, the start's value,
/// if any): `option` itself when there is no other, else a copy of both joined by a colon, kept for the
/// process (a kept name may become an object's name). Where `option`'s names lie is kept for the
/// diagnostic's source (`state::in_preload_option`). Cold: program mode only.
#[cold]
#[inline(never)]
fn fold_preload(env: Option<&'static [u8]>, option: &'static [u8]) -> &'static [u8] {
    let Some(env) = env.filter(|e| !e.is_empty()) else {
        super::state::keep_preload_option(option);
        return option;
    };
    // The two values, a colon between them and a NUL after them.
    let len = env.len().saturating_add(option.len()).saturating_add(2);
    let flags = super::MAP_PRIVATE_I32 | super::MAP_ANONYMOUS_I32;
    let at = crate::raw::mmap(0, len, map::PROT_READ | map::PROT_WRITE, flags, -1, 0).unwrap_or_else(|_| refuse(b"ld.so: cannot allocate memory for --preload\n"));
    // UNSAFE(slice): a fresh anonymous mapping, owned by this value.
    #[allow(unsafe_code)]
    // SAFETY: `len` bytes from `at` were just mapped private, readable and writable; nothing else refers
    // to them, and they are never unmapped (the value lives as long as the process).
    let buf = unsafe { conv::bytes_at_mut(at, len) };
    // The mapping is zero-filled, so the NUL is there already. Copied without an operation that can
    // panic: each panic site's location is a self-relocation that every start of ld.so pays for.
    let joined = env.iter().chain(b":").chain(option);
    for (to, &from) in buf.iter_mut().zip(joined) {
        *to = from;
    }
    let value: &'static [u8] = buf;
    let value = value.get(..len.saturating_sub(1)).unwrap_or(&[]);
    super::state::keep_preload_option(value.get(env.len().saturating_add(1)..).unwrap_or(&[]));
    value
}

/// The `glibc-hwcaps` subdirectories for `--glibc-hwcaps-prepend` and `--glibc-hwcaps-mask`
/// ([`program::hwcaps_subdirs`] of the supported ones, `builtin`), in a private mapping kept for the
/// process and for `dlopen` (`state::program_hwcaps`). Names are bytes, as given. Cold: program
/// mode only.
#[cold]
#[inline(never)]
fn hwcaps_list(builtin: &'static [&'static [u8]], prepend: Option<&'static [u8]>, mask: Option<&'static [u8]>) -> &'static [&'static [u8]] {
    let n = program::hwcaps_subdirs(builtin, prepend, mask).count();
    let len = n.saturating_mul(core::mem::size_of::<&[u8]>()).max(1);
    let flags = super::MAP_PRIVATE_I32 | super::MAP_ANONYMOUS_I32;
    let at = crate::raw::mmap(0, len, map::PROT_READ | map::PROT_WRITE, flags, -1, 0).unwrap_or_else(|_| refuse(b"ld.so: cannot allocate memory for the glibc-hwcaps options\n"));
    // UNSAFE(slice): a fresh anonymous mapping as `n` slots for names, none read before written.
    #[allow(unsafe_code)]
    // SAFETY: `len` >= n * size_of::<&[u8]>() bytes from `at` were just mapped private and writable,
    // page-aligned (so aligned for `&[u8]`); nothing else refers to them, and they are never unmapped.
    let slots = unsafe { core::slice::from_raw_parts_mut(core::ptr::with_exposed_provenance_mut::<core::mem::MaybeUninit<&'static [u8]>>(at), n) };
    let names = program::hwcaps_subdirs(builtin, prepend, mask);
    let mut kept = 0usize;
    for (slot, name) in slots.iter_mut().zip(names) {
        slot.write(name);
        kept = kept.wrapping_add(1);
    }
    // UNSAFE(slice): the slots just written.
    #[allow(unsafe_code)]
    // SAFETY: the first `kept` (<= n) slots from `at` were initialised above and are not written again.
    let list: &'static [&'static [u8]] = unsafe { core::slice::from_raw_parts(core::ptr::with_exposed_provenance::<&'static [u8]>(at), kept) };
    super::state::keep_program_hwcaps(list);
    list
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
fn run(sp: usize, prog: &start::Program, bias: usize, page: u64, path: usize, env: &List<'_>) -> ! {
    let headers = program_headers(prog);
    // R14-13: a program whose PT_GNU_STACK asks for an executable stack is refused (word 0 of a header
    // holds p_type in its low and p_flags in its high half).
    let types = headers.chunks_exact(PHDR_WORDS).take(prog.phnum).filter_map(|h| h.first()).map(|&w| (lo32(w), lo32(w >> 32)));
    if map::executable_stack(types) && !exec_stack_accepted(env.ld, env.secure) {
        refuse_exec_stack_program(sp, path);
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
            // Its `$ORIGIN`: from the path it was run by (`ld.so PROGRAM`), else from the file the
            // kernel ran (threat model rule 2), looked up only when a search expands it.
            let origin = Origin { path };
            if path != 0 {
                START_RUN_PATH.store(path, Relaxed);
            }
            link(prog, bias, page, if at != 0 { c_string(at) } else { b"the program" }, sp, env, origin, &preloads)
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

/// What the dependency list needs from the start (DL-8).
pub(super) struct List<'a> {
    pub(super) ld: &'a start::LdEnv<'static>,
    pub(super) secure: bool,
    /// The `LD_PRELOAD` value program start uses (in secure mode the one read before the clear).
    preload: Option<&'static [u8]>,
    /// `$PLATFORM` and the `glibc-hwcaps` subdirectories searched (program mode's options may change these).
    platform: &'static str,
    hwcaps: &'static [&'static [u8]],
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
            platform: self.platform,
            hwcaps: self.hwcaps,
            cache,
        }
    }
}
