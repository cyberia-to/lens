// ---
// tags: nebu, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Goldilocks prime field (p = 2^64 - 2^32 + 1).

use core::ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};

/// The Goldilocks prime: p = 2^64 - 2^32 + 1.
pub const P: u64 = 0xFFFF_FFFF_0000_0001;

/// Correction constant: ε = 2^32 - 1 = p.wrapping_neg().
pub const EPSILON: u64 = P.wrapping_neg(); // 0xFFFF_FFFF

/// A Goldilocks field element.
///
/// Internal value may be non-canonical (in `[0, 2^64)`).
/// Use `as_u64()` to reduce to `[0, p)`.
#[derive(Clone, Copy, Default, Eq, Hash)]
#[repr(transparent)]
pub struct Goldilocks {
    value: u64,
}

impl Goldilocks {
    pub const ZERO: Self = Self { value: 0 };
    pub const ONE: Self = Self { value: 1 };
    pub const NEG_ONE: Self = Self { value: P - 1 };

    #[inline]
    pub const fn new(value: u64) -> Self {
        Self { value }
    }

    /// Reduce to canonical form in [0, p).
    #[inline]
    pub const fn canonicalize(self) -> Self {
        let mut c = self.value;
        if c >= P {
            c -= P;
        }
        Self { value: c }
    }

    /// Canonical u64 value in [0, p).
    #[inline]
    pub const fn as_u64(self) -> u64 {
        self.canonicalize().value
    }

    #[inline]
    pub const fn is_zero(self) -> bool {
        self.as_u64() == 0
    }

    /// Compute x^2.
    #[inline]
    pub fn square(self) -> Self {
        self * self
    }

    /// Compute x^7 (Poseidon2 S-box). 4 multiplications.
    #[inline]
    pub fn pow7(self) -> Self {
        let x2 = self.square();
        let x3 = x2 * self;
        let x4 = x2.square();
        x3 * x4
    }

    /// General exponentiation by square-and-multiply.
    #[inline]
    pub fn exp(self, mut e: u64) -> Self {
        let mut base = self;
        let mut result = Self::ONE;
        while e > 0 {
            if e & 1 == 1 {
                result = result * base;
            }
            base = base.square();
            e >>= 1;
        }
        result
    }

    /// Multiplicative inverse via Fermat: a^(p-2).
    ///
    /// p-2 = 0xFFFFFFFE_FFFFFFFF: all bits set except bit 32.
    /// Square-and-multiply scanning bits 62..0.
    pub fn inv(self) -> Self {
        let mut t = self;
        for i in (0..=62).rev() {
            t = t.square();
            if i != 32 {
                t = t * self;
            }
        }
        t
    }

    /// Negation: -x mod p.
    #[inline]
    pub fn field_neg(self) -> Self {
        if self.is_zero() {
            Self::ZERO
        } else {
            Self::new(P - self.as_u64())
        }
    }
}

impl PartialEq for Goldilocks {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_u64() == other.as_u64()
    }
}

impl core::fmt::Debug for Goldilocks {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Goldilocks(0x{:016X})", self.as_u64())
    }
}

impl core::fmt::Display for Goldilocks {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "0x{:016X}", self.as_u64())
    }
}

// ── Arithmetic operators ────────────────────────────────────────────

impl Add for Goldilocks {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self {
        let (sum, over) = self.value.overflowing_add(rhs.value);
        let (mut sum, over) = sum.overflowing_add(u64::from(over) * EPSILON);
        if over {
            sum += EPSILON;
        }
        Self::new(sum)
    }
}

impl AddAssign for Goldilocks {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for Goldilocks {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self {
        let (diff, under) = self.value.overflowing_sub(rhs.value);
        let (mut diff, under) = diff.overflowing_sub(u64::from(under) * EPSILON);
        if under {
            diff -= EPSILON;
        }
        Self::new(diff)
    }
}

impl SubAssign for Goldilocks {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul for Goldilocks {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: Self) -> Self {
        reduce128(u128::from(self.value) * u128::from(rhs.value))
    }
}

impl MulAssign for Goldilocks {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl Neg for Goldilocks {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        self.field_neg()
    }
}

/// Reduce a 128-bit product to a Goldilocks element.
///
/// Uses the identity 2^64 ≡ ε (mod p).
#[inline]
fn reduce128(x: u128) -> Goldilocks {
    let x_lo = x as u64;
    let x_hi = (x >> 64) as u64;
    let x_hi_hi = x_hi >> 32;
    let x_hi_lo = x_hi & EPSILON;

    let (mut t0, borrow) = x_lo.overflowing_sub(x_hi_hi);
    if borrow {
        t0 -= EPSILON;
    }
    let t1 = x_hi_lo * EPSILON;
    let (res, carry) = t0.overflowing_add(t1);
    Goldilocks::new(res + EPSILON * u64::from(carry))
}
