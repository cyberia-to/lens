//! Reed–Solomon encoding over Goldilocks.
//!
//! `RS[n, k]`: a message `c ∈ F^k` (`k ≤ n`, both powers of two) is the
//! coefficient vector of `ĉ(X) = Σ c_i X^i`; the codeword is
//! `(ĉ(ω^j))_{j<n}` with `ω` the generator of the order-`n` subgroup
//! (`root_of_unity`). Distinct codewords agree on at most `k − 1` points
//! (degree bound), so the minimum distance is `n − k + 1` (MDS).
//!
//! Encoding splits the domain into `n/k` cosets `ω^s·H` of the order-`k`
//! subgroup `H`: index `j = u·(n/k) + s` evaluates at `ω^s·η^u` with
//! `η = ω^{n/k}`, so each coset is one size-`k` NTT of `c_i·ω^{s·i}`.
//! Cosets are independent and run in parallel.

use nebu::ntt::ntt;
use nebu::{Fp3, Goldilocks};

use crate::field::root_of_unity;
use crate::par::par_map_min;

fn coset_ntt(coeffs: &[Goldilocks], shift: Goldilocks) -> Vec<Goldilocks> {
    let mut v = Vec::with_capacity(coeffs.len());
    let mut s = Goldilocks::ONE;
    for &c in coeffs {
        v.push(c * s);
        s *= shift;
    }
    ntt(&mut v);
    v
}

/// Encode a base-field message (length a power of two) into `2^log_n`
/// symbols.
pub fn encode_base(coeffs: &[Goldilocks], log_n: u32) -> Vec<Goldilocks> {
    encode_base_with(coeffs, log_n, true)
}

/// As `encode_base`; `parallel = false` keeps the work on this thread
/// (for callers already running one encoding per worker).
pub fn encode_base_with(coeffs: &[Goldilocks], log_n: u32, parallel: bool) -> Vec<Goldilocks> {
    let k = coeffs.len();
    let n = 1usize << log_n;
    assert!(k.is_power_of_two() && k <= n);
    let blowup = n / k;
    let omega = root_of_unity(log_n);
    let cosets: Vec<Vec<Goldilocks>> = if parallel && blowup > 1 && k >= 1024 {
        par_map_min(blowup, 2, |s| coset_ntt(coeffs, omega.exp(s as u64)))
    } else {
        (0..blowup)
            .map(|s| coset_ntt(coeffs, omega.exp(s as u64)))
            .collect()
    };
    interleave(&cosets, k, blowup)
}

/// Encode an Fp3 message limb-wise (the code is F-linear and the domain
/// lies in F, so encoding commutes with the limb decomposition).
pub fn encode_ext(coeffs: &[Fp3], log_n: u32) -> Vec<Fp3> {
    let limb = |f: fn(&Fp3) -> Goldilocks| coeffs.iter().map(f).collect::<Vec<_>>();
    let l0 = encode_base(&limb(|x| x.c0), log_n);
    let l1 = encode_base(&limb(|x| x.c1), log_n);
    let l2 = encode_base(&limb(|x| x.c2), log_n);
    l0.into_iter()
        .zip(l1)
        .zip(l2)
        .map(|((a, b), c)| Fp3::new(a, b, c))
        .collect()
}

fn interleave(cosets: &[Vec<Goldilocks>], k: usize, blowup: usize) -> Vec<Goldilocks> {
    let mut out = vec![Goldilocks::ZERO; k * blowup];
    for (s, coset) in cosets.iter().enumerate() {
        for (u, &v) in coset.iter().enumerate() {
            out[u * blowup + s] = v;
        }
    }
    out
}
