// ---
// tags: nebu, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Quadratic extension field F_{p²} = F_p[u] / (u² - 7).
//!
//! Elements are (re, im) representing re + im·u where u² = 7.

use crate::field::Goldilocks;
use core::ops::{Add, Mul, Neg, Sub};

const SEVEN: Goldilocks = Goldilocks::new(7);
const EIGHT: Goldilocks = Goldilocks::new(8);

/// An element of F_{p²}.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct Fp2 {
    pub re: Goldilocks,
    pub im: Goldilocks,
}

impl Fp2 {
    pub const ZERO: Self = Self { re: Goldilocks::ZERO, im: Goldilocks::ZERO };
    pub const ONE: Self = Self { re: Goldilocks::ONE, im: Goldilocks::ZERO };

    #[inline]
    pub const fn new(re: Goldilocks, im: Goldilocks) -> Self {
        Self { re, im }
    }

    /// Embed a base field element as (a, 0).
    #[inline]
    pub const fn from_base(a: Goldilocks) -> Self {
        Self { re: a, im: Goldilocks::ZERO }
    }

    /// Conjugate: (a, b) → (a, -b).
    #[inline]
    pub fn conj(self) -> Self {
        Self { re: self.re, im: -self.im }
    }

    /// Norm: a² - 7b² (in F_p).
    #[inline]
    pub fn norm(self) -> Goldilocks {
        self.re.square() - SEVEN * self.im.square()
    }

    /// Optimized squaring: 2 muls + small-constant muls.
    pub fn sqr(self) -> Self {
        let ab = self.re * self.im;
        let re = (self.re + self.im) * (self.re + SEVEN * self.im) - EIGHT * ab;
        let im = ab + ab;
        Self { re, im }
    }

    /// Inversion via norm: (a + bu)⁻¹ = (a - bu) / (a² - 7b²).
    pub fn inv(self) -> Self {
        let n_inv = self.norm().inv();
        Self {
            re: self.re * n_inv,
            im: (-self.im) * n_inv,
        }
    }
}

impl core::fmt::Debug for Fp2 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Fp2({:?}, {:?})", self.re, self.im)
    }
}

impl Add for Fp2 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self { re: self.re + rhs.re, im: self.im + rhs.im }
    }
}

impl Sub for Fp2 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self { re: self.re - rhs.re, im: self.im - rhs.im }
    }
}

/// Karatsuba multiplication: 3 base muls + 1 mul-by-7 + 5 add/subs.
impl Mul for Fp2 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        let v0 = self.re * rhs.re;
        let v1 = self.im * rhs.im;
        let re = v0 + SEVEN * v1;
        let im = (self.re + self.im) * (rhs.re + rhs.im) - v0 - v1;
        Self { re, im }
    }
}

impl Neg for Fp2 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self { re: -self.re, im: -self.im }
    }
}
