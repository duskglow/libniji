//! Constants computed at run time and cached at the largest precision requested so far.
//!
//! * pi by Machin's formula `pi = 16 atan(1/5) - 4 atan(1/239)`;
//! * ln 2 by `ln 2 = 2 atanh(1/3) = sum 2 / ((2k+1) 3^(2k+1))`;
//! * ln 10 by `ln 10 = (10 ln 2 - ln(128/125)) / 3` (`2^10 = 1000 · 128/125`), with the cached
//!   ln 2 and `ln(128/125) = 2 atanh(3/253) = sum 2 · 3^(2k+1) / ((2k+1) 253^(2k+1))`
//!   (about 12.8 bits per term, against 3.2 for `atanh(1/3)` and 6.3 for `atanh(1/9)`);
//! * 2/pi from pi with the general division.
//!
//! The series are summed in fixed point with `W` fraction bits. Every term is a truncated
//! quotient, so its error is below 2.2 units: the running power `p` is truncated once per step
//! (`p' = floor(p a^2 / b^2)` from `p = floor(2^W a / b)`), so it carries at most
//! `1 / (1 - a^2/b^2)` units of error (1.125 for `1/3`, 1.0002 for `3/253`), which the division
//! by `2k+1` passes on (k = 0) or shrinks, plus one unit of truncation. The series stops when
//! `p` reaches zero, so the dropped tail is below `1.2` units. A sum of `T < W/4` terms (the
//! slowest series, `atanh(1/3)`, gains 3.17 bits per term) is then within `0.6 W + 2` units;
//! with the factors (16 and 4 for pi, 2 for ln 2) the total is below `9 W` units, and pi and
//! ln 2 exceed 1/2, so `W = bits + 40` leaves them within `2^-(bits + 5)` relative for any
//! `bits < 2^30`. ln 10's error is proved at `compute_ln10`.

use std::sync::Mutex;

use super::nat::Nat;
use super::{Big, Round};

type Cache = Mutex<Option<(u32, Big)>>;

static PI: Cache = Mutex::new(None);
static LN2: Cache = Mutex::new(None);
static LN10: Cache = Mutex::new(None);
static TWO_OVER_PI: Cache = Mutex::new(None);

/// A value of the constant with relative error below `2^-w`, truncated to `w + 2` bits.
fn cached(cache: &Cache, w: u32, compute: fn(u32) -> Big) -> Big {
    let need = w.saturating_add(4);
    {
        let g = match cache.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some((bits, v)) = g.as_ref() {
            if *bits >= need {
                // Cached error < 2^-(w+4); truncation adds < 2^-(w+1).
                return v.round(w.saturating_add(2), Round::Zero);
            }
        }
    }
    // Grow geometrically so a slowly rising demand costs O(1) recomputations.
    let old = match cache.lock() {
        Ok(g) => g.as_ref().map_or(0, |c| c.0),
        Err(p) => p.into_inner().as_ref().map_or(0, |c| c.0),
    };
    let bits = need.max(old.saturating_mul(2)).max(256);
    let v = compute(bits);
    let r = v.round(w.saturating_add(2), Round::Zero);
    let mut g = match cache.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    if g.as_ref().is_none_or(|c| c.0 < bits) {
        *g = Some((bits, v));
    }
    r
}

/// `sum_k (-1)^k 2^W / ((2k+1) n^(2k+1))` (or without the signs), in fixed point.
fn atan_inv(n: u64, wbits: u64, alternate: bool) -> (Nat, Nat) {
    atan_frac(1, n, wbits, alternate)
}

/// `sum_k (-1)^k 2^W a^(2k+1) / ((2k+1) b^(2k+1))` (or without the signs), in fixed point;
/// `a < b < 2^32`.
fn atan_frac(a: u64, b: u64, wbits: u64, alternate: bool) -> (Nat, Nat) {
    let (a2, b2) = (a * a, b * b);
    let mut p = Nat::from_u64(a).shl(wbits).divrem_small(b).0;
    let (mut pos, mut neg) = (Nat::zero(), Nat::zero());
    let mut k: u64 = 0;
    while !p.is_zero() {
        let t = p.divrem_small(2 * k + 1).0;
        if alternate && k % 2 == 1 {
            neg = neg.add(&t);
        } else {
            pos = pos.add(&t);
        }
        if a2 != 1 {
            p = p.mul_small(a2);
        }
        p = p.divrem_small(b2).0;
        k += 1;
    }
    (pos, neg)
}

fn compute_pi(bits: u32) -> Big {
    let w = u64::from(bits) + 40;
    let (p5, n5) = atan_inv(5, w, true);
    let (p239, n239) = atan_inv(239, w, true);
    // 16 (p5 - n5) - 4 (p239 - n239), all as one non-negative combination.
    let plus = p5.mul_small(16).add(&n239.mul_small(4));
    let minus = n5.mul_small(16).add(&p239.mul_small(4));
    Big::fin(false, plus.sub(&minus), -(w as i64))
}

fn compute_ln2(bits: u32) -> Big {
    let w = u64::from(bits) + 40;
    let (s, _) = atan_inv(3, w, false);
    Big::fin(false, s.shl(1), -(w as i64))
}

/// ln 10 within `2^-(bits + 6)` relative.
///
/// `ln 10 = (10 L - R) / 3` with `L = ln2_w(bits + 8)` and `R = 2 atanh(3/253)` summed with
/// `W = bits + 40`. Absolute errors: `10 |L - ln 2| < 10 · 0.7 · 2^-(bits+8) = 0.028 · 2^-bits`;
/// `R` (at most `0.6 W + 2` units, doubled) below `2^-(bits+8)` for `bits < 2^30`; so the
/// numerator (6.91) is within `0.032 · 2^-bits`, the exact quotient by 3 within
/// `0.011 · 2^-bits`, that is `0.005 · 2^-bits` relative to ln 10 (2.30). The division
/// truncated to `bits + 8` bits adds below `2^-(bits+7)` relative: in all below `0.013 ·
/// 2^-bits < 2^-(bits+6)`.
fn compute_ln10(bits: u32) -> Big {
    let w = u64::from(bits) + 40;
    let l2 = ln2_w(bits.saturating_add(8));
    let (s, _) = atan_frac(3, 253, w, false);
    let r = Big::fin(false, s.shl(1), -(w as i64));
    l2.mul(&Big::from_i64(10)).sub(&r).div(&Big::from_i64(3), bits.saturating_add(8), Round::Zero)
}

fn compute_two_over_pi(bits: u32) -> Big {
    Big::from_i64(2).div(&pi_w(bits.saturating_add(4)), bits.saturating_add(4), Round::Zero)
}

/// pi with relative error below `2^-w`.
pub(crate) fn pi_w(w: u32) -> Big {
    cached(&PI, w, compute_pi)
}

/// ln 2 with relative error below `2^-w`.
pub(crate) fn ln2_w(w: u32) -> Big {
    cached(&LN2, w, compute_ln2)
}

/// ln 10 with relative error below `2^-w`.
pub(crate) fn ln10_w(w: u32) -> Big {
    cached(&LN10, w, compute_ln10)
}

/// 2/pi with relative error below `2^-w`.
pub(crate) fn two_over_pi_w(w: u32) -> Big {
    cached(&TWO_OVER_PI, w, compute_two_over_pi)
}

/// Rounds a constant correctly: `f(w)` must have relative error below `2^-w`. Ziv's loop; it
/// ends because the constants are irrational.
fn round_const(prec: u32, f: fn(u32) -> Big) -> Big {
    let mut w = prec.saturating_add(32);
    loop {
        let v = f(w);
        let d = v.mul_pow2(-i64::from(w));
        let lo = v.sub(&d).round(prec, Round::Nearest);
        let hi = v.add(&d).round(prec, Round::Nearest);
        if lo.identical(&hi) || w >= prec.saturating_mul(8).saturating_add(4096) {
            return hi;
        }
        w = w.saturating_mul(2);
    }
}

/// pi correctly rounded to nearest at `prec` bits.
pub fn pi(prec: u32) -> Big {
    round_const(prec, pi_w)
}

/// ln 2 correctly rounded to nearest at `prec` bits.
pub fn ln2(prec: u32) -> Big {
    round_const(prec, ln2_w)
}

/// ln 10 correctly rounded to nearest at `prec` bits.
pub fn ln10(prec: u32) -> Big {
    round_const(prec, ln10_w)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use crate::mp::func::log_core;

    /// `floor(ln 10 · 2^4400) · 2^-4400` (within `2^-4400` below ln 10). Computed outside this
    /// crate by two independent methods that agree: Python's `decimal` module (`Decimal(10).ln()`
    /// at 1360 digits) and exact integer sums of `ln 2 = 2 atanh(1/3)` with
    /// `ln 5/4 = 2 atanh(1/9)`.
    const LN10_4400: &str = concat!(
        "0x",
        "24d763776aaa2b05ba95b58ae0b4c28a38a3fb3e76977e43a0f187a0807c0b5ca58bc0b5ec6a0417331c32f00b17c35a",
        "0b1889061042f8b6bee3de2100b945b59e0b3e28a2a324479d96a9b0ec360c7efbd9b3ac12acf1be94586ed2748671ee",
        "f299ecd6c8d8142163a4cda3511e2713d6c22c15f57b7883d1a7a963a4c17a607891e3f2ab4ebba627356d0b9a89c586",
        "691fb2c9a5e31753f6c74a3a95f53f703902fcf30785049a915d973789a0ce76fd1fea5b7ac9c4182be2121baa6dd007",
        "8f7f4f145d239b5b8e12323497ebc6f2ba2011fc5ec366d42a527aaab7da7a297ddf8dd813a50e5838e295c03ff78b6c",
        "6b5afefff6086e82932c119b586e9923bbe672397da5d3cd8e60a9e3291777f20250985e06449e9b8edc3f368b5ccb51",
        "ed792c7230396842aa9981294c93b0f72f193aa01b86615987c74d9b08198e0d1357a10fc8190ae5c298b46391e3def5",
        "63f3420c929ecca9b7be16817ad58c8e9087bd782c010428283670981e52a5dbfdbfcd8f6c02daccfd0b1637be28f14b",
        "7b5a6c4f70680dcc2d94937063059fa1a675b483a8b7bf4af401be2d2f85168ab7cf32ecc62769276f0498722cf936ab",
        "5a8db3b32cb56b96e26fdd4bed77ecd4ffcc1971033f3bb206f96a6a59ff7715f12e0cc0401afc1892d03b5e95b1b373",
        "68b5bfa44e23a78f922a4fd7d78edbad2e071296d4f527a9a399a2baaea0f1fe3ed7c1b0d62b23dc3ea763166315d140",
        "aebd5325b212447c0120b4254682bc08aaeeac203b25f",
        "p-4400",
    );

    fn reference() -> Big {
        Big::parse(LN10_4400).expect("reference parses")
    }

    /// `|v - r| + 2^-4400 < r · 2^-e`: within `2^-e` of ln 10, the reference's own error included.
    fn within(v: &Big, r: &Big, e: i64) -> bool {
        let d = v.sub(r).abs().add(&Big::pow2(-4400));
        d.cmp(&r.mul_pow2(-e)) == Some(core::cmp::Ordering::Less)
    }

    #[test]
    fn ln10_series_and_logarithm_within_their_bounds() {
        // no spec clause: the oracle's constants (D-10.12); compute_ln10 promises 2^-(bits+6)
        // relative, the general logarithm (the previous source of ln 10) 2^-w.
        let r = reference();
        assert!(r.to_f64() == core::f64::consts::LN_10);
        for bits in [53u32, 256, 1000, 2048, 4300] {
            assert!(within(&compute_ln10(bits), &r, i64::from(bits) + 6), "series ln 10 at {bits} bits");
            let w = bits + 8;
            assert!(within(&log_core(&Big::from_i64(10), w), &r, i64::from(w)), "log 10 at {w} bits");
        }
    }

    #[test]
    fn ln10_series_agrees_with_the_logarithm_at_high_precision() {
        // no spec clause: the oracle's constants (D-10.12); two methods sharing only ln 2:
        // within 2^-(b+6) + 2^-(b+8) < 2^-(b+5) relative of each other.
        let b = 20000u32;
        let s = compute_ln10(b);
        let l = log_core(&Big::from_i64(10), b + 8);
        let d = s.sub(&l).abs();
        assert!(d.cmp(&l.mul_pow2(-i64::from(b) - 5)) == Some(core::cmp::Ordering::Less));
    }
}
