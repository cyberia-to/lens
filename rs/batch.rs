// ---
// tags: nebu, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Batch inversion via Montgomery's trick.
//!
//! Inverts N elements with 1 inversion + 3(N-1) multiplications.

use crate::field::Goldilocks;

/// Batch-invert `a` into `result`. Both slices must have the same length.
/// All elements of `a` must be non-zero.
///
/// Uses `result` as scratch space for prefix products.
pub fn batch_inv(a: &[Goldilocks], result: &mut [Goldilocks]) {
    let n = a.len();
    assert!(n == result.len());
    if n == 0 {
        return;
    }

    // Phase 1: prefix products in result
    result[0] = a[0];
    for i in 1..n {
        result[i] = result[i - 1] * a[i];
    }

    // Phase 2: invert the total product
    let mut inv_all = result[n - 1].inv();

    // Phase 3: propagate inverses backward
    for i in (1..n).rev() {
        result[i] = inv_all * result[i - 1];
        inv_all = inv_all * a[i];
    }
    result[0] = inv_all;
}

/// Batch-invert with zero handling. Zero inputs produce zero outputs.
pub fn batch_inv_safe(a: &[Goldilocks], result: &mut [Goldilocks]) {
    let n = a.len();
    assert!(n == result.len());
    if n == 0 {
        return;
    }

    // Replace zeros with 1 in the prefix product
    result[0] = if a[0].is_zero() { Goldilocks::ONE } else { a[0] };
    for i in 1..n {
        let ai = if a[i].is_zero() { Goldilocks::ONE } else { a[i] };
        result[i] = result[i - 1] * ai;
    }

    let mut inv_all = result[n - 1].inv();

    for i in (1..n).rev() {
        let ai = if a[i].is_zero() { Goldilocks::ONE } else { a[i] };
        result[i] = if a[i].is_zero() {
            Goldilocks::ZERO
        } else {
            inv_all * result[i - 1]
        };
        inv_all = inv_all * ai;
    }
    result[0] = if a[0].is_zero() { Goldilocks::ZERO } else { inv_all };
}
