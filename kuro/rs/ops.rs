//! std::ops trait implementations for F2_128.
//!
//! In characteristic 2: add = sub = XOR, neg = identity.

use crate::tower::F2_128;
use core::ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};

impl Add for F2_128 {
    type Output = Self;
    #[inline(always)]
    fn add(self, rhs: Self) -> Self {
        F2_128::add(self, rhs)
    }
}

impl AddAssign for F2_128 {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Self) {
        *self = F2_128::add(*self, rhs);
    }
}

impl Sub for F2_128 {
    type Output = Self;
    #[inline(always)]
    fn sub(self, rhs: Self) -> Self {
        // In char 2, subtraction = addition = XOR
        F2_128::add(self, rhs)
    }
}

impl SubAssign for F2_128 {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: Self) {
        *self = F2_128::add(*self, rhs);
    }
}

impl Mul for F2_128 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        F2_128::mul(self, rhs)
    }
}

impl MulAssign for F2_128 {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        *self = F2_128::mul(*self, rhs);
    }
}

impl Neg for F2_128 {
    type Output = Self;
    #[inline(always)]
    fn neg(self) -> Self {
        // In char 2, negation is identity: -x = x
        self
    }
}
