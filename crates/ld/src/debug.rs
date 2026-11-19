//! Debugger rendezvous: `_r_debug`, `_dl_debug_state` (spec §27.4; plan/ph14/design.md §13, M14-9;
//! DL-23). The values a debugger and a program read from the rendezvous; the storage and the calls
//! are `niji_boundary::ld::debug`'s.
//!
//! The structure is `<link.h>`'s `struct r_debug` (40 bytes on LP64): `r_version`, `r_map` (the
//! base namespace's `link_map` list), `r_brk` (the function the loader calls around every change of
//! the list), `r_state`, `r_ldbase` (the loader's load address). Version 1 has no `r_next`; version 2
//! (`r_debug_extended`, one structure per namespace linked through `r_next`) extends it with
//! `dlmopen` (DL-22) without changing these fields.

/// `r_version` of the base namespace's rendezvous while there is one namespace (probe: the reference
/// reports 1).
pub const VERSION: i32 = 1;

/// `_r_debug`'s size and the offsets of its fields (`<link.h>`, LP64).
pub const SIZE: usize = 40;
/// Offset of `r_map`.
pub const MAP: usize = 8;
/// Offset of `r_brk`.
pub const BRK: usize = 16;
/// Offset of `r_state`.
pub const STATE: usize = 24;
/// Offset of `r_ldbase`.
pub const LDBASE: usize = 32;

/// `r_state`: what the list is doing when the loader calls `r_brk`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum State {
    /// `RT_CONSISTENT`: the list is complete and may be read.
    Consistent = 0,
    /// `RT_ADD`: objects are being added.
    Add = 1,
    /// `RT_DELETE`: objects are being removed.
    Delete = 2,
}

impl State {
    /// The `r_state` word.
    #[must_use]
    pub const fn word(self) -> i32 {
        self as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Spec s27_4_c_0: the RT_* values of <link.h>.
    #[test]
    fn states_are_the_link_h_values() {
        assert_eq!((State::Consistent.word(), State::Add.word(), State::Delete.word()), (0, 1, 2));
        assert_eq!((MAP, BRK, STATE, LDBASE, SIZE), (8, 16, 24, 32, 40));
    }
}
