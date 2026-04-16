//! std::ops trait implementations for Fq.
//!
//! Fq uses static methods with & references. These trait impls
//! wrap them into consuming-style operations for Field trait compatibility.

use crate::fq::Fq;
use core::ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};

impl Add for Fq {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Fq::add(&self, &rhs)
    }
}

impl AddAssign for Fq {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        *self = Fq::add(self, &rhs);
    }
}

impl Sub for Fq {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Fq::sub(&self, &rhs)
    }
}

impl SubAssign for Fq {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self = Fq::sub(self, &rhs);
    }
}

impl Mul for Fq {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        Fq::mul(&self, &rhs)
    }
}

impl MulAssign for Fq {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        *self = Fq::mul(self, &rhs);
    }
}

impl Neg for Fq {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Fq::neg(&self)
    }
}
