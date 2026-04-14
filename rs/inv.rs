//! Inversion utilities for F₂ tower fields.
//!
//! Tower-recursive inversion reduces an inversion in GF(2^{2k})
//! to one inversion in GF(2^k) plus a few sub-field multiplications.
//! The recursion bottoms out at F₂ where 1⁻¹ = 1.
//!
//! This module provides checked_inv functions returning Option.

use crate::tower::*;

/// Checked inversion for F₂¹²⁸. Returns None for zero.
pub fn checked_inv_128(a: F2_128) -> Option<F2_128> {
    if a.is_zero() { None } else { Some(a.inv()) }
}

/// Checked inversion for F₂⁶⁴. Returns None for zero.
pub fn checked_inv_64(a: F2_64) -> Option<F2_64> {
    if a.is_zero() { None } else { Some(a.inv()) }
}

/// Checked inversion for F₂³². Returns None for zero.
pub fn checked_inv_32(a: F2_32) -> Option<F2_32> {
    if a.is_zero() { None } else { Some(a.inv()) }
}

/// Checked inversion for F₂¹⁶. Returns None for zero.
pub fn checked_inv_16(a: F2_16) -> Option<F2_16> {
    if a.is_zero() { None } else { Some(a.inv()) }
}

/// Checked inversion for F₂⁸. Returns None for zero.
pub fn checked_inv_8(a: F2_8) -> Option<F2_8> {
    if a.is_zero() { None } else { Some(a.inv()) }
}

/// Checked inversion for F₂⁴. Returns None for zero.
pub fn checked_inv_4(a: F2_4) -> Option<F2_4> {
    if a.is_zero() { None } else { Some(a.inv()) }
}

/// Checked inversion for F₂². Returns None for zero.
pub fn checked_inv_2(a: F2_2) -> Option<F2_2> {
    if a.is_zero() { None } else { Some(a.inv()) }
}
