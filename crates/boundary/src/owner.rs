//! The opted-in registrations (DL-ATEXIT-OWNER, D-27.29; handoff/DL-NONSHARED-OPTOUT.md, owner
//! rulings #4860-#4862): `atexit`, `at_quick_exit` and `pthread_atfork` exported by `libc.so.6` at
//! the libniji-only, NON-default version node `NIJI_OWNER_1`, for modules linked without
//! `libc_nonshared.a` (the link-stub directory `niji-owner/`). A default link never binds them.
//!
//! * **Attribution (mechanism (a)).** The entry shims (`csys/owner/owner.S`) pass the caller's
//!   return address. The owner is the module holding it, found once at registration through the
//!   loader's `_dl_find_object` lookup (lock-free: a registration may run in a constructor while
//!   `dlopen` holds the loader's lock), and stored as that module's lowest mapped address. No
//!   module (JIT or anonymous memory): the process, whose entries run at exit only.
//! * **The net (mechanism (b), ruling #4862: opted-in entries only).** When `ld.so` unloads a
//!   module it calls [`unload`] with the module's mapped range: every opted-in entry whose owner
//!   OR handler lies in it runs (exit list) or is forgotten (quick-exit list, atfork list), so a
//!   handler is never called after its code is unmapped. Entries registered through the archive's
//!   stubs or `__cxa_atexit` keep the owner-only behaviour (`__cxa_finalize`).

use core::sync::atomic::Ordering::Acquire;

use niji_abi::ctypes::c_int;
use niji_abi::layout::struct_dl_find_object;
use niji_legacy::exit_handlers::{self, OPTED_EXIT, OPTED_QUICK};

use crate::heap::KernelPages;
use crate::process::{self, EXIT_HANDLERS, QUICK_HANDLERS};
use crate::sys::CSys;
use crate::{conv, fork, ret};

/// The owner word of an entry registered from return address `ret`: the lowest mapped address of
/// the module holding `ret`, 0 (the process) when no module holds it or no loader runs.
pub fn module_of(ret_addr: usize) -> usize {
    let find = crate::posix::s27_dynlink::LOADER_FIND.load(Acquire);
    if find == 0 || ret_addr == 0 {
        return 0;
    }
    let mut r = struct_dl_find_object { dlfo_flags: 0, dlfo_map_start: 0, dlfo_map_end: 0, dlfo_link_map: 0, dlfo_eh_frame: 0, __dflo_reserved: [0; 7] };
    let w = |v: usize| i64::from_ne_bytes(u64::try_from(v).unwrap_or(0).to_ne_bytes());
    let at = core::ptr::from_mut(&mut r).expose_provenance();
    if crate::ffi::niji_machine_call3(find, w(ret_addr), w(at), 0) != 0 {
        return 0;
    }
    r.dlfo_map_start
}

/// `atexit(f)` at `NIJI_OWNER_1`, called from return address `ret_addr`.
pub fn atexit(f: usize, ret_addr: usize) -> c_int {
    let owner = module_of(ret_addr);
    ret::registered(process::register(process::entry(OPTED_EXIT, f, 0, owner)))
}

/// `at_quick_exit(f)` at `NIJI_OWNER_1`.
pub fn at_quick_exit(f: usize, ret_addr: usize) -> c_int {
    let owner = module_of(ret_addr);
    ret::registered(process::register_quick(process::entry(OPTED_QUICK, f, 0, owner)))
}

/// `pthread_atfork(prepare, parent, child)` at `NIJI_OWNER_1`: the node's owner word is the
/// owner with bit 0 set (a module's lowest address is page-aligned and a `__dso_handle` is
/// pointer-aligned, so the bit marks an opted-in node and never equals a `__cxa_finalize` key).
pub fn pthread_atfork(prepare: usize, parent: usize, child: usize, ret_addr: usize) -> c_int {
    let owner = module_of(ret_addr) | 1;
    ret::errnum(fork::register_dso(prepare, parent, child, owner))
}

/// `ld.so` is unloading the module mapped at `[lo, hi)` (after its finalisers ran): run the
/// opted-in exit entries it owns or whose handler it holds, most recent first, and forget such
/// quick-exit and atfork entries.
pub fn unload(lo: usize, hi: usize) {
    exit_handlers::finalize_range(&EXIT_HANDLERS, &QUICK_HANDLERS, &CSys, &KernelPages, lo, hi, &mut conv::CCalls);
    fork::forget_opted(lo, hi);
}

crate::internal_entry! {
/// The `atexit@NIJI_OWNER_1` shim's target (`csys/owner/owner.S`).
pub extern "C" fn niji_owner_atexit(f: usize, ret_addr: usize) -> c_int {
    atexit(f, ret_addr)
}}

crate::internal_entry! {
/// The `at_quick_exit@NIJI_OWNER_1` shim's target.
pub extern "C" fn niji_owner_at_quick_exit(f: usize, ret_addr: usize) -> c_int {
    at_quick_exit(f, ret_addr)
}}

crate::internal_entry! {
/// The `pthread_atfork@NIJI_OWNER_1` shim's target.
pub extern "C" fn niji_owner_pthread_atfork(prepare: usize, parent: usize, child: usize, ret_addr: usize) -> c_int {
    pthread_atfork(prepare, parent, child, ret_addr)
}}
