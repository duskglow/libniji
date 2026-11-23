//! The user shadow stack's object rules (D-21.6): which objects are marked for it, whether a start
//! may enable it, and what a `dlopen` of an unmarked object does once it is on. The boundary makes
//! the system calls; these are the decisions, kept here so that a host without the CPU feature
//! still tests them.
//!
//! Marking: x86-64 psABI §2.1.7 ("Program Property"), the `GNU_PROPERTY_X86_FEATURE_1_AND`
//! property of a `PT_GNU_PROPERTY` note, bit `GNU_PROPERTY_X86_FEATURE_1_SHSTK`. An object with no
//! such note, or with a malformed one, is unmarked.

use crate::arch::Arch;
use crate::arch::ELFDATA2LSB;
use crate::elf::{pt, Elf};

/// How the shadow stack, once on, treats unmarked objects (`glibc.cpu.x86_shstk`, parsed by
/// `niji::config::ShstkMode`, whose `off` never reaches the loader's rules).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShstkMode {
    /// Lock; refuse an unmarked `dlopen`.
    Default,
    /// Lock; load an unmarked object with the shadow stack kept on.
    On,
    /// Do not lock; turn the shadow stack off for an unmarked object.
    Permissive,
}

/// `GNU_PROPERTY_X86_FEATURE_1_AND` (psABI §2.1.7): a 32-bit word whose bits are AND-ed across a
/// link's inputs.
const FEATURE_1_AND: u32 = 0xc000_0002;
/// Its bit `GNU_PROPERTY_X86_FEATURE_1_SHSTK`.
const FEATURE_1_SHSTK: u32 = 1 << 1;

/// Whether `elf` is marked as compatible with the shadow stack.
pub fn marked<A: Arch>(elf: &Elf<'_>) -> bool {
    let Some(ph) = elf.find(pt::GNU_PROPERTY) else { return false };
    let Ok(notes) = elf.notes(&ph) else { return false };
    let word = |d: &[u8]| {
        let b: [u8; 4] = d.get(..4)?.try_into().ok()?;
        Some(if A::EI_DATA == ELFDATA2LSB { u32::from_le_bytes(b) } else { u32::from_be_bytes(b) })
    };
    notes
        .map_while(Result::ok)
        .flat_map(|n| n.properties().map_while(Result::ok))
        .any(|p| p.pr_type == FEATURE_1_AND && word(p.data).is_some_and(|w| w & FEATURE_1_SHSTK != 0))
}

/// Whether a start whose objects are `objs` (the program, every `DT_NEEDED` object and every
/// preload; the interpreter and the C library among them when they are named) may enable the
/// shadow stack: every one of them is marked.
pub fn start_allowed<'e, 'a: 'e, A: Arch>(mut objs: impl Iterator<Item = &'e Elf<'a>>) -> bool {
    objs.all(|e| marked::<A>(e))
}

/// What a `dlopen` does with an object while the calling thread's shadow stack is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OnDlopen {
    /// Load it.
    Load,
    /// Refuse it (`Refusal::Shstk`); the shadow stack stays on.
    Refuse,
    /// Turn the shadow stack off in every thread, then load it.
    Disable,
}

/// The rule for an object `marked` (or not) under `mode` (the reference's behaviour,
/// plan/reviews/SHSTK-SCOPE.md §2, probe-settled): a marked object always loads; an unmarked one
/// is refused by default, loads with the shadow stack kept under `on`, and loads after the shadow
/// stack is turned off under `permissive`.
pub fn on_dlopen(mode: ShstkMode, marked: bool) -> OnDlopen {
    match (marked, mode) {
        (true, _) | (false, ShstkMode::On) => OnDlopen::Load,
        (false, ShstkMode::Default) => OnDlopen::Refuse,
        (false, ShstkMode::Permissive) => OnDlopen::Disable,
    }
}

/// Whether a start in `mode` locks the shadow stack once enabled (every mode but `permissive`,
/// which may have to turn it off later; probe-settled).
pub fn locks(mode: ShstkMode) -> bool {
    mode != ShstkMode::Permissive
}

#[cfg(test)]
mod tests {
    use super::{locks, on_dlopen, OnDlopen, ShstkMode};

    // D-21.6 (no spec clause: the reference's dlopen behaviour, probe-settled): the policy table.
    #[test]
    fn dlopen_policy() {
        for m in [ShstkMode::Default, ShstkMode::On, ShstkMode::Permissive] {
            assert_eq!(on_dlopen(m, true), OnDlopen::Load);
        }
        assert_eq!(on_dlopen(ShstkMode::Default, false), OnDlopen::Refuse);
        assert_eq!(on_dlopen(ShstkMode::On, false), OnDlopen::Load);
        assert_eq!(on_dlopen(ShstkMode::Permissive, false), OnDlopen::Disable);
        assert!(locks(ShstkMode::Default) && locks(ShstkMode::On) && !locks(ShstkMode::Permissive));
    }
}
