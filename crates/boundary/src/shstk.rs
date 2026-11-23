//! The user shadow stack, opt-in (D-21.6): the start's decision for a static program, the lock
//! mask, and turning it off in every thread for a `permissive` `dlopen`. The rules themselves are
//! `niji_ld::shstk`'s; the dynamic start's decision is in `ld::link`.

use niji::config::ShstkMode;

/// `ARCH_SHSTK_DISABLE` and `ARCH_SHSTK_SHSTK` (kernel UAPI `asm/prctl.h`).
const ARCH_SHSTK_DISABLE: i64 = 0x5002;
const ARCH_SHSTK_SHSTK: i64 = 1;
/// `SYS_arch_prctl`.
// ARCH(x86_64): the shadow stack's interface is x86-64's arch_prctl.
const SYS_ARCH_PRCTL: i64 = 158;

/// The loader's form of a mode the tunables asked for (`Off` never asks).
#[cfg(any(feature = "ld", feature = "runtime"))]
pub(crate) fn loader_mode(m: ShstkMode) -> niji_ld::shstk::ShstkMode {
    match m {
        ShstkMode::On => niji_ld::shstk::ShstkMode::On,
        ShstkMode::Permissive => niji_ld::shstk::ShstkMode::Permissive,
        ShstkMode::Default | ShstkMode::Off => niji_ld::shstk::ShstkMode::Default,
    }
}

/// The code `ld.so` keeps for mode `m` (0 means "not asked").
#[cfg(feature = "ld")]
pub(crate) fn code(m: ShstkMode) -> u8 {
    match m {
        ShstkMode::Default | ShstkMode::Off => 1,
        ShstkMode::On => 2,
        ShstkMode::Permissive => 3,
    }
}

/// The loader's mode for a kept code, `None` for 0 (not asked).
#[cfg(feature = "ld")]
pub(crate) fn mode_of(code: u8) -> Option<niji_ld::shstk::ShstkMode> {
    match code {
        0 => None,
        2 => Some(niji_ld::shstk::ShstkMode::On),
        3 => Some(niji_ld::shstk::ShstkMode::Permissive),
        _ => Some(niji_ld::shstk::ShstkMode::Default),
    }
}

/// The `ARCH_SHSTK_LOCK` mask a start in mode `m` passes: every feature (probe-settled: the
/// reference locks `shstk` and `wrss`), or 0 (no lock) for `permissive`.
#[cfg(any(feature = "ld", feature = "runtime"))]
pub(crate) fn lock_mask(m: niji_ld::shstk::ShstkMode) -> u64 {
    if niji_ld::shstk::locks(m) { u64::MAX } else { 0 }
}

/// The static start's decision (D-21.6): the lock mask to enable with when `wanted` asks for the
/// shadow stack and the program's own image (program headers at `phdr`, `phnum` of them, its
/// `PT_GNU_PROPERTY` holding the AND of every linked input) is marked; `None` otherwise. Cold: a
/// start reaches it only when `GLIBC_TUNABLES` asks.
#[cold]
#[inline(never)]
#[cfg(feature = "runtime")]
pub(crate) fn static_start(wanted: ShstkMode, phdr: usize, phnum: usize) -> Option<u64> {
    use niji_ld::arch::Native;
    use niji_ld::elf::{pt, Elf, Span};
    const PHDR_SIZE: usize = 56;
    if phdr == 0 || phnum == 0 {
        return None;
    }
    // UNSAFE(slice): the program's headers.
    #[allow(unsafe_code)]
    // SAFETY: the kernel mapped `phnum` 56-byte program headers at AT_PHDR (⚑ KERN), readable for
    // the process's lifetime.
    let headers = unsafe { crate::conv::bytes_at(phdr, phnum.saturating_mul(PHDR_SIZE)) };
    let word = |h: &[u8], at: usize| h.get(at..at.saturating_add(8)).and_then(|b| b.try_into().ok()).map_or(0, u64::from_le_bytes);
    let entries = || headers.chunks_exact(PHDR_SIZE).map(|h| (word(h, 0) & 0xffff_ffff, word(h, 16), word(h, 32)));
    // The load bias: PT_PHDR's address against AT_PHDR (a static PIE), else 0.
    let bias = entries().find(|e| e.0 == u64::from(pt::PHDR)).map_or(0, |e| u64::try_from(phdr).unwrap_or(0).wrapping_sub(e.1));
    let (_, vaddr, filesz) = entries().find(|e| e.0 == u64::from(pt::GNU_PROPERTY))?;
    let at = usize::try_from(bias.wrapping_add(vaddr)).ok()?;
    // UNSAFE(slice): the program's property note, inside its first loaded segment.
    #[allow(unsafe_code)]
    // SAFETY: PT_GNU_PROPERTY's `p_filesz` bytes at bias + `p_vaddr` lie in a PT_LOAD the kernel mapped
    // readable (the static link places the note in the read-only segment); `from_image` checks the
    // span against the PT_LOAD table before reading it.
    let bytes = unsafe { crate::conv::bytes_at(at, usize::try_from(filesz).ok()?) };
    let spans = [Span { vaddr, bytes }];
    let elf = Elf::from_image::<Native>(headers, &spans).ok()?;
    let m = loader_mode(wanted);
    niji_ld::shstk::marked::<Native>(&elf).then(|| lock_mask(m))
}

/// `__niji_shstk_off` (D-21.6, `permissive`): turn the shadow stack off in the calling thread and
/// in every other thread the library runs (the set*id broadcast's replay, D-17.7).
#[cfg(feature = "runtime")]
pub(crate) fn off_everywhere() {
    if crate::setxid::broadcast(SYS_ARCH_PRCTL, ARCH_SHSTK_DISABLE, ARCH_SHSTK_SHSTK, 0).is_err() {
        // Not a library thread (a test process): the calling thread alone.
        let _ = crate::raw::syscall(SYS_ARCH_PRCTL, [ARCH_SHSTK_DISABLE, ARCH_SHSTK_SHSTK, 0, 0, 0, 0]);
    }
}

/// The calling thread alone (a `permissive` `dlopen` in a process without libniji's C library).
#[cfg(feature = "ld")]
pub(crate) fn off_here() {
    let _ = crate::raw::syscall(SYS_ARCH_PRCTL, [ARCH_SHSTK_DISABLE, ARCH_SHSTK_SHSTK, 0, 0, 0, 0]);
}
