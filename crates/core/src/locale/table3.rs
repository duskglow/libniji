//! The three-level tables of compiled locales (spec Appendix D.1.2): bit tables (classes) and
//! value tables (case maps, widths, collation). Every read is bounds-checked: an offset that
//! leaves the table reads as "absent".
#![allow(clippy::arithmetic_side_effects)] // offsets are u32 file values or slice lengths: sums of a few stay far below usize::MAX

use super::file::u32_at;

/// A three-level table over a byte region.
#[derive(Clone, Copy, Debug)]
pub struct Table3 {
    data: &'static [u8],
    /// The `bound` level-1 entries: one slice test covers both `i1 >= bound` and the read.
    level1: &'static [u8],
    shift1: u32,
    shift2: u32,
    mask2: u32,
    mask3: u32,
}

impl Table3 {
    /// A table with no entries: every code point is absent (not a member, value 0).
    pub const EMPTY: Table3 = Table3 { data: &[], level1: &[], shift1: 0, shift2: 0, mask2: 0, mask3: 0 };

    /// Read the header; `None` if it does not fit or its shifts are out of range.
    pub fn parse(data: &'static [u8]) -> Option<Table3> {
        let bound = usize::try_from(u32_at(data, 4)?).ok()?;
        let t = Table3 {
            data,
            level1: data.get(20..)?.get(..bound.checked_mul(4)?)?,
            shift1: u32_at(data, 0)?,
            shift2: u32_at(data, 8)?,
            mask2: u32_at(data, 12)?,
            mask3: u32_at(data, 16)?,
        };
        (t.shift1 < 32 && t.shift2 < 32).then_some(t)
    }

    /// The byte offset of the level-3 block for `wc`, if present.
    ///
    /// Each level is one checked slice read of a whole word (TIMING-COV-WCTYPE-L, D-5.7): the
    /// level-1 slice's length is `bound`, and a block offset names word `offset / 4` (a
    /// well-formed file's offsets are multiples of 4; any other offset reads the word it falls
    /// in, never outside the table).
    #[inline(always)]
    fn block(&self, wc: u32) -> Option<u32> {
        // Both shifts are below 32 ([`Table3::parse`]): a plain shift (masked to 5 bits, so it
        // equals the checked one) leaves out a test per level (STRXFRM-3).
        let l2 = word(self.level1, u64::from(wc >> (self.shift1 & 31)))?;
        if l2 == 0 {
            return None;
        }
        let l3 = word(self.data, u64::from(l2 >> 2) + u64::from((wc >> (self.shift2 & 31)) & self.mask2))?;
        if l3 == 0 { None } else { Some(l3) }
    }

    /// A bit table's membership of `wc`.
    #[inline]
    pub fn bit(&self, wc: u32) -> bool {
        let Some(l3) = self.block(wc) else { return false };
        let Some(w) = word(self.data, u64::from(l3 >> 2) + u64::from((wc >> 5) & self.mask3)) else { return false };
        (w >> (wc & 31)) & 1 != 0
    }

    /// A bit table's membership of the 32 characters from `wc & !31` on, bit `i` for character
    /// `(wc & !31) + i`: [`Table3::bit`] of each. With both shifts at least 5 the 32 take the
    /// same level-1, level-2 and level-3 slots, so this is one walk to their level-3 word
    /// (WCTYPE-TRIE-FILE-8: a class-table block in 8 walks per class instead of 256); otherwise
    /// it is 32 walks.
    pub fn bits32(&self, wc: u32) -> u32 {
        let first = wc & !31;
        if self.shift1 < 5 || self.shift2 < 5 {
            return (0..32u32).fold(0, |m, i| m | (u32::from(self.bit(first | i)) << i));
        }
        let Some(l3) = self.block(first) else { return 0 };
        word(self.data, u64::from(l3 >> 2) + u64::from((first >> 5) & self.mask3)).unwrap_or(0)
    }

    /// A value table's `i32` entry for `wc` (0 when absent).
    #[inline]
    pub fn value_i32(&self, wc: u32) -> i32 {
        let Some(l3) = self.block(wc) else { return 0 };
        word(self.data, u64::from(l3 >> 2) + u64::from(wc & self.mask3)).map_or(0, |v| i32::from_le_bytes(v.to_le_bytes()))
    }

    /// Whether [`Table3::value_i32`] of every `wc` below `expected.len() / 4` is the
    /// little-endian word at `4 * wc` of `expected`, proven a level-3 run at a time (one slice
    /// compare per run instead of a table walk per character). `false` means "not proven": an
    /// entry differs, or a run could not be compared whole (absent block, irregular masks, short
    /// data); the caller then compares per character, which gives the exact answer.
    pub fn runs_equal(&self, expected: &[u8]) -> bool {
        let n = expected.len() / 4;
        let (Some(run), Some(step2)) = (usize::try_from(self.mask3).ok().and_then(|m| m.checked_add(1)), 1usize.checked_shl(self.shift2)) else { return false };
        // Within one chunk below, `wc >> shift2` (so the level-2 slot and, with
        // `shift1 >= shift2`, the level-1 slot) is fixed and `wc & mask3` counts up by one.
        if !run.is_power_of_two() || self.shift1 < self.shift2 {
            return false;
        }
        let mut wc = 0usize;
        while wc < n {
            let len = (n - wc).min(run - wc % run).min(step2 - wc % step2);
            let Ok(w) = u32::try_from(wc) else { return false };
            // The block's word as `value_i32` reads it (an offset rounded down to a word).
            let Some(l3) = self.block(w).and_then(|b| usize::try_from(b & !3).ok()) else { return false };
            let at = l3.saturating_add(4 * (wc & (run - 1)));
            let (Some(have), Some(want)) = (self.data.get(at..at.saturating_add(4 * len)), expected.get(4 * wc..4 * (wc + len))) else { return false };
            if have != want {
                return false;
            }
            wc += len;
        }
        true
    }

    /// A value table's `u32` entry for `wc` (`None` when its block is absent).
    pub fn value_u32(&self, wc: u32) -> Option<u32> {
        let l3 = self.block(wc)?;
        word(self.data, u64::from(l3 >> 2) + u64::from(wc & self.mask3))
    }

    /// A value table's byte entry for `wc` (`None` when absent).
    #[inline]
    pub fn value_u8(&self, wc: u32) -> Option<u8> {
        let l3 = usize::try_from(self.block(wc)?).ok()?;
        let i = usize::try_from(wc & self.mask3).ok()?;
        self.data.get(l3.checked_add(i)?).copied()
    }
}

/// Little-endian word `i` of `b` (bytes `4 * i` on; a `u64` index: the sum of a shifted
/// offset and an in-block index cannot overflow it, nor can four times it); `None` past the end.
#[inline(always)]
fn word(b: &[u8], i: u64) -> Option<u32> {
    u32_at(b, usize::try_from(i.checked_mul(4)?).ok()?)
}

#[cfg(test)]
mod tests {
    use super::Table3;

    /// no spec clause (App. D.1.2's table shape; WCTYPE-TRIE-FILE-8): `bits32` is `bit` of each of
    /// its 32 characters for a table whose shifts are below 5, where two characters of one
    /// 32-character run take different level-3 words: shift1 = shift2 = 4, mask2 = mask3 = 0,
    /// so characters 0-15 read bits 0-15 of one word and 16-31 bits 16-31 of another, whose
    /// other half is set to catch a read of the wrong word.
    #[test]
    fn bits32_is_bit_of_each_character_with_small_shifts() {
        let words: [u32; 13] = [
            4, 4, 4, 0, 0, // shift1, bound, shift2, mask2, mask3
            36, 40, 44, 48, // level 1: the level-2 tables (byte offsets)
            52, 56, 60, 64, // level 2: one level-3 block each
        ];
        let l3: [u32; 4] = [0x0000_a5a5, 0x5a5a_ffff, 0xffff_00f0, 0x8001_0000];
        let bytes: Vec<u8> = words.iter().chain(&l3).flat_map(|w| w.to_le_bytes()).collect();
        let t = Table3::parse(Vec::leak(bytes)).unwrap();
        assert_eq!(t.bits32(0), 0x5a5a_a5a5);
        assert_eq!(t.bits32(40), 0x8001_00f0);
        for wc in 0..80u32 {
            let each = (0..32u32).fold(0u32, |m, i| m | (u32::from(t.bit((wc & !31) | i)) << i));
            assert_eq!(t.bits32(wc), each, "{wc}");
        }
    }
}
