//! `makecontext` and `__longjmp_chk` (spec §21.1–§21.2; D-21.1–D-21.3): the parts of
//! non-local transfer that need decisions. The register save and restore is the C layer's
//! (`machine/setjmp.S`, `machine/context.S`).

#![allow(clippy::arithmetic_side_effects)] // word offsets inside the caller's context and stack

use core::sync::atomic::{AtomicUsize, Ordering::Relaxed};

use niji_kernel::errno::KResult;
use niji_kernel::sys::Sys;
use niji_kernel::uapi::SS_ONSTACK;

use crate::sys::CSys;
use crate::{ffi, tb};

// `ucontext_t` as words (spec/abi/layouts.tsv): uc_flags 0, uc_link 1, uc_stack.ss_sp 2,
// ss_size 4, the general registers from word 5 (`REG_*` order).
const W_FLAGS: usize = 0;
const W_LINK: usize = 1;
const W_SS_SP: usize = 2;
const W_SS_SIZE: usize = 4;
const W_GREGS: usize = 5;
const REG_R8: usize = 0;
const REG_R9: usize = 1;
const REG_R12: usize = 4;
const REG_RDI: usize = 8;
const REG_RSI: usize = 9;
const REG_RBX: usize = 11;
const REG_RDX: usize = 12;
const REG_RCX: usize = 14;
const REG_RSP: usize = 15;
const REG_RIP: usize = 16;
/// `__ssp` (word 117, byte 936): the shadow-stack pointer the context resumes with, then the
/// base and length of a shadow stack makecontext mapped (D-21.5).
const W_SSP: usize = 117;
/// The registers that carry the first six arguments.
const ARG_REGS: [usize; 6] = [REG_RDI, REG_RSI, REG_RDX, REG_RCX, REG_R8, REG_R9];

fn get(w: &[AtomicUsize], i: usize) -> usize {
    w.get(i).map_or(0, |x| x.load(Relaxed))
}

fn put(w: &[AtomicUsize], i: usize, v: usize) {
    if let Some(x) = w.get(i) {
        x.store(v, Relaxed);
    }
}

/// Where a context function whose `uc_link` is NULL goes: the thread exits (spec §21.2).
extern "C" fn context_end() -> ! {
    crate::cancel::unwind(0)
}

#[allow(clippy::as_conversions)] // a function pointer's address: the only spelling Rust has
fn end_addr() -> usize {
    context_end as extern "C" fn() -> ! as usize
}

#[allow(clippy::as_conversions)] // as above
fn start_context_addr() -> usize {
    ffi::niji_machine_start_context as extern "C" fn() as usize
}

/// `makecontext(uc, func, argc, ...)`: `uc` resumes in `func` with `argc` register-sized
/// arguments on `uc.uc_stack` (glibc accepts 64-bit values on x86-64, §21.2), returning to
/// `uc.uc_link` or ending the thread. `stack` is the stack's words (from its lowest 8-aligned
/// address); `ap` is the adapter's `va_list *`.
pub fn make(uc: KResult<&[AtomicUsize]>, stack: KResult<(usize, &[AtomicUsize])>, func: usize, argc: i32, ap: usize) {
    let (Ok(uc), Ok((base, stack))) = (uc, stack) else { return };
    let Ok(argc) = usize::try_from(argc) else { return };
    let top = get(uc, W_SS_SP) + get(uc, W_SS_SIZE);
    // Above the return address: the arguments past the sixth, then uc_link and the exit
    // function (the trampoline finds them through %rbx).
    let extra = argc.saturating_sub(6);
    let link_idx = extra + 1;
    let Some(sp) = top.checked_sub((link_idx + 2) * 8).map(|v| (v & !15) - 8) else { return };
    if sp < base || (sp - base) / 8 + link_idx + 1 >= stack.len() {
        return;
    }
    let at = |i: usize| (sp - base) / 8 + i;
    // The context starts in niji_machine_start_context, which calls `func` (in %r12) from one
    // word above `sp`: the call writes its return address at `sp`, so `func` starts with the
    // stack it would have been jumped to with, and its return has a shadow-stack entry.
    put(stack, at(link_idx), get(uc, W_LINK));
    put(stack, at(link_idx + 1), end_addr());
    put(uc, W_GREGS + REG_RIP, start_context_addr());
    put(uc, W_GREGS + REG_R12, func);
    put(uc, W_GREGS + REG_RSP, sp + 8);
    put(uc, W_GREGS + REG_RBX, sp + link_idx * 8);
    if ffi::niji_machine_ssp() != 0 {
        shadow_stack(uc, get(uc, W_SS_SIZE));
    }
    for i in 0..argc {
        // One argument per call, exactly argc of them (the obligation of `niji_va_long`).
        let v = usize::from_ne_bytes(ffi::niji_va_long(ap).to_ne_bytes());
        match ARG_REGS.get(i) {
            Some(&r) => put(uc, W_GREGS + r, v),
            None => put(stack, at(i - 5), v),
        }
    }
}

/// The calling thread has a shadow stack: give the new context its own (D-21.5), `len` bytes
/// rounded up to a page (each call takes 8 bytes of it and at least 8 of the context's stack, so
/// it cannot run out first), with a restore token on top (`SHADOW_STACK_SET_TOKEN`), which
/// setcontext and swapcontext switch to. When the kernel refuses, the context records none and
/// runs on its resumer's shadow stack. The interface has no point where a context ends, so the
/// shadow stack is unmapped only when a later makecontext is given the same stack memory
/// (`uc_stack.ss_sp`, [`registry`]; D-21.7 (5)); its base and length also stay in `__ssp[1]`,
/// `__ssp[2]`. `__ssp[0]` takes the raw convention (D-21.7): the token's address, 8 below the
/// SSP the context resumes with; and `uc_flags` becomes 0, a user context, even when `uc` began
/// as a copy of a signal frame's.
#[cold]
#[inline(never)]
fn shadow_stack(uc: &[AtomicUsize], len: usize) {
    const SHADOW_STACK_SET_TOKEN: i64 = 1;
    let len = len.saturating_add(4095) & !4095;
    let mapped = i64::try_from(len).ok().and_then(|l| crate::raw::syscall(crate::nr::MAP_SHADOW_STACK, [0, l, SHADOW_STACK_SET_TOKEN, 0, 0, 0]).ok());
    let base = mapped.and_then(|b| usize::try_from(b).ok()).filter(|_| len != 0);
    put(uc, W_FLAGS, 0);
    put(uc, W_SSP, base.map_or(0, |b| b + len - 8));
    put(uc, W_SSP + 1, base.unwrap_or(0));
    put(uc, W_SSP + 2, base.map_or(0, |_| len));
    if let Some(b) = base {
        registry::replace(get(uc, W_SS_SP), b, len);
    }
}

/// The shadow stacks makecontext mapped, keyed by the context stack they were made for
/// (`uc_stack.ss_sp`; D-21.7 (5)): when makecontext is given that stack memory again, every
/// context made on it before is over (the new one's frames overwrite theirs), so the shadow
/// stack mapped for it then is unmapped. The record is the library's own (never `__ssp[1..2]`
/// of a `ucontext_t`, which a struct copy shares and fresh memory fills with garbage), so only
/// a mapping this module made is ever unmapped. An open-addressing table of `(key, base, len)`
/// slots in anonymous memory, at most half full, doubled when it would pass that; under one
/// lock (makecontext is not async-signal-safe). Left mapped: the shadow stack of stack memory
/// that is freed and never given to makecontext again (until the process ends), and one the
/// calling thread is running on.
mod registry {
    use core::sync::atomic::{AtomicUsize, Ordering::Relaxed};

    use niji_rt::sync::RawMutex;

    use super::{get, put};
    use crate::sys::CSys;

    /// Words per slot: the key (0: empty), the shadow stack's base and its length.
    const SLOT: usize = 3;
    /// Slots in the first table (one page holds them).
    const FIRST: usize = 128;

    static LOCK: RawMutex = RawMutex::new();
    /// The table's address (0 before the first use) and its slots (a power of two).
    static TABLE: AtomicUsize = AtomicUsize::new(0);
    static CAP: AtomicUsize = AtomicUsize::new(0);
    /// Slots in use.
    static COUNT: AtomicUsize = AtomicUsize::new(0);

    /// `key`'s slot in a table of `cap` slots, or the empty one where it goes (the table always
    /// has an empty slot: it is never more than half full).
    fn find(t: &[AtomicUsize], cap: usize, key: usize) -> usize {
        let mut i = ((key >> 4).wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 32) & (cap - 1);
        for _ in 0..cap {
            let k = get(t, i * SLOT);
            if k == key || k == 0 {
                break;
            }
            i = (i + 1) & (cap - 1);
        }
        i
    }

    /// Record `(base, len)` for `key` (non-zero) in a table of `cap` slots holding `count`:
    /// `Some` with the shadow stack it replaces, or `None` when `key` was new; a new key past
    /// half the slots is not recorded (`None`, nothing changed).
    pub(super) fn insert(t: &[AtomicUsize], cap: usize, count: &AtomicUsize, key: usize, base: usize, len: usize) -> Option<(usize, usize)> {
        let i = find(t, cap, key) * SLOT;
        if get(t, i) == key {
            let old = (get(t, i + 1), get(t, i + 2));
            put(t, i + 1, base);
            put(t, i + 2, len);
            return Some(old);
        }
        let n = count.load(Relaxed);
        if (n + 1) * 2 > cap {
            return None;
        }
        put(t, i, key);
        put(t, i + 1, base);
        put(t, i + 2, len);
        count.store(n + 1, Relaxed);
        None
    }

    /// Copy every slot of `old` (`old_cap` slots) into the empty table `new` of `new_cap`.
    pub(super) fn rehash(old: &[AtomicUsize], old_cap: usize, new: &[AtomicUsize], new_cap: usize) {
        for s in 0..old_cap {
            let key = get(old, s * SLOT);
            if key != 0 {
                let i = find(new, new_cap, key) * SLOT;
                put(new, i, key);
                put(new, i + 1, get(old, s * SLOT + 1));
                put(new, i + 2, get(old, s * SLOT + 2));
            }
        }
    }

    fn bytes(cap: usize) -> usize {
        (cap * SLOT * 8 + 4095) & !4095
    }

    fn map(len: usize) -> Option<usize> {
        const PROT_READ_WRITE: i64 = 3;
        const MAP_PRIVATE_ANONYMOUS: i64 = 0x22;
        let l = i64::try_from(len).ok()?;
        let a = crate::raw::syscall(crate::nr::MMAP, [0, l, PROT_READ_WRITE, MAP_PRIVATE_ANONYMOUS, -1, 0]).ok()?;
        usize::try_from(a).ok()
    }

    fn unmap(base: usize, len: usize) {
        if let (Ok(b), Ok(l)) = (i64::try_from(base), i64::try_from(len)) {
            let _ = crate::raw::syscall(crate::nr::MUNMAP, [b, l, 0, 0, 0, 0]);
        }
    }

    /// Under the lock: the table's address and slots, doubled first when one more key would
    /// pass half; when no memory can be had, the table as it is (address 0 before the first
    /// use), where a known key is still replaced and a new one goes unrecorded.
    fn room() -> (usize, usize) {
        let (t, cap) = (TABLE.load(Relaxed), CAP.load(Relaxed));
        if (COUNT.load(Relaxed) + 1) * 2 <= cap {
            return (t, cap);
        }
        let new_cap = if cap == 0 { FIRST } else { cap * 2 };
        let Some(n) = map(bytes(new_cap)) else { return (t, cap) };
        if t != 0 {
            rehash(crate::tb::words64(t, cap * SLOT), cap, crate::tb::words64(n, new_cap * SLOT), new_cap);
        }
        TABLE.store(n, Relaxed);
        CAP.store(new_cap, Relaxed);
        if t != 0 {
            unmap(t, bytes(cap));
        }
        (n, new_cap)
    }

    /// makecontext mapped the shadow stack `(base, len)` for the context stack `key`: record it
    /// and unmap the one mapped for `key` before, unless the calling thread runs on it.
    pub(super) fn replace(key: usize, base: usize, len: usize) {
        if key == 0 {
            return;
        }
        let old = {
            let _g = LOCK.lock(&CSys);
            let (t, cap) = room();
            if t == 0 {
                return;
            }
            insert(crate::tb::words64(t, cap * SLOT), cap, &COUNT, key, base, len)
        };
        if let Some((b, l)) = old {
            let ssp = crate::ffi::niji_machine_ssp();
            if b != 0 && !(b..b + l).contains(&ssp) {
                unmap(b, l);
            }
        }
    }

    /// A `fork` child: the lock, if a thread that does not exist there held it.
    pub(crate) fn after_fork_child() {
        LOCK.reset();
    }
}

pub(crate) use registry::after_fork_child;

/// `__longjmp_chk(env, val)`: aborts if the jump would land below the current stack pointer,
/// unless it leaves the alternate signal stack for the stack it interrupted (§21.1).
pub fn longjmp_chk(env: KResult<&[AtomicUsize]>, val: i32) -> ! {
    let Ok(env) = env else { crate::process::fortify_fail(b"longjmp causes uninitialized stack frame") };
    let guard = tb::current().pointer_guard.load(Relaxed);
    let target = get(env, 6).rotate_right(0x11) ^ guard;
    let here = core::ptr::from_ref(&guard).expose_provenance();
    if target < here {
        let leaving_altstack = CSys.sigaltstack_get().is_ok_and(|s| s.flags & SS_ONSTACK != 0 && (target < s.sp || target >= s.sp + s.size));
        if !leaving_altstack {
            crate::process::fortify_fail(b"longjmp causes uninitialized stack frame");
        }
    }
    ffi::niji_machine_siglongjmp(env.as_ptr().expose_provenance(), val)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use core::sync::atomic::AtomicUsize;

    use super::registry::{insert, rehash};

    fn table(cap: usize) -> Vec<AtomicUsize> {
        (0..cap * 3).map(|_| AtomicUsize::new(0)).collect()
    }

    /// §21.2, D-21.7 (5): makecontext given the same stack memory again gets back the shadow
    /// stack recorded for it (to unmap), whatever other stacks share its probe chain; a new
    /// stack is recorded once; a table at half its slots records no new stack but still
    /// replaces a known one; a doubled table keeps every record.
    #[test]
    fn makecontext_shadow_stack_registry() {
        let count = AtomicUsize::new(0);
        let t = table(128);
        // Stacks 64 KiB apart, as an allocator hands them out: 64 keys fill half the table.
        let key = |i: usize| 0x7f00_0000_0000 + i * 0x1_0000;
        for i in 0..64 {
            assert_eq!(insert(&t, 128, &count, key(i), 0x1000 * (i + 1), 0x2000), None, "stack {i} is new");
        }
        assert_eq!(count.load(core::sync::atomic::Ordering::Relaxed), 64);
        for i in 0..64 {
            assert_eq!(insert(&t, 128, &count, key(i), 0x9000 * (i + 1), 0x3000), Some((0x1000 * (i + 1), 0x2000)), "stack {i} reused");
        }
        assert_eq!(count.load(core::sync::atomic::Ordering::Relaxed), 64, "a reuse adds no slot");
        assert_eq!(insert(&t, 128, &count, key(64), 0x5000, 0x1000), None, "past half: not recorded");
        assert_eq!(insert(&t, 128, &count, key(64), 0x6000, 0x1000), None, "so not found later");
        assert_eq!(insert(&t, 128, &count, key(7), 0x8000, 0x1000), Some((0x9000 * 8, 0x3000)), "a known stack still replaces");
        let big = table(256);
        rehash(&t, 128, &big, 256);
        for i in 0..64 {
            let want = if i == 7 { (0x8000, 0x1000) } else { (0x9000 * (i + 1), 0x3000) };
            assert_eq!(insert(&big, 256, &count, key(i), 1, 1), Some(want), "stack {i} after doubling");
        }
    }
}
