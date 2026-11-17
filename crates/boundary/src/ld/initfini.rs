//! Constructors and exit finalisers of the start-time objects (DL-11, D-27.19): `run_init`
//! runs the initialisation functions after relocation and records the termination hooks
//! that `fini_all` runs at exit.

use core::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, AtomicUsize};
use core::sync::atomic::Ordering::{AcqRel, Acquire, Relaxed, Release};
use niji_ld::{init, start};
use crate::conv;
use super::{dlopen, readers};
use super::io::{refuse, stack_word};
use super::state::{OBJECTS_MAX, START_ARGS};
use super::tls::{THREAD_TLS_ALIGN, THREAD_TLS_SIZE, thread_tls_init, thread_tls_release};

/// The address of one of the loader's `extern "C" fn(usize) -> usize` hooks.
#[allow(clippy::as_conversions)] // a function's address has no conversion other than `as`
fn fn_addr(f: extern "C" fn(usize) -> usize) -> usize {
    (f as *const ()).expose_provenance()
}

/// The words of a function array in a mapped, relocated object.
pub(super) fn array_words(a: init::Array) -> &'static [u64] {
    let (Ok(addr), Ok(len)) = (usize::try_from(a.addr), usize::try_from(a.len)) else { return &[] };
    // UNSAFE(slice): an object's DT_PREINIT_ARRAY, DT_INIT_ARRAY or DT_FINI_ARRAY, read only.
    #[allow(unsafe_code)]
    // SAFETY: `addr` is bias + the array's link-time address in an object mapped for the process's
    // lifetime (the loader never unmaps an object at start), `len` its size tag / 8, both from the
    // object's dynamic array as the static linker wrote them; relocation already ran.
    unsafe {
        conv::words_at(addr, len)
    }
}

/// The objects' termination hooks in finalisation order (DL-11): `[DT_FINI_ARRAY address, entries,
/// DT_FINI]` per object, written once before the program is entered, read by [`fini_all`].
static FINI_HOOKS: [[AtomicU64; 3]; OBJECTS_MAX] = [const { [const { AtomicU64::new(0) }; 3] }; OBJECTS_MAX];
static FINI_LEN: AtomicUsize = AtomicUsize::new(0);
/// Each [`FINI_HOOKS`] slot's start-time object (its index in [`START_OBJS`], the `dlopen` loader's
/// node index), for the exit order with the `dlopen`ed objects (DL-EXIT-ORDER-PROBE).
static FINI_IDX: [AtomicU8; OBJECTS_MAX] = [const { AtomicU8::new(0) }; OBJECTS_MAX];
static FINI_DONE: AtomicBool = AtomicBool::new(false);

/// Run the initialisation functions after relocation (design §9, D-27.19): the program's
/// `DT_PREINIT_ARRAY`, then each other object's `DT_INIT` and `DT_INIT_ARRAY` in
/// [`init::init_order`], each called with `(argc, argv, envp)` from the initial stack at `sp`. The
/// program's own `DT_INIT`/`DT_INIT_ARRAY` are its C library's start (probe: the reference loader
/// runs neither). Records the finalisation order for [`fini_all`] and returns its address, the
/// termination function the program gets in `%rdx`. First of all, `early` (the C library's
/// `__niji_libc_early_init`, design §14, D-27.22) gets the initial stack, so every constructor,
/// `.preinit_array` included, sees an initialised C library.
pub(super) fn run_init(hooks: &[init::Hooks], edges: &[(u8, u8)], sp: usize, early: Option<u64>, rseq_size: usize) -> usize {
    let mut order = [0u8; OBJECTS_MAX];
    let mut fini_order = [0u8; OBJECTS_MAX];
    let (Ok(len), Ok(fini_len)) = (init::init_order(hooks.len(), edges, &mut order), init::fini_order(hooks.len(), edges, &mut fini_order)) else {
        refuse(b"ld.so: cannot order the objects' initialisation\n")
    };
    let argc = stack_word(sp);
    let argv = sp.wrapping_add(8);
    let envp = argv.wrapping_add(usize::try_from(argc).unwrap_or(0).wrapping_add(1).wrapping_mul(8));
    for (slot, v) in START_ARGS.iter().zip([usize::try_from(argc).unwrap_or(0), argv, envp]) {
        slot.store(v, Relaxed);
    }
    let args = |v: usize| i64::from_ne_bytes(u64::try_from(v).unwrap_or(0).to_ne_bytes());
    let (a, b, c) = (i64::from_ne_bytes(argc.to_ne_bytes()), args(argv), args(envp));
    let call = |f: u64| {
        let _ = crate::ffi::niji_machine_call3(usize::try_from(f).unwrap_or(0), a, b, c);
    };
    if let Some(f) = early {
        let mut auxv = envp;
        while stack_word(auxv) != 0 {
            auxv = auxv.wrapping_add(8);
        }
        START_ARGS[3].store(auxv.wrapping_add(8), Relaxed);
        // The program's own DT_INIT and DT_INIT_ARRAY go to its C library's start (design §14,
        // D-27.24): `__libc_start_main` runs them after every shared object's initialisation.
        let prog = hooks.first().copied().unwrap_or_default();
        let w = |v: u64| usize::try_from(v).unwrap_or(0);
        let info = start::private::StartInfo {
            argc: usize::try_from(argc).unwrap_or(0),
            argv,
            envp,
            auxv: auxv.wrapping_add(8),
            stack_end: sp,
            init: w(prog.init),
            init_array: w(prog.init_array.addr),
            init_array_len: w(prog.init_array.len),
            rseq_size,
            tls_size: usize::try_from(THREAD_TLS_SIZE.load(Relaxed)).unwrap_or(0),
            tls_align: usize::try_from(THREAD_TLS_ALIGN.load(Relaxed)).unwrap_or(0),
            tls_init: fn_addr(thread_tls_init),
            tls_release: fn_addr(thread_tls_release),
            dlopen: dlopen::entry_address(),
            dlsym: dlopen::dlsym::entry_address(),
            dl_iterate: readers::iterate_address(),
            dl_find_object: readers::find_object_address(),
            dl_fork: dlopen::fork_address(),
            dlclose: dlopen::close_address(),
            dladdr: dlopen::dladdr::dladdr_address(),
            dlinfo: dlopen::dladdr::dlinfo_address(),
            dl_pin: dlopen::pin_address(),
        };
        let at = core::ptr::from_ref(&info).expose_provenance();
        let _ = crate::ffi::niji_machine_call3(usize::try_from(f).unwrap_or(0), args(at), 0, 0);
    }
    // Only the program's preinit array is non-empty (`init::hooks` drops a shared object's).
    for h in hooks {
        init::init_calls(0, array_words(h.preinit)).for_each(call);
    }
    for &i in order.get(..len).unwrap_or(&[]) {
        let Some(h) = hooks.get(usize::from(i)).filter(|_| i != 0) else { continue };
        init::init_calls(h.init, array_words(h.init_array)).for_each(call);
    }
    for ((slot, x), &i) in FINI_HOOKS.iter().zip(&FINI_IDX).zip(fini_order.get(..fini_len).unwrap_or(&[])) {
        x.store(i, Relaxed);
        let h = hooks.get(usize::from(i)).copied().unwrap_or_default();
        for (w, v) in slot.iter().zip([h.fini_array.addr, h.fini_array.len, h.fini]) {
            w.store(v, Relaxed);
        }
    }
    FINI_LEN.store(fini_len, Release);
    #[allow(clippy::as_conversions)] // a function's address has no conversion other than `as`
    (fini_all as *const ()).expose_provenance()
}

/// The termination function passed to the program (`rtld_fini`): each object's `DT_FINI_ARRAY` in
/// reverse, then its `DT_FINI`. Without `dlopen`, the objects in [`init::fini_order`], the program
/// first; with it, the start-time and `dlopen`ed objects as one sequence ([`dlopen::fini_exit`],
/// DL-EXIT-ORDER-PROBE). Runs once.
extern "C" fn fini_all() {
    if FINI_DONE.swap(true, AcqRel) {
        return;
    }
    let len = FINI_LEN.load(Acquire).min(OBJECTS_MAX);
    let start: [(usize, [u64; 3]); OBJECTS_MAX] = core::array::from_fn(|k| {
        let words = FINI_HOOKS.get(k).map_or([0; 3], |s| [0, 1, 2].map(|j| s.get(j).map_or(0, |w| w.load(Relaxed))));
        (FINI_IDX.get(k).map_or(0, |x| usize::from(x.load(Relaxed))), words)
    });
    let start = start.get(..len).unwrap_or(&[]);
    if dlopen::fini_exit(start) {
        return;
    }
    for &(_, [addr, len, fini]) in start {
        for f in init::fini_calls(fini, array_words(init::Array { addr, len })) {
            let _ = crate::ffi::niji_machine_call0(usize::try_from(f).unwrap_or(0));
        }
    }
}
