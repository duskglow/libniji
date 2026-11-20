//! COMPAT-19, executable stacks (opt-in tunable `niji.compat.execstack`): whether the start
//! accepts an object that asks for one, the refusals, and making the main and thread
//! stacks executable before any constructor runs.

use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering::Relaxed;
use niji::config::Tunables;
use niji_kernel::uapi::AT_EXECFN;
use niji_ld::arch::{Arch, Native};
use niji_ld::program;
use niji_ld::dlopen::Refusal;
use niji_ld::{map, start};
use crate::raw;
use super::dlopen;
use super::entry::List;
use super::image::SysMapper;
use super::io::{c_string, fail_loading, fail_with, stack_word};
use super::state::START_PAGE;

/// COMPAT-19: set once an object asking for an executable stack is accepted at start (the program
/// or a start-up dependency); `run_init` then makes the stacks executable.
pub(super) static EXEC_STACK: AtomicBool = AtomicBool::new(false);

/// `PROT_GROWSDOWN` (kernel UAPI `asm-generic/mman-common.h`): the change reaches down to the start
/// of a mapping that grows down (mprotect(2)).
const PROT_GROWSDOWN: i32 = 0x0100_0000;

/// COMPAT-19: whether the start's `GLIBC_TUNABLES` set `niji.compat.execstack=1` (never in a secure
/// process: `Tunables::parse` honours nothing there, as for COMPAT-12, ruling #2662). Reached only
/// for an object asking for an executable stack; a yes is remembered for `run_init`.
#[cold]
#[inline(never)]
pub(super) fn exec_stack_accepted(ld: &start::LdEnv<'_>, secure: bool) -> bool {
    let yes = ld.tunables.is_some_and(|t| Tunables::parse(t, secure).exec_stack == Some(true));
    if yes {
        EXEC_STACK.store(true, Relaxed);
    }
    yes
}

/// COMPAT-19: the program asks for an executable stack and the tunable does not accept it: refused,
/// naming the program (its path under `ld.so PROGRAM`, else `argv[0]` above `sp`) and the ways in.
#[cold]
#[inline(never)]
pub(super) fn refuse_exec_stack_program(sp: usize, path: usize) -> ! {
    let at = if path != 0 { path } else { usize::try_from(stack_word(sp.wrapping_add(8))).unwrap_or(0) };
    let name: &[u8] = if at != 0 { c_string(at) } else { b"the program" };
    fail_with(&[name, b": the program asks for an executable stack (PT_GNU_STACK with PF_X); refused; ", niji_ld::dlopen::EXEC_STACK_WAYS, b"\n"], program::FAILURE)
}

/// COMPAT-19: start-up object `name` (needed by `prog`, file `file` open as `fd`) asks for an
/// executable stack: mapped when the tunable accepts it (the search's candidate check let it
/// through only then), else refused as that check refuses it, naming both and the ways in.
#[cold]
#[inline(never)]
pub(super) fn exec_stack_dependency(fd: i32, file: &[u8], page: u64, name: &[u8], prog: &[u8], env: &List<'_>) -> Result<map::Loaded, map::MapError> {
    if !exec_stack_accepted(env.ld, env.secure) {
        fail_loading(prog, name, Refusal::ExecutableStack.at_start().0);
    }
    map::load_accepting_exec_stack::<Native, _>(&mut SysMapper { fd }, file, page)
}

/// COMPAT-19: make the main stack executable. The kernel puts the `AT_EXECFN` string at the top of
/// the initial stack mapping, so its last byte's page is that mapping's last page; one protection
/// change on that page with `PROT_GROWSDOWN` reaches the whole mapping, and the stack's later growth
/// keeps it. Without `AT_EXECFN` the page of the auxiliary vector `auxv` is used (all below it).
#[cold]
#[inline(never)]
pub(crate) fn exec_main_stack(auxv: usize, page: u64) {
    let mut at = auxv;
    let mut top = auxv;
    while at != 0 {
        let t = stack_word(at);
        if t == 0 {
            break;
        }
        if t == AT_EXECFN {
            let s = usize::try_from(stack_word(at.wrapping_add(8))).unwrap_or(0);
            if s != 0 {
                top = s.wrapping_add(c_string(s).len());
            }
            break;
        }
        at = at.wrapping_add(16);
    }
    let page = usize::try_from(page).ok().filter(|p| p.is_power_of_two()).unwrap_or(4096);
    if top != 0 {
        let rwx = map::PROT_READ | map::PROT_WRITE | map::PROT_EXEC;
        let _ = raw::mprotect(top & !page.wrapping_sub(1), page, rwx | PROT_GROWSDOWN);
    }
}

/// COMPAT-19 at start, before any constructor: the main stack (its auxiliary vector follows the
/// environment `envp`) and the C library's thread stacks become executable.
#[cold]
#[inline(never)]
pub(super) fn exec_stacks_at_start(envp: usize) {
    let mut auxv = envp;
    while stack_word(auxv) != 0 {
        auxv = auxv.wrapping_add(8);
    }
    exec_main_stack(auxv.wrapping_add(8), START_PAGE.load(Relaxed));
    dlopen::stacks_executable_at_start();
}

/// COMPAT-19: a start candidate (dependency or preload) asking for an executable stack passes the
/// remaining checks when the tunable accepts it; refused otherwise.
#[cold]
#[inline(never)]
pub(super) fn exec_stack_candidate(file: &[u8], page: u64, env: &List<'_>) -> Result<(), Refusal> {
    if !exec_stack_accepted(env.ld, env.secure) {
        return Err(Refusal::ExecutableStack);
    }
    niji_ld::dlopen::check_accepting_exec_stack::<Native>(file, page).map(|_| ())
}
