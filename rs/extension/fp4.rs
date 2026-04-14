// ---
// tags: nebu, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Quartic extension field F_{p⁴} = F_p[w] / (w⁴ - 7).
//!
//! Elements are (c0, c1, c2, c3) representing c0 + c1·w + c2·w² + c3·w³.
//! Reduction: w⁴ = 7.
//!
//! Tower decomposition: Fp4 = Fp2[v] / (v² - u) where u² = 7, v = w.

use crate::field::Goldilocks;
use super::fp2::Fp2;
use core::ops::{Add, Mul, Neg, Sub};

const SEVEN: Goldilocks = Goldilocks::new(7);

/// Frobenius constant: 7^((p-1)/4) = 2^48.
const W_FROB: Goldilocks = Goldilocks::new(0x0001000000000000);

/// An element of F_{p⁴}.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct Fp4 {
    pub c0: Goldilocks,
    pub c1: Goldilocks,
    pub c2: Goldilocks,
    pub c3: Goldilocks,
}

impl Fp4 {
    pub const ZERO: Self = Self {
        c0: Goldilocks::ZERO,
        c1: Goldilocks::ZERO,
        c2: Goldilocks::ZERO,
        c3: Goldilocks::ZERO,
    };
    pub const ONE: Self = Self {
        c0: Goldilocks::ONE,
        c1: Goldilocks::ZERO,
        c2: Goldilocks::ZERO,
        c3: Goldilocks::ZERO,
    };

    #[inline]
    pub const fn new(c0: Goldilocks, c1: Goldilocks, c2: Goldilocks, c3: Goldilocks) -> Self {
        Self { c0, c1, c2, c3 }
    }

    /// Embed a base field element as (a, 0, 0, 0).
    #[inline]
    pub const fn from_base(a: Goldilocks) -> Self {
        Self { c0: a, c1: Goldilocks::ZERO, c2: Goldilocks::ZERO, c3: Goldilocks::ZERO }
    }

    /// Embed an Fp2 element via tower: (re, im) → (re, 0, im, 0).
    #[inline]
    pub const fn from_fp2(x: Fp2) -> Self {
        Self { c0: x.re, c1: Goldilocks::ZERO, c2: x.im, c3: Goldilocks::ZERO }
    }

    /// Extract tower components: A = (c0, c2), B = (c1, c3) in Fp2.
    #[inline]
    pub fn to_fp2_pair(self) -> (Fp2, Fp2) {
        (Fp2::new(self.c0, self.c2), Fp2::new(self.c1, self.c3))
    }

    /// Tower conjugate: (A + Bv) → (A - Bv) = (c0, -c1, c2, -c3).
    #[inline]
    pub fn conj(self) -> Self {
        Self { c0: self.c0, c1: -self.c1, c2: self.c2, c3: -self.c3 }
    }

    /// Squaring using w⁴ = 7.
    pub fn sqr(self) -> Self {
        let a0 = self.c0;
        let a1 = self.c1;
        let a2 = self.c2;
        let a3 = self.c3;

        let s0 = a0.square();
        let s1 = a0 * a1;
        let s1 = s1 + s1;
        let s2 = a1.square() + a0 * a2 + a0 * a2;
        let s3 = a0 * a3 + a1 * a2;
        let s3 = s3 + s3;
        let s4 = a2.square() + a1 * a3 + a1 * a3;
        let s5 = a2 * a3;
        let s5 = s5 + s5;
        let s6 = a3.square();

        Self {
            c0: s0 + SEVEN * s4,
            c1: s1 + SEVEN * s5,
            c2: s2 + SEVEN * s6,
            c3: s3,
        }
    }

    /// Norm to Fp2: N = A² - u·B² where A,B are Fp2 tower components.
    pub fn norm_fp2(self) -> Fp2 {
        let (a, b) = self.to_fp2_pair();
        let a_sq = a.sqr();
        let b_sq = b.sqr();
        // u·B² : multiply Fp2 element by u = (0, 1), so (re, im) → (7·im, re)
        let u_b_sq = Fp2::new(SEVEN * b_sq.im, b_sq.re);
        a_sq - u_b_sq
    }

    /// Norm to Fp: compose Fp4→Fp2→Fp norms.
    pub fn norm(self) -> Goldilocks {
        self.norm_fp2().norm()
    }

    /// Inversion via tower norm.
    pub fn inv(self) -> Self {
        let (a, b) = self.to_fp2_pair();
        let n = self.norm_fp2();
        let n_inv = n.inv();

        // result = conj · n_inv = (A·n_inv, -B·n_inv)
        let r_a = a * n_inv;
        let r_b = -(b * n_inv);
        Self { c0: r_a.re, c1: r_b.re, c2: r_a.im, c3: r_b.im }
    }

    /// Frobenius: σ(w) = 2⁴⁸·w.
    /// σ(c0 + c1·w + c2·w² + c3·w³) = c0 + 2⁴⁸·c1·w − c2·w² − 2⁴⁸·c3·w³
    pub fn frobenius(self) -> Self {
        Self {
            c0: self.c0,
            c1: W_FROB * self.c1,
            c2: -self.c2,
            c3: -(W_FROB * self.c3),
        }
    }
}

impl core::fmt::Debug for Fp4 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Fp4({:?}, {:?}, {:?}, {:?})", self.c0, self.c1, self.c2, self.c3)
    }
}

impl Add for Fp4 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self {
            c0: self.c0 + rhs.c0,
            c1: self.c1 + rhs.c1,
            c2: self.c2 + rhs.c2,
            c3: self.c3 + rhs.c3,
        }
    }
}

impl Sub for Fp4 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self {
            c0: self.c0 - rhs.c0,
            c1: self.c1 - rhs.c1,
            c2: self.c2 - rhs.c2,
            c3: self.c3 - rhs.c3,
        }
    }
}

/// Schoolbook multiplication with w⁴ = 7 reduction.
/// 16 base muls + 3 mul-by-7 + 9 adds.
impl Mul for Fp4 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        let a0 = self.c0;
        let a1 = self.c1;
        let a2 = self.c2;
        let a3 = self.c3;
        let b0 = rhs.c0;
        let b1 = rhs.c1;
        let b2 = rhs.c2;
        let b3 = rhs.c3;

        let d0 = a0 * b0;
        let d1 = a0 * b1 + a1 * b0;
        let d2 = a0 * b2 + a1 * b1 + a2 * b0;
        let d3 = a0 * b3 + a1 * b2 + a2 * b1 + a3 * b0;
        let d4 = a1 * b3 + a2 * b2 + a3 * b1;
        let d5 = a2 * b3 + a3 * b2;
        let d6 = a3 * b3;

        Self {
            c0: d0 + SEVEN * d4,
            c1: d1 + SEVEN * d5,
            c2: d2 + SEVEN * d6,
            c3: d3,
        }
    }
}

impl Neg for Fp4 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self { c0: -self.c0, c1: -self.c1, c2: -self.c2, c3: -self.c3 }
    }
}
