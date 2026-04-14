// ---
// tags: nebu, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Cubic extension field F_{p³} = F_p[t] / (t³ - t - 1).
//!
//! Elements are (c0, c1, c2) representing c0 + c1·t + c2·t².
//! Reduction: t³ = t + 1.

use crate::field::Goldilocks;
use core::ops::{Add, Mul, Neg, Sub};

/// An element of F_{p³}.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct Fp3 {
    pub c0: Goldilocks,
    pub c1: Goldilocks,
    pub c2: Goldilocks,
}

impl Fp3 {
    pub const ZERO: Self = Self {
        c0: Goldilocks::ZERO,
        c1: Goldilocks::ZERO,
        c2: Goldilocks::ZERO,
    };
    pub const ONE: Self = Self {
        c0: Goldilocks::ONE,
        c1: Goldilocks::ZERO,
        c2: Goldilocks::ZERO,
    };

    #[inline]
    pub const fn new(c0: Goldilocks, c1: Goldilocks, c2: Goldilocks) -> Self {
        Self { c0, c1, c2 }
    }

    /// Embed a base field element as (a, 0, 0).
    #[inline]
    pub const fn from_base(a: Goldilocks) -> Self {
        Self { c0: a, c1: Goldilocks::ZERO, c2: Goldilocks::ZERO }
    }

    /// Squaring using t³ = t + 1.
    pub fn sqr(self) -> Self {
        let a0 = self.c0;
        let a1 = self.c1;
        let a2 = self.c2;

        let s0 = a0.square();
        let s1 = a0 * a1;
        let s1 = s1 + s1;                    // 2·a0·a1
        let s2 = a1.square() + a0 * a2 + a0 * a2;  // a1² + 2·a0·a2
        let a1a2 = a1 * a2;
        let s3 = a1a2 + a1a2;                // 2·a1·a2
        let s4 = a2.square();

        // reduce: t³ = t + 1, t⁴ = t² + t
        let c0 = s0 + s3;
        let c1 = s1 + s3 + s4;
        let c2 = s2 + s4;
        Self { c0, c1, c2 }
    }

    /// Norm: F_{p³} → F_p.
    ///
    /// norm(a) = c0³ + c1³ + c2³ - 3·c0·c1·c2 + 2·c0²·c2 + c0·c2² - c1·c2² - c0·c1²
    pub fn norm(self) -> Goldilocks {
        let c0 = self.c0;
        let c1 = self.c1;
        let c2 = self.c2;

        let c0_2 = c0.square();
        let c1_2 = c1.square();
        let c2_2 = c2.square();
        let c0_3 = c0_2 * c0;
        let c1_3 = c1_2 * c1;
        let c2_3 = c2_2 * c2;
        let c0c1c2 = c0 * c1 * c2;
        let three = Goldilocks::new(3);

        c0_3 + c1_3 + c2_3 - three * c0c1c2
            + c0_2 * c2 + c0_2 * c2
            + c0 * c2_2
            - c1 * c2_2
            - c0 * c1_2
    }

    /// Inversion via norm and adjugate.
    ///
    /// The multiplication matrix M for t³ = t + 1 is:
    ///   [[c0, c2, c1], [c1, c0+c2, c1+c2], [c2, c1, c0+c2]]
    /// inv(alpha) = adj(M)[first column] / det(M)
    pub fn inv(self) -> Self {
        let c0 = self.c0;
        let c1 = self.c1;
        let c2 = self.c2;

        let n_inv = self.norm().inv();

        let c0_2 = c0.square();
        let c1_2 = c1.square();
        let c2_2 = c2.square();

        // First column of adjugate matrix
        let r0 = (c0_2 + c0 * c2 + c0 * c2 - c1_2 - c1 * c2 + c2_2) * n_inv;
        let r1 = (c2_2 - c0 * c1) * n_inv;
        let r2 = (c1_2 - c0 * c2 - c2_2) * n_inv;

        Self { c0: r0, c1: r1, c2: r2 }
    }
}

impl core::fmt::Debug for Fp3 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Fp3({:?}, {:?}, {:?})", self.c0, self.c1, self.c2)
    }
}

impl Add for Fp3 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self {
            c0: self.c0 + rhs.c0,
            c1: self.c1 + rhs.c1,
            c2: self.c2 + rhs.c2,
        }
    }
}

impl Sub for Fp3 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self {
            c0: self.c0 - rhs.c0,
            c1: self.c1 - rhs.c1,
            c2: self.c2 - rhs.c2,
        }
    }
}

/// Schoolbook multiplication with t³ = t + 1 reduction.
/// 9 base muls + 6 adds.
impl Mul for Fp3 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        let a0 = self.c0;
        let a1 = self.c1;
        let a2 = self.c2;
        let b0 = rhs.c0;
        let b1 = rhs.c1;
        let b2 = rhs.c2;

        // Schoolbook: 9 muls
        let d0 = a0 * b0;
        let d1 = a0 * b1 + a1 * b0;
        let d2 = a0 * b2 + a1 * b1 + a2 * b0;
        let d3 = a1 * b2 + a2 * b1;
        let d4 = a2 * b2;

        // Reduce: t³ = t + 1, t⁴ = t² + t
        let c0 = d0 + d3;
        let c1 = d1 + d3 + d4;
        let c2 = d2 + d4;
        Self { c0, c1, c2 }
    }
}

impl Neg for Fp3 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self { c0: -self.c0, c1: -self.c1, c2: -self.c2 }
    }
}
