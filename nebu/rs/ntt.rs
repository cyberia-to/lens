// ---
// tags: nebu, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Number Theoretic Transform over the Goldilocks field.
//!
//! Forward NTT: Cooley-Tukey decimation-in-time (bit-reversed input, natural output).
//! Inverse NTT: Gentleman-Sande decimation-in-frequency (natural input, bit-reversed output).

use crate::field::{Goldilocks, P};

/// Primitive root of F_p*.
const G: Goldilocks = Goldilocks::new(7);

/// Reverse the lowest `k` bits of `i`.
#[inline]
pub fn bit_reverse(i: usize, k: u32) -> usize {
    let mut result = 0usize;
    let val = i;
    for b in 0..k {
        result |= ((val >> b as usize) & 1) << (k - 1 - b) as usize;
        let _ = val; // suppress unused warning
    }
    // Simpler: use the standard bit-reverse approach
    result
}

/// In-place bit-reversal permutation.
pub fn bit_reverse_permute(a: &mut [Goldilocks]) {
    let n = a.len();
    let k = n.trailing_zeros();
    for i in 0..n {
        let j = bit_reverse(i, k);
        if i < j {
            a.swap(i, j);
        }
    }
}

/// Forward NTT (Cooley-Tukey DIT).
///
/// Input in natural order, output in natural order.
/// Length must be a power of 2.
pub fn ntt(a: &mut [Goldilocks]) {
    let n = a.len();
    assert!(n.is_power_of_two());
    if n == 1 {
        return;
    }
    let k = n.trailing_zeros();

    bit_reverse_permute(a);

    for s in 0..k {
        let m = 1usize << (s + 1);
        let omega_m = G.exp((P - 1) / m as u64);

        let half_m = m / 2;
        let mut j = 0;
        while j < n {
            let mut w = Goldilocks::ONE;
            for i in 0..half_m {
                let t = w * a[j + i + half_m];
                a[j + i + half_m] = a[j + i] - t;
                a[j + i] = a[j + i] + t;
                w = w * omega_m;
            }
            j += m;
        }
    }
}

/// Inverse NTT (Gentleman-Sande DIF).
///
/// Input in natural order, output in natural order.
/// Includes the N⁻¹ scaling.
pub fn intt(a: &mut [Goldilocks]) {
    let n = a.len();
    assert!(n.is_power_of_two());
    if n == 1 {
        return;
    }
    let k = n.trailing_zeros();

    for s in (0..k).rev() {
        let m = 1usize << (s + 1);
        let omega_m = G.exp((P - 1) / m as u64);
        let omega_m_inv = omega_m.exp(m as u64 - 1);

        let half_m = m / 2;
        let mut j = 0;
        while j < n {
            let mut w = Goldilocks::ONE;
            for i in 0..half_m {
                let u = a[j + i];
                let v = a[j + i + half_m];
                a[j + i] = u + v;
                a[j + i + half_m] = w * (u - v);
                w = w * omega_m_inv;
            }
            j += m;
        }
    }

    bit_reverse_permute(a);

    // Scale by N⁻¹ mod p: n_inv = p - (p-1)/N
    let n_inv = Goldilocks::new(P - (P - 1) / n as u64);
    for x in a.iter_mut() {
        *x = *x * n_inv;
    }
}

/// Precompute twiddle factors for a length-N NTT.
/// Writes N-1 factors into `table` (sum of m/2 entries across all stages).
pub fn precompute_twiddles(n: usize, table: &mut [Goldilocks]) {
    assert!(n.is_power_of_two());
    let k = n.trailing_zeros();
    let mut idx = 0;
    for s in 0..k {
        let m = 1usize << (s + 1);
        let omega_m = G.exp((P - 1) / m as u64);
        let half_m = m / 2;
        let mut w = Goldilocks::ONE;
        for _ in 0..half_m {
            table[idx] = w;
            idx += 1;
            w = w * omega_m;
        }
    }
}
