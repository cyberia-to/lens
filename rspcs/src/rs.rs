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
    if parallel && blowup > 1 && blowup * k >= PAR_MIN {
        // the process's backend (cpu: one coset per thread), interleaved in threads
        let shifts: Vec<u64> = (0..blowup).map(|s| omega.exp(s as u64).as_u64()).collect();
        let cosets = crate::backend::current().coset_ntt(&crate::backend::limbs(coeffs), &shifts);
        return interleave_par(blowup * k, blowup, |s, u| Goldilocks::new(cosets[s][u]));
    }
    let cosets: Vec<Vec<Goldilocks>> = (0..blowup)
        .map(|s| coset_ntt(coeffs, omega.exp(s as u64)))
        .collect();
    interleave(&cosets, k, blowup)
}

/// Codewords at least this long encode in threads.
const PAR_MIN: usize = 1 << 14;

/// `out[u·blowup + s] = at(s, u)`, chunks of `out` in threads.
fn interleave_par<T: Copy + Default + Send>(
    n: usize,
    blowup: usize,
    at: impl Fn(usize, usize) -> T + Sync,
) -> Vec<T> {
    let mut out = vec![T::default(); n];
    let t = crate::par::threads();
    let chunk = n.div_ceil(t).next_multiple_of(blowup).max(blowup);
    std::thread::scope(|sc| {
        for (ci, part) in out.chunks_mut(chunk).enumerate() {
            let at = &at;
            sc.spawn(move || {
                for (j, o) in part.iter_mut().enumerate() {
                    let idx = ci * chunk + j;
                    *o = at(idx % blowup, idx / blowup);
                }
            });
        }
    });
    out
}

/// Encode an Fp3 message limb-wise (the code is F-linear and the domain
/// lies in F, so encoding commutes with the limb decomposition).
pub fn encode_ext(coeffs: &[Fp3], log_n: u32) -> Vec<Fp3> {
    let k = coeffs.len();
    let n = 1usize << log_n;
    let blowup = n / k;
    if blowup > 1 && blowup * k >= PAR_MIN {
        // three limbs, every coset, in one backend call; interleaved in threads
        let omega = root_of_unity(log_n);
        let shifts: Vec<u64> = (0..blowup).map(|s| omega.exp(s as u64).as_u64()).collect();
        let be = crate::backend::current();
        let limb = |f: fn(&Fp3) -> u64| coeffs.iter().map(f).collect::<Vec<u64>>();
        let l: Vec<Vec<Vec<u64>>> = [
            limb(|x| x.c0.as_u64()),
            limb(|x| x.c1.as_u64()),
            limb(|x| x.c2.as_u64()),
        ]
        .iter()
        .map(|v| be.coset_ntt(v, &shifts))
        .collect();
        return interleave_par(n, blowup, |s, u| {
            Fp3::new(
                Goldilocks::new(l[0][s][u]),
                Goldilocks::new(l[1][s][u]),
                Goldilocks::new(l[2][s][u]),
            )
        });
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn threaded_encodings_equal_the_serial_ones() {
        let k = 1usize << 11;
        let base: Vec<Goldilocks> = (0..k as u64).map(|i| Goldilocks::new(i * i + 3)).collect();
        let ext: Vec<Fp3> = (0..k as u64)
            .map(|i| {
                Fp3::new(
                    Goldilocks::new(i),
                    Goldilocks::new(i * 7 + 1),
                    Goldilocks::new(i ^ 99),
                )
            })
            .collect();
        let log_n = 15;
        assert_eq!(
            encode_base(&base, log_n),
            encode_base_with(&base, log_n, false)
        );
        let limb = |f: fn(&Fp3) -> Goldilocks| {
            encode_base_with(&ext.iter().map(f).collect::<Vec<_>>(), log_n, false)
        };
        let (a, b, c) = (limb(|x| x.c0), limb(|x| x.c1), limb(|x| x.c2));
        let want: Vec<Fp3> = (0..1usize << log_n)
            .map(|i| Fp3::new(a[i], b[i], c[i]))
            .collect();
        assert_eq!(encode_ext(&ext, log_n), want);
    }
}
