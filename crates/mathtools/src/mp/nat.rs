//! Unsigned big integers: little-endian `u64` limbs with no high zero limbs, so every value has
//! exactly one representation and `Vec` equality is numeric equality.
//!
//! Karatsuba multiplication above a threshold, schoolbook below; division is Knuth's Algorithm D (TAOCP vol. 2, 4.3.1); the square
//! root is Zimmermann's Karatsuba square root (D-10.130); the cube root is Newton's
//! iteration from above, started from a floating-point estimate.

use std::cmp::Ordering;

/// Factors with at least this many limbs each are multiplied by Karatsuba's method.
const KARATSUBA_MIN: usize = 40;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Nat {
    d: Vec<u64>,
}

impl Nat {
    pub(crate) fn zero() -> Nat {
        Nat { d: Vec::new() }
    }

    pub(crate) fn from_u64(v: u64) -> Nat {
        let mut n = Nat { d: vec![v] };
        n.trim();
        n
    }

    pub(crate) fn from_u128(v: u128) -> Nat {
        let mut n = Nat { d: vec![v as u64, (v >> 64) as u64] };
        n.trim();
        n
    }

    fn from_limbs(d: Vec<u64>) -> Nat {
        let mut n = Nat { d };
        n.trim();
        n
    }

    fn trim(&mut self) {
        while self.d.last() == Some(&0) {
            self.d.pop();
        }
    }

    pub(crate) fn is_zero(&self) -> bool {
        self.d.is_empty()
    }

    pub(crate) fn is_one(&self) -> bool {
        self.d.len() == 1 && self.d[0] == 1
    }

    pub(crate) fn is_odd(&self) -> bool {
        self.d.first().is_some_and(|&l| l & 1 == 1)
    }

    /// Number of significant bits (0 for zero).
    pub(crate) fn bit_len(&self) -> u64 {
        match self.d.last() {
            None => 0,
            Some(&t) => self.d.len() as u64 * 64 - u64::from(t.leading_zeros()),
        }
    }

    /// Number of trailing zero bits (0 for zero).
    pub(crate) fn trailing_zeros(&self) -> u64 {
        for (i, &l) in self.d.iter().enumerate() {
            if l != 0 {
                return i as u64 * 64 + u64::from(l.trailing_zeros());
            }
        }
        0
    }

    pub(crate) fn bit(&self, i: u64) -> bool {
        let li = i / 64;
        if li >= self.d.len() as u64 {
            return false;
        }
        (self.d[li as usize] >> (i % 64)) & 1 == 1
    }

    /// True if any of the bits below position `k` is set.
    pub(crate) fn low_bits_nonzero(&self, k: u64) -> bool {
        let full = (k / 64).min(self.d.len() as u64) as usize;
        if self.d[..full].iter().any(|&l| l != 0) {
            return true;
        }
        let r = k % 64;
        if (full as u64) < k / 64 || full >= self.d.len() || r == 0 {
            return false;
        }
        self.d[full] & ((1u64 << r) - 1) != 0
    }

    pub(crate) fn to_u128(&self) -> Option<u128> {
        match self.d.len() {
            0 => Some(0),
            1 => Some(u128::from(self.d[0])),
            2 => Some(u128::from(self.d[0]) | (u128::from(self.d[1]) << 64)),
            _ => None,
        }
    }

    pub(crate) fn cmp(&self, o: &Nat) -> Ordering {
        if self.d.len() != o.d.len() {
            return self.d.len().cmp(&o.d.len());
        }
        for (a, b) in self.d.iter().rev().zip(o.d.iter().rev()) {
            if a != b {
                return a.cmp(b);
            }
        }
        Ordering::Equal
    }

    pub(crate) fn add(&self, o: &Nat) -> Nat {
        let (a, b) = if self.d.len() >= o.d.len() { (self, o) } else { (o, self) };
        let mut r = Vec::with_capacity(a.d.len() + 1);
        let mut carry = false;
        for (i, &x) in a.d.iter().enumerate() {
            let y = b.d.get(i).copied().unwrap_or(0);
            let (s1, c1) = x.overflowing_add(y);
            let (s2, c2) = s1.overflowing_add(u64::from(carry));
            r.push(s2);
            carry = c1 || c2;
        }
        if carry {
            r.push(1);
        }
        Nat::from_limbs(r)
    }

    pub(crate) fn add_small(&self, v: u64) -> Nat {
        self.add(&Nat::from_u64(v))
    }

    /// `self - o`, saturating at zero (callers only subtract a smaller value).
    pub(crate) fn sub(&self, o: &Nat) -> Nat {
        if self.cmp(o) != Ordering::Greater {
            return Nat::zero();
        }
        let mut r = Vec::with_capacity(self.d.len());
        let mut borrow = false;
        for (i, &x) in self.d.iter().enumerate() {
            let y = o.d.get(i).copied().unwrap_or(0);
            let (s1, b1) = x.overflowing_sub(y);
            let (s2, b2) = s1.overflowing_sub(u64::from(borrow));
            r.push(s2);
            borrow = b1 || b2;
        }
        Nat::from_limbs(r)
    }

    /// The product: Karatsuba's method above `KARATSUBA_MIN` limbs in both factors (Knuth, TAOCP
    /// vol. 2, 4.3.3; D-10.130), schoolbook below.
    pub(crate) fn mul(&self, o: &Nat) -> Nat {
        if self.d.len().min(o.d.len()) < KARATSUBA_MIN {
            return self.mul_schoolbook(o);
        }
        // Split at h limbs: x = x1 B^h + x0 with B = 2^64.
        let h = self.d.len().max(o.d.len()) / 2;
        let (long, short) = if self.d.len() >= o.d.len() { (self, o) } else { (o, self) };
        if short.d.len() <= h {
            // Unbalanced: x0 y + (x1 y) B^h, each half split again as needed.
            return long.low_limbs(h).mul(short).add(&long.shr(64 * h as u64).mul(short).shl(64 * h as u64));
        }
        let (a0, a1) = (self.low_limbs(h), self.shr(64 * h as u64));
        let (b0, b1) = (o.low_limbs(h), o.shr(64 * h as u64));
        let z0 = a0.mul(&b0);
        let z2 = a1.mul(&b1);
        // (a0 + a1)(b0 + b1) = z0 + z2 + a0 b1 + a1 b0 >= z0 + z2.
        let z1 = a0.add(&a1).mul(&b0.add(&b1)).sub(&z0).sub(&z2);
        z0.add(&z1.shl(64 * h as u64)).add(&z2.shl(128 * h as u64))
    }

    /// `self mod B^h`, `B = 2^64`.
    fn low_limbs(&self, h: usize) -> Nat {
        Nat::from_limbs(self.d[..h.min(self.d.len())].to_vec())
    }

    fn mul_schoolbook(&self, o: &Nat) -> Nat {
        if self.is_zero() || o.is_zero() {
            return Nat::zero();
        }
        let (a, b) = (&self.d, &o.d);
        let mut r = vec![0u64; a.len() + b.len()];
        for (i, &ai) in a.iter().enumerate() {
            if ai == 0 {
                continue;
            }
            let mut carry: u128 = 0;
            for (j, &bj) in b.iter().enumerate() {
                // (2^64-1)^2 + 2(2^64-1) = 2^128-1: no overflow.
                let t = u128::from(ai) * u128::from(bj) + u128::from(r[i + j]) + carry;
                r[i + j] = t as u64;
                carry = t >> 64;
            }
            r[i + b.len()] = carry as u64;
        }
        Nat::from_limbs(r)
    }

    pub(crate) fn mul_small(&self, m: u64) -> Nat {
        if m == 0 || self.is_zero() {
            return Nat::zero();
        }
        let mut r = Vec::with_capacity(self.d.len() + 1);
        let mut carry: u128 = 0;
        for &x in &self.d {
            let t = u128::from(x) * u128::from(m) + carry;
            r.push(t as u64);
            carry = t >> 64;
        }
        r.push(carry as u64);
        Nat::from_limbs(r)
    }

    pub(crate) fn shl(&self, k: u64) -> Nat {
        if self.is_zero() {
            return Nat::zero();
        }
        let limbs = (k / 64) as usize;
        let bits = (k % 64) as u32;
        let mut r = vec![0u64; limbs];
        r.reserve(self.d.len() + 1);
        if bits == 0 {
            r.extend_from_slice(&self.d);
        } else {
            let mut prev = 0u64;
            for &x in &self.d {
                r.push((x << bits) | (prev >> (64 - bits)));
                prev = x;
            }
            r.push(prev >> (64 - bits));
        }
        Nat::from_limbs(r)
    }

    pub(crate) fn shr(&self, k: u64) -> Nat {
        let limbs = k / 64;
        if limbs >= self.d.len() as u64 {
            return Nat::zero();
        }
        let src = &self.d[limbs as usize..];
        let bits = (k % 64) as u32;
        if bits == 0 {
            return Nat::from_limbs(src.to_vec());
        }
        let mut r = Vec::with_capacity(src.len());
        for (i, &x) in src.iter().enumerate() {
            let hi = src.get(i + 1).copied().unwrap_or(0);
            r.push((x >> bits) | (hi << (64 - bits)));
        }
        Nat::from_limbs(r)
    }

    /// Quotient and remainder by a single limb; division by zero gives (0, 0).
    pub(crate) fn divrem_small(&self, v: u64) -> (Nat, u64) {
        if v == 0 {
            return (Nat::zero(), 0);
        }
        let mut q = vec![0u64; self.d.len()];
        let mut rem: u128 = 0;
        for i in (0..self.d.len()).rev() {
            let cur = (rem << 64) | u128::from(self.d[i]);
            q[i] = (cur / u128::from(v)) as u64;
            rem = cur % u128::from(v);
        }
        (Nat::from_limbs(q), rem as u64)
    }

    /// Quotient and remainder (Knuth, Algorithm D). Division by zero gives (0, self).
    pub(crate) fn divrem(&self, v: &Nat) -> (Nat, Nat) {
        if v.is_zero() {
            return (Nat::zero(), self.clone());
        }
        if self.cmp(v) == Ordering::Less {
            return (Nat::zero(), self.clone());
        }
        if v.d.len() == 1 {
            let (q, r) = self.divrem_small(v.d[0]);
            return (q, Nat::from_u64(r));
        }
        let n = v.d.len();
        let m = self.d.len() - n;
        // Normalize so the divisor's top limb has its high bit set; then each quotient
        // estimate from the top two limbs is at most 2 too large.
        let s = u64::from(v.d[n - 1].leading_zeros());
        let vn = v.shl(s).d;
        let mut un = self.shl(s).d;
        un.resize(self.d.len() + 1, 0);
        let b: u128 = 1 << 64;
        let mut q = vec![0u64; m + 1];
        for j in (0..=m).rev() {
            let num = (u128::from(un[j + n]) << 64) | u128::from(un[j + n - 1]);
            let top = u128::from(vn[n - 1]);
            let mut qhat = num / top;
            let mut rhat = num % top;
            while qhat >= b || qhat * u128::from(vn[n - 2]) > ((rhat << 64) | u128::from(un[j + n - 2])) {
                qhat -= 1;
                rhat += top;
                if rhat >= b {
                    break;
                }
            }
            // Multiply and subtract qhat * vn from un[j..=j+n].
            let mut borrow: u64 = 0;
            let mut carry: u128 = 0;
            for i in 0..n {
                let p = qhat * u128::from(vn[i]) + carry;
                carry = p >> 64;
                let (t1, b1) = un[i + j].overflowing_sub(p as u64);
                let (t2, b2) = t1.overflowing_sub(borrow);
                un[i + j] = t2;
                borrow = u64::from(b1) + u64::from(b2);
            }
            let (t1, b1) = un[j + n].overflowing_sub(carry as u64);
            let (t2, b2) = t1.overflowing_sub(borrow);
            un[j + n] = t2;
            if b1 || b2 {
                // qhat was one too large: add the divisor back.
                qhat -= 1;
                let mut c = false;
                for i in 0..n {
                    let (s1, c1) = un[i + j].overflowing_add(vn[i]);
                    let (s2, c2) = s1.overflowing_add(u64::from(c));
                    un[i + j] = s2;
                    c = c1 || c2;
                }
                un[j + n] = un[j + n].wrapping_add(u64::from(c));
            }
            q[j] = qhat as u64;
        }
        un.truncate(n);
        (Nat::from_limbs(q), Nat::from_limbs(un).shr(s))
    }

    pub(crate) fn pow(&self, mut e: u64) -> Nat {
        let mut base = self.clone();
        let mut r = Nat::from_u64(1);
        while e > 0 {
            if e & 1 == 1 {
                r = r.mul(&base);
            }
            e >>= 1;
            if e > 0 {
                base = base.mul(&base);
            }
        }
        r
    }

    /// `self mod 2^k`.
    fn low_bits(&self, k: u64) -> Nat {
        let full = (k / 64) as usize;
        if full >= self.d.len() {
            return self.clone();
        }
        let mut d = self.d[..full].to_vec();
        let r = k % 64;
        if r > 0 {
            d.push(self.d[full] & ((1u64 << r) - 1));
        }
        Nat::from_limbs(d)
    }

    /// `(s, r)` with `s = floor(sqrt(self))` and `r = self - s^2` (so `0 <= r <= 2s`).
    ///
    /// Zimmermann's Karatsuba square root (P. Zimmermann, "Karatsuba Square Root", INRIA
    /// RR-3805, 1999; R. Brent and P. Zimmermann, *Modern Computer Arithmetic*, Algorithm 1.12
    /// SqrtRem), on bit fields instead of words: with `n = a3 2^3k + a2 2^2k + a1 2^k + a0`,
    /// `a2, a1, a0 < 2^k` and `a3 >= 2^(k-2)`, take `(s', r')` = the root and remainder of
    /// `a3 2^k + a2`, then `(q, u) = divrem(r' 2^k + a1, 2s')`, `s = s' 2^k + q` and
    /// `r = u 2^k + a0 - q^2`. The paper bounds the error of `s` to one too large (one
    /// correction); the loops below correct any error, so the result never depends on that bound.
    /// The only divisions are a half-size one per level, against Newton's full-size one per
    /// iteration.
    pub(crate) fn sqrtrem(&self) -> (Nat, Nat) {
        let len = self.bit_len();
        if len <= 128 {
            let n = self.to_u128().unwrap_or(0);
            let s = isqrt_u128(n);
            return (Nat::from_u128(s), Nat::from_u128(n - s * s));
        }
        // 4k <= len + 1 makes a3 = n >> 3k at least 2^(k-2), so s' >= 2^(k-1) as the method needs.
        let k = (len + 1) / 4;
        let (s1, r1) = self.shr(2 * k).sqrtrem();
        let a1 = self.shr(k).low_bits(k);
        let a0 = self.low_bits(k);
        let (q, u) = r1.shl(k).add(&a1).divrem(&s1.shl(1));
        let mut s = s1.shl(k).add(&q);
        let a = u.shl(k).add(&a0);
        let q2 = q.mul(&q);
        let mut r = if a.cmp(&q2) != Ordering::Less {
            a.sub(&q2)
        } else {
            // r = a - q^2 < 0: s is too large; each step down adds 2s - 1 (the new 2s + 1).
            let mut deficit = q2.sub(&a);
            loop {
                s = s.sub(&Nat::from_u64(1));
                let step = s.shl(1).add_small(1);
                if step.cmp(&deficit) != Ordering::Less {
                    break step.sub(&deficit);
                }
                deficit = deficit.sub(&step);
            }
        };
        // r > 2s: s is too small (not reached when the bound holds).
        while r.cmp(&s.shl(1)) == Ordering::Greater {
            r = r.sub(&s.shl(1).add_small(1));
            s = s.add_small(1);
        }
        (s, r)
    }

    /// `floor(cbrt(self))`, by Newton's iteration `x <- (2x + n/x^2)/3` from above (by the
    /// AM-GM inequality each iterate stays `>= floor(cbrt(n))`).
    pub(crate) fn icbrt(&self) -> Nat {
        if self.is_zero() {
            return Nat::zero();
        }
        let s0 = self.bit_len().saturating_sub(100);
        let s = s0 + (3 - s0 % 3) % 3;
        let t = self.shr(s).to_u128().unwrap_or(0) as f64;
        let r = (t.cbrt() * (1.0 + 2f64.powi(-40))) as u128 + 2;
        let mut x = Nat::from_u128(r).shl(s / 3);
        loop {
            let (q, _) = self.divrem(&x.mul(&x));
            let (y, _) = x.shl(1).add(&q).divrem_small(3);
            if y.cmp(&x) != Ordering::Less {
                return x;
            }
            x = y;
        }
    }

    /// Lower-case hexadecimal digits, at least `min_digits` of them.
    pub(crate) fn to_hex(&self, min_digits: usize) -> String {
        let mut s = String::new();
        for (i, &l) in self.d.iter().rev().enumerate() {
            if i == 0 {
                s.push_str(&format!("{l:x}"));
            } else {
                s.push_str(&format!("{l:016x}"));
            }
        }
        if s.is_empty() {
            s.push('0');
        }
        if s.len() < min_digits {
            s = "0".repeat(min_digits - s.len()) + &s;
        }
        s
    }

    /// Decimal digits.
    pub(crate) fn to_dec(&self) -> String {
        const CHUNK: u64 = 10_000_000_000_000_000_000;
        let mut parts = Vec::new();
        let mut n = self.clone();
        while !n.is_zero() {
            let (q, r) = n.divrem_small(CHUNK);
            parts.push(r);
            n = q;
        }
        let mut s = String::new();
        for (i, p) in parts.iter().rev().enumerate() {
            if i == 0 {
                s.push_str(&p.to_string());
            } else {
                s.push_str(&format!("{p:019}"));
            }
        }
        if s.is_empty() {
            s.push('0');
        }
        s
    }
}

/// `floor(sqrt(n))`: the double-precision root (within a few units of the true root for any
/// `n < 2^128`), corrected by exact integer comparisons.
fn isqrt_u128(n: u128) -> u128 {
    let mut s = ((n as f64).sqrt() as u128).min(u128::from(u64::MAX));
    while s * s > n {
        s -= 1;
    }
    while (s + 1).checked_mul(s + 1).is_some_and(|p| p <= n) {
        s += 1;
    }
    s
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            // splitmix64
            self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^ (z >> 31)
        }
        /// A u128 with a random number of significant bits, so small values are frequent.
        fn u128(&mut self) -> u128 {
            let v = (u128::from(self.next()) << 64) | u128::from(self.next());
            v >> (self.next() % 128)
        }
        fn nat(&mut self, max_limbs: u64) -> Nat {
            let n = self.next() % (max_limbs + 1);
            let mut d: Vec<u64> = (0..n).map(|_| self.next()).collect();
            if let Some(l) = d.last_mut() {
                *l >>= self.next() % 64;
            }
            // Runs of all-ones or zero limbs stress the carry and correction paths.
            for l in &mut d {
                match self.next() % 8 {
                    0 => *l = u64::MAX,
                    1 => *l = 0,
                    _ => {}
                }
            }
            Nat::from_limbs(d)
        }
    }

    #[test]
    fn against_u128() {
        let mut g = Rng(1);
        for _ in 0..200_000 {
            let (a, b) = (g.u128(), g.u128());
            let (na, nb) = (Nat::from_u128(a), Nat::from_u128(b));
            if let Some(s) = a.checked_add(b) {
                assert_eq!(na.add(&nb).to_u128(), Some(s));
            }
            assert_eq!(na.sub(&nb).to_u128(), Some(a.saturating_sub(b)));
            if let Some(p) = a.checked_mul(b) {
                assert_eq!(na.mul(&nb).to_u128(), Some(p));
            }
            if let (Some(wq), Some(wr)) = (a.checked_div(b), a.checked_rem(b)) {
                let (q, r) = na.divrem(&nb);
                assert_eq!((q.to_u128(), r.to_u128()), (Some(wq), Some(wr)), "{a} / {b}");
            }
            assert_eq!(na.cmp(&nb), a.cmp(&b));
            let k = g.next() % 140;
            assert_eq!(na.shr(k).to_u128(), Some(a.checked_shr(k as u32).unwrap_or(0)));
            if k < 128 && (a.leading_zeros() as u64) >= k {
                assert_eq!(na.shl(k).to_u128(), Some(a << k));
            }
            assert_eq!(na.bit_len(), u64::from(128 - a.leading_zeros()));
            assert_eq!(na.sqrtrem().0.to_u128(), Some(isqrt_bisect(a)));
            assert_eq!(na.low_bits_nonzero(k), k > 0 && a & (u128::MAX >> (128 - k.min(128))) != 0);
            assert_eq!(na.low_bits(k).to_u128(), Some(if k >= 128 { a } else { a & ((1u128 << k) - 1) }));
        }
    }

    fn isqrt_bisect(a: u128) -> u128 {
        let mut lo = 0u128;
        let mut hi = 1u128 << 64;
        while hi - lo > 1 {
            let mid = lo + (hi - lo) / 2;
            if mid.checked_mul(mid).is_some_and(|p| p <= a) {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        lo
    }

    #[test]
    fn multi_limb_division_identity() {
        let mut g = Rng(2);
        for _ in 0..30_000 {
            let b = g.nat(12);
            if b.is_zero() {
                continue;
            }
            let q = g.nat(12);
            let r = g.nat(12).divrem(&b).1;
            let a = q.mul(&b).add(&r);
            let (q2, r2) = a.divrem(&b);
            assert_eq!((q2, r2), (q, r));
        }
    }

    #[test]
    fn roots() {
        let mut g = Rng(3);
        for _ in 0..3000 {
            let n = g.nat(20);
            let s = n.sqrtrem().0;
            assert!(s.mul(&s).cmp(&n) != Ordering::Greater);
            let s1 = s.add_small(1);
            assert_eq!(s1.mul(&s1).cmp(&n), Ordering::Greater);
            let c = n.icbrt();
            assert!(c.pow(3).cmp(&n) != Ordering::Greater);
            assert_eq!(c.add_small(1).pow(3).cmp(&n), Ordering::Greater);
            // Exact powers.
            let x = g.nat(5);
            assert_eq!(x.mul(&x).sqrtrem(), (x.clone(), Nat::zero()));
            assert_eq!(x.pow(3).icbrt(), x);
        }
    }

    #[test]
    fn karatsuba_product_matches_schoolbook() {
        // no spec clause: oracle arithmetic (D-10.130). A wrong split, middle term or recombination
        // shift in Karatsuba's method changes the product; sizes straddle the threshold and its
        // recursion levels, balanced and unbalanced, with runs of zero and all-ones limbs (carry
        // and borrow chains through the middle term).
        let mut g = Rng(7);
        let sizes = [KARATSUBA_MIN - 1, KARATSUBA_MIN, KARATSUBA_MIN + 1, 2 * KARATSUBA_MIN + 1, 97, 300, 1025];
        for &la in &sizes {
            for &lb in &sizes {
                let limbs_of = |g: &mut Rng, n: usize| {
                    let mut d: Vec<u64> = (0..n).map(|_| match g.next() % 4 { 0 => u64::MAX, 1 => 0, _ => g.next() }).collect();
                    d.push(1 + g.next() % 7);
                    Nat::from_limbs(d)
                };
                let (a, b) = (limbs_of(&mut g, la), limbs_of(&mut g, lb));
                assert_eq!(a.mul(&b), a.mul_schoolbook(&b), "{la} x {lb} limbs");
            }
        }
        let ones = Nat::from_limbs(vec![u64::MAX; 513]);
        assert_eq!(ones.mul(&ones), ones.mul_schoolbook(&ones));
    }

    /// `floor(sqrt(n))` by Newton's iteration `x <- (x + n/x)/2` on full division, from a
    /// floating-point start above the root: the method `sqrtrem` replaced, kept as its reference.
    fn isqrt_newton(n: &Nat) -> Nat {
        if n.is_zero() {
            return Nat::zero();
        }
        let s = n.bit_len().saturating_sub(100) & !1;
        let t = n.shr(s).to_u128().unwrap() as f64;
        let mut x = Nat::from_u128((t.sqrt() * (1.0 + 2f64.powi(-40))) as u128 + 2).shl(s / 2);
        loop {
            let y = x.add(&n.divrem(&x).0).shr(1);
            if y.cmp(&x) != Ordering::Less {
                return x;
            }
            x = y;
        }
    }

    /// The root of `n` by `sqrtrem`, after checking its remainder: `s^2 + r = n` with
    /// `0 <= r <= 2s` holds for `s = floor(sqrt(n))` only, so this alone proves the pair.
    fn checked_sqrtrem(n: &Nat) -> Nat {
        let (s, r) = n.sqrtrem();
        assert_eq!(s.mul(&s).add(&r), *n, "s^2 + r != n for n = {}", n.to_hex(0));
        assert!(r.cmp(&s.shl(1)) != Ordering::Greater, "r > 2s for n = {}", n.to_hex(0));
        s
    }

    #[test]
    fn sqrtrem_matches_newton_on_edges_and_random_values() {
        // no spec clause: oracle arithmetic (D-10.130). A wrong split point, quotient or
        // correction step in the Karatsuba square root gives a root one off or a wrong remainder;
        // the cases are powers of two and their neighbours (every limb boundary 2^64k +- 1 up
        // to 700 bits), squares and their neighbours on both sides, the largest value with a
        // given root ((x+1)^2 - 1), and random values with runs of zero and all-ones limbs.
        let one = Nat::from_u64(1);
        let mut cases = Vec::new();
        for j in 0..700 {
            let p = one.shl(j);
            cases.push(p.sub(&one));
            cases.push(p.add_small(1));
            cases.push(p);
        }
        let mut g = Rng(4);
        for _ in 0..600 {
            let x = g.nat(12);
            let sq = x.mul(&x);
            cases.push(sq.sub(&one));
            cases.push(sq.add_small(1));
            cases.push(sq.add(&x.shl(1)));
            cases.push(sq);
            cases.push(g.nat(24));
        }
        for n in &cases {
            assert_eq!(checked_sqrtrem(n), isqrt_newton(n), "n = {}", n.to_hex(0));
        }
    }

    #[test]
    fn u128_root_corrects_the_float_estimate() {
        // no spec clause: oracle arithmetic (D-10.130). The base case starts from the
        // double-precision root, which is off by units for roots above 2^53; squares and their
        // neighbours there (up to u128::MAX) must still give the floor.
        let mut g = Rng(5);
        let mut xs: Vec<u128> = vec![1, 2, 3, (1 << 26) - 1, 1 << 26, (1 << 32) - 1, 1 << 32, (1 << 53) - 1, 1 << 53, (1 << 53) + 1, (1 << 63) - 1, 1 << 63];
        xs.extend([u128::from(u64::MAX) - 1, u128::from(u64::MAX)]);
        xs.extend((0..2000).map(|_| u128::from(g.next() >> (g.next() % 64)).max(1)));
        for x in xs {
            for n in [x * x - 1, x * x, x * x + 1, x * x + 2 * x] {
                assert_eq!(isqrt_u128(n), isqrt_bisect(n), "n = {n}");
            }
        }
        assert_eq!(isqrt_u128(u128::MAX), u128::from(u64::MAX));
    }

    #[test]
    fn sqrtrem_on_huge_values() {
        // no spec clause: oracle arithmetic (D-10.130). The oracle's operands reach the
        // Ziv cap (2^17 bits and beyond); deep recursion with odd and even split sizes, exact
        // squares and the values next to them at those lengths.
        let mut g = Rng(6);
        for limbs in [33usize, 100, 257, 1024, 2049, 4100] {
            let mut d: Vec<u64> = (0..limbs).map(|_| g.next()).collect();
            *d.last_mut().unwrap() >>= g.next() % 64;
            let n = Nat::from_limbs(d.clone());
            checked_sqrtrem(&n);
            let x = Nat::from_limbs(d[..limbs / 2].to_vec());
            let sq = x.mul(&x);
            assert_eq!(sq.sqrtrem(), (x.clone(), Nat::zero()));
            assert_eq!(checked_sqrtrem(&sq.sub(&Nat::from_u64(1))), x.sub(&Nat::from_u64(1)));
        }
    }

    #[test]
    fn text() {
        assert_eq!(Nat::from_u128(u128::MAX).to_dec(), u128::MAX.to_string());
        assert_eq!(Nat::from_u128(0xabc_0000_0000_0000_0001).to_hex(0), "abc0000000000000001");
        assert_eq!(Nat::from_u64(10).pow(40).to_dec(), format!("1{}", "0".repeat(40)));
    }
}
