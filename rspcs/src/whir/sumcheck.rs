//! Weighted sumcheck rounds `Σ_b f(b)·w(b) = σ` with degree-2 round
//! polynomials sent as `(h(0), h(2))`; `h(1) = σ − h(0)` is implied.

use cyber_lens_core::{PcsError, Transcript};
use nebu::{Fp3, Goldilocks};

use crate::field::{fold_evals_ext, quadratic_at};
use crate::par::{par_map_min, threads};
use crate::pow;

/// `(h(0), h(1), h(2))` for the first variable of `f·w`.
pub fn round_poly(f: &[Fp3], w: &[Fp3]) -> (Fp3, Fp3, Fp3) {
    let pairs = f.len() / 2;
    let two = Fp3::from_base(Goldilocks::new(2));
    let part = |lo: usize, hi: usize| {
        let (mut a, mut b, mut c) = (Fp3::ZERO, Fp3::ZERO, Fp3::ZERO);
        for i in lo..hi {
            let (f0, f1, w0, w1) = (f[2 * i], f[2 * i + 1], w[2 * i], w[2 * i + 1]);
            a += f0 * w0;
            b += f1 * w1;
            c += (two * f1 - f0) * (two * w1 - w0);
        }
        (a, b, c)
    };
    let chunks = if pairs >= 1 << 14 { threads() } else { 1 };
    let size = pairs.div_ceil(chunks);
    par_map_min(chunks, 2, |k| part(k * size, ((k + 1) * size).min(pairs)))
        .into_iter()
        .fold((Fp3::ZERO, Fp3::ZERO, Fp3::ZERO), |acc, x| {
            (acc.0 + x.0, acc.1 + x.1, acc.2 + x.2)
        })
}

/// Prover: run `rounds` folding rounds; returns the challenges. Folds
/// `f` and `w` in place; `on_alpha` folds any companion table.
#[allow(clippy::too_many_arguments)]
pub fn prove(
    t: &mut Transcript,
    f: &mut Vec<Fp3>,
    w: &mut Vec<Fp3>,
    rounds: usize,
    fold_pow: u32,
    msgs: &mut Vec<Fp3>,
    nonces: &mut Vec<u64>,
    mut on_alpha: impl FnMut(Fp3),
) -> Vec<Fp3> {
    let mut alphas = Vec::with_capacity(rounds);
    for _ in 0..rounds {
        let (h0, _h1, h2) = round_poly(f, w);
        msgs.push(h0);
        msgs.push(h2);
        t.absorb_fp3(h0);
        t.absorb_fp3(h2);
        nonces.extend(pow::grind(t, fold_pow));
        let alpha = t.squeeze_fp3();
        fold_evals_ext(f, alpha);
        fold_evals_ext(w, alpha);
        on_alpha(alpha);
        alphas.push(alpha);
    }
    alphas
}

/// Verifier: consume `rounds` messages; updates `sigma`, returns the
/// challenges.
pub fn verify(
    t: &mut Transcript,
    sigma: &mut Fp3,
    msgs: &[Fp3],
    nonces: &[u64],
    rounds: usize,
    fold_pow: u32,
) -> Result<Vec<Fp3>, PcsError> {
    let expect_nonces = if fold_pow > 0 { rounds } else { 0 };
    if msgs.len() != 2 * rounds || nonces.len() != expect_nonces {
        return Err(PcsError::Shape);
    }
    let mut alphas = Vec::with_capacity(rounds);
    for l in 0..rounds {
        let (h0, h2) = (msgs[2 * l], msgs[2 * l + 1]);
        let h1 = *sigma - h0;
        t.absorb_fp3(h0);
        t.absorb_fp3(h2);
        if !pow::check(t, fold_pow, nonces.get(l).copied().unwrap_or(0)) {
            return Err(PcsError::ProofOfWork);
        }
        let alpha = t.squeeze_fp3();
        *sigma = quadratic_at(h0, h1, h2, alpha);
        alphas.push(alpha);
    }
    Ok(alphas)
}
