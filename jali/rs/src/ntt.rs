// ---
// tags: jali, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! NTT domain transforms for negacyclic convolution.
//!
//! For R_q = F_p[x]/(x^n+1) we need the negacyclic NTT:
//! - Standard NTT works modulo x^n - 1
//! - Negacyclic NTT works modulo x^n + 1
//!
//! Implementation:
//! 1. Pre-multiply: coeffs[i] *= psi^i where psi is a primitive 2n-th root of unity
//! 2. Apply nebu's forward NTT
//! 3. For inverse: apply nebu's INTT, then post-multiply by psi^(-i)

use nebu::Goldilocks;
use nebu::field::P;

use crate::ring::RingElement;

/// Primitive root of F_p* (same as nebu's internal G=7).
const G: Goldilocks = Goldilocks::new(7);

/// Compute a primitive 2n-th root of unity.
///
/// Since Goldilocks has multiplicative order p-1 = 2^32 * (2^32 - 1),
/// and 2n divides 2^32 for n <= 4096, a 2n-th root always exists.
/// We compute g^((p-1)/(2n)) where g=7 is a primitive root.
#[inline]
fn psi_for(n: usize) -> Goldilocks {
    let two_n = (2 * n) as u64;
    G.exp((P - 1) / two_n)
}

/// Forward negacyclic NTT on raw coefficient slice.
///
/// Transforms coefficients so that polynomial multiplication mod (x^n+1)
/// becomes pointwise multiplication.
pub fn negacyclic_forward(coeffs: &mut [Goldilocks], n: usize) {
    assert!(n.is_power_of_two());
    assert!(coeffs.len() >= n);

    let psi = psi_for(n);

    // Step 1: twist — multiply coeffs[i] by psi^i
    let mut psi_pow = Goldilocks::ONE;
    for i in 0..n {
        coeffs[i] = coeffs[i] * psi_pow;
        psi_pow = psi_pow * psi;
    }

    // Step 2: standard forward NTT
    nebu::ntt::ntt(&mut coeffs[..n]);
}

/// Inverse negacyclic NTT on raw coefficient slice.
///
/// Recovers polynomial coefficients from NTT domain.
pub fn negacyclic_inverse(coeffs: &mut [Goldilocks], n: usize) {
    assert!(n.is_power_of_two());
    assert!(coeffs.len() >= n);

    // Step 1: standard inverse NTT (includes 1/n scaling)
    nebu::ntt::intt(&mut coeffs[..n]);

    // Step 2: untwist — multiply coeffs[i] by psi^(-i)
    let psi = psi_for(n);
    let psi_inv = psi.inv();
    let mut psi_inv_pow = Goldilocks::ONE;
    for i in 0..n {
        coeffs[i] = coeffs[i] * psi_inv_pow;
        psi_inv_pow = psi_inv_pow * psi_inv;
    }
}

/// Convert a RingElement to NTT form (in-place).
pub fn to_ntt(elem: &mut RingElement) {
    assert!(!elem.is_ntt, "already in NTT form");
    let n = elem.n;
    negacyclic_forward(&mut elem.coeffs, n);
    elem.is_ntt = true;
}

/// Convert a RingElement from NTT form back to coefficient form (in-place).
pub fn from_ntt(elem: &mut RingElement) {
    assert!(elem.is_ntt, "not in NTT form");
    let n = elem.n;
    negacyclic_inverse(&mut elem.coeffs, n);
    elem.is_ntt = false;
}
