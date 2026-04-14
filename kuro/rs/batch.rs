//! Batch inversion via Montgomery's trick.
//!
//! Computes N inversions using 1 actual inversion + 3(N-1) multiplications.
//! This amortizes the expensive tower-recursive inversion across many elements.
//!
//! Works at any tower level. Implemented for F₂¹²⁸ as the primary use case.

use crate::tower::F2_128;

/// Batch-invert an array of F₂¹²⁸ elements.
///
/// Uses Montgomery's trick:
/// 1. Compute running products: prefix[i] = a[0] · a[1] · ... · a[i]
/// 2. Invert the final product once: inv_all = prefix[N-1]⁻¹
/// 3. Walk backward recovering each individual inverse.
///
/// Zero elements are left as zero in output.
///
/// Cost: 1 inversion + 3·(N-1) multiplications (vs N inversions naively).
pub fn batch_inv_128(input: &[F2_128], output: &mut [F2_128]) {
    let n = input.len();
    assert!(n == output.len(), "length mismatch");
    if n == 0 { return; }

    // Step 1: running products (skip zeros)
    let mut running = F2_128::ONE;
    for i in 0..n {
        if input[i].is_zero() {
            output[i] = F2_128::ZERO;
        } else {
            running = running.mul(input[i]);
            output[i] = running;
        }
    }

    if running.is_zero() {
        for i in 0..n { output[i] = F2_128::ZERO; }
        return;
    }

    // Step 2: invert accumulated product
    let mut inv_acc = running.inv();

    // Step 3: walk backward extracting each inverse
    let mut i = n;
    while i > 1 {
        i -= 1;
        if input[i].is_zero() {
            output[i] = F2_128::ZERO;
        } else {
            // output[i-1] holds prefix product up to i-1
            let prev = if i > 0 { output[i - 1] } else { F2_128::ONE };
            output[i] = inv_acc.mul(if prev.is_zero() { F2_128::ONE } else { prev });
            inv_acc = inv_acc.mul(input[i]);
        }
    }
    if !input[0].is_zero() {
        output[0] = inv_acc;
    } else {
        output[0] = F2_128::ZERO;
    }
}
