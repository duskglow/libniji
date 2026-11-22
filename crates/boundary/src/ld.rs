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
//!
//! **Module map** (LD-SPLIT: one module per concern, so loader stories on different concerns edit
//! different files; `tools/ld_split.py` made the split). This file keeps the product glue: the
//! panic handler, `NoHeap`, the local memory symbols (`ld_mem!`), the debugger rendezvous's exports
//! (`_r_debug`, `_dl_debug_state`) and the re-exports that keep every item at its old path.
//!
//! * `io`: writes, `exit_group`, initial-stack and C-string reads, the fatal start-up diagnostics.
//! * `state`: the start-time state the parts share (own base, the start-time objects, bounds).
//! * `tls`: the provisional thread block, static TLS and its surplus, the module table, the dtv,
//!   `__tls_get_addr`, `_dl_get_tls_static_info`.
//! * `image`: opening, identifying and mapping objects, image views, the cache file, candidates.
//! * `initfini`: constructors after relocation and the exit finalisers of start-time objects.
//! * `link`: program start's link: `DT_NEEDED` search, preloads, mapping, relocation and binding,
//!   the TLS layout, the rendezvous's start-time link maps.
//! * `trace`: the dependency list (`--list`, `ldd`, `LD_TRACE_LOADED_OBJECTS`).
//! * `entry`: `niji_ld_start` and `run`: auxv and environment, secure mode, rseq, the start sequence.
//! * `exec_stack`: COMPAT-19 executable stacks: accepting or refusing objects that ask for one at
//!   start, and making the main and thread stacks executable before any constructor.
//! * `debug`: the debugger rendezvous (`r_debug`, `r_brk` calls).
//! * `dlopen` (with `dlopen/dladdr`, `dlopen/dlsym`): `dlopen`, `dlsym`, `dlerror`, `dlclose`,
//!   `dladdr`, `dlinfo`, exit-set finalisers.
//! * `readers`: the lock-free readers (`dl_iterate_phdr`, `_dl_find_object`).
//! * `table`: the tables that grow with the number of objects (inline, then a mapping).

use core::alloc::{GlobalAlloc, Layout};
#[cfg(not(feature = "export"))]
use core::ffi::{c_int, c_void};
use niji_ld::program;
use self::state::{AUXV_WORDS_MAX, OWN_BASE, START_ARGS, START_INTERP, START_N, START_ORIGIN, START_PAGE, START_RUN_PATH, start_obj};
use self::tls::{INITIAL_TAIL, NewTls, THREAD_MODS_LEN, initial_place, module_room, publish_tls, release_tls, surplus, thread_mod, tls_generation};
use self::image::{AT_FDCWD, MAP_ANONYMOUS_I32, MAP_PRIVATE_I32, ProcImage, SysMapper, dyn_string, image_view, map_view, own_object, read_cache, read_image};
use self::initfini::{array_words, fn_addr0};
use self::entry::{env_entries, read_auxv, report_nonshared};

pub(crate) use self::io::{c_string, exit, stack_word, write_all};
pub use self::tls::{__tls_get_addr, _dl_get_tls_static_info};
pub use self::entry::{__libc_stack_end, __rseq_flags, __rseq_offset, __rseq_size, niji_ld_start};
pub(crate) use self::exec_stack::exec_main_stack;

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

mod io;
mod state;
mod tls;
mod image;
mod initfini;
mod link;
mod trace;
mod entry;
mod exec_stack;
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
