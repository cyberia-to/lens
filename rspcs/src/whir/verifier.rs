//! WHIR verifier.

use cyber_lens_core::{Commitment, PcsError, Transcript};
use nebu::Fp3;

use super::fold::{check_opening, shift_point};
use super::proof::{Opening, WhirProof};
use super::{WhirConfig, WhirParams, prologue, sumcheck};
use crate::field::{coeff_ml_eval, eq_eval, pow_point, univariate_ext_at_base};
use crate::pow;

/// A weight `coef · eq(point, X)` added in round `round`.
pub(crate) struct Constraint {
    pub round: usize,
    pub point: Vec<Fp3>,
    pub coef: Fp3,
}

fn nonce(v: &[u64], bits: u32) -> Result<u64, PcsError> {
    match (bits > 0, v) {
        (true, [n]) => Ok(*n),
        (false, []) => Ok(0),
        _ => Err(PcsError::Shape),
    }
}

/// Authenticates opened leaves of `f_0` and returns their folded values.
pub(crate) type CheckOpening0<'a> =
    &'a dyn Fn(&[usize], &Opening, &[Fp3]) -> Result<Vec<Fp3>, PcsError>;

#[allow(clippy::too_many_arguments)]
pub fn verify(
    params: &WhirParams,
    commitment: &Commitment,
    m: usize,
    point: &[Fp3],
    value: Fp3,
    proof: &WhirProof,
    t: &mut Transcript,
    ext0: bool,
) -> Result<(), PcsError> {
    if proof.params != *params {
        return Err(PcsError::ParamsMismatch);
    }
    let cfg = WhirConfig::derive(params, m)?;
    if point.len() != m {
        return Err(PcsError::Shape);
    }
    shape(&cfg, proof)?;
    prologue(t, params, m, commitment, point, value, ext0);
    let cons = vec![Constraint {
        round: 0,
        point: point.to_vec(),
        coef: Fp3::ONE,
    }];
    let spec0 = cfg.rounds[0];
    let root = commitment.0;
    verify_rounds(&cfg, t, cons, &[], value, proof, &|idx, opening, alphas| {
        check_opening(&root, &spec0, ext0, idx, opening, alphas)
    })
}

/// Lengths a proof must have under `cfg`.
pub(crate) fn shape(cfg: &WhirConfig, proof: &WhirProof) -> Result<(), PcsError> {
    if proof.ood0.len() != cfg.rounds[0].ood
        || proof.rounds.len() + 1 != cfg.rounds.len()
        || proof.final_poly.len() != 1 << cfg.final_vars
    {
        return Err(PcsError::Shape);
    }
    Ok(())
}

/// WHIR from step 2 of the transcript on: `cons` holds the claim's eq
/// terms (round 0), `explicit` its entries at hypercube points, `sigma`
/// its value; `check0` authenticates and folds openings of `f_0`.
pub(crate) fn verify_rounds(
    cfg: &WhirConfig,
    t: &mut Transcript,
    mut cons: Vec<Constraint>,
    explicit: &[(usize, Fp3)],
    mut sigma: Fp3,
    proof: &WhirProof,
    check0: CheckOpening0<'_>,
) -> Result<(), PcsError> {
    let m = cfg.num_vars;
    let spec0 = cfg.rounds[0];
    let mut zs = Vec::with_capacity(spec0.ood);
    for &y in &proof.ood0 {
        zs.push(t.squeeze_fp3());
        t.absorb_fp3(y);
    }
    let gamma = t.squeeze_fp3();
    let mut g = gamma;
    for (&z, &y) in zs.iter().zip(&proof.ood0) {
        sigma += g * y;
        cons.push(Constraint {
            round: 0,
            point: pow_point(z, m),
            coef: g,
        });
        g *= gamma;
    }
    let mut alphas = sumcheck::verify(
        t,
        &mut sigma,
        &proof.sumcheck0,
        &proof.fold_nonces0,
        spec0.fold,
        spec0.fold_pow,
    )?;

    let mut prev_root: Option<cyber_hemera::Hash> = None;
    let mut prev_spec = spec0;
    let fold_prev = |prev_root: &Option<cyber_hemera::Hash>,
                     prev_spec: &super::RoundSpec,
                     idx: &[usize],
                     opening: &Opening,
                     alphas: &[Fp3]| match prev_root {
        Some(root) => check_opening(root, prev_spec, true, idx, opening, alphas),
        None => check0(idx, opening, alphas),
    };
    for (i, rp) in proof.rounds.iter().enumerate() {
        let i = i + 1;
        let spec = cfg.rounds[i];
        if rp.ood.len() != spec.ood {
            return Err(PcsError::Shape);
        }
        t.absorb(rp.root.as_bytes());
        let mut zs = Vec::with_capacity(spec.ood);
        for &y in &rp.ood {
            zs.push(t.squeeze_fp3());
            t.absorb_fp3(y);
        }
        if !pow::check(
            t,
            prev_spec.query_pow,
            nonce(&rp.query_nonce, prev_spec.query_pow)?,
        ) {
            return Err(PcsError::ProofOfWork);
        }
        let idx = prev_spec.sample(t);
        let gamma = t.squeeze_fp3();
        let prev_alphas = &alphas[alphas.len() - prev_spec.fold..];
        let folded = fold_prev(&prev_root, &prev_spec, &idx, &rp.opening, prev_alphas)?;

        let mut g = gamma;
        for (&z, &y) in zs.iter().zip(&rp.ood) {
            sigma += g * y;
            cons.push(Constraint {
                round: i,
                point: pow_point(z, spec.num_vars),
                coef: g,
            });
            g *= gamma;
        }
        for (&j, &v) in idx.iter().zip(&folded) {
            sigma += g * v;
            cons.push(Constraint {
                round: i,
                point: pow_point(Fp3::from_base(shift_point(&prev_spec, j)), spec.num_vars),
                coef: g,
            });
            g *= gamma;
        }
        alphas.extend(sumcheck::verify(
            t,
            &mut sigma,
            &rp.sumcheck,
            &rp.fold_nonces,
            spec.fold,
            spec.fold_pow,
        )?);
        prev_root = Some(rp.root);
        prev_spec = spec;
    }

    // final polynomial, final queries
    t.absorb_fp3_slice(&proof.final_poly);
    if !pow::check(
        t,
        prev_spec.query_pow,
        nonce(&proof.final_nonce, prev_spec.query_pow)?,
    ) {
        return Err(PcsError::ProofOfWork);
    }
    let idx = prev_spec.sample(t);
    let prev_alphas = &alphas[alphas.len() - prev_spec.fold..];
    let folded = fold_prev(&prev_root, &prev_spec, &idx, &proof.final_opening, prev_alphas)?;
    for (&j, &v) in idx.iter().zip(&folded) {
        if univariate_ext_at_base(&proof.final_poly, shift_point(&prev_spec, j)) != v {
            return Err(PcsError::Consistency);
        }
    }

    // closing check: σ = Σ_b ŵ(f̂_M(b), α, b)
    let fv = cfg.final_vars;
    let mut expected = Fp3::ZERO;
    for c in &cons {
        let off = cfg.alpha_offset(c.round);
        let pre = c.point.len() - fv;
        let e = eq_eval(&c.point[..pre], &alphas[off..off + pre]);
        expected += c.coef * e * coeff_ml_eval(&proof.final_poly, &c.point[pre..]);
    }
    if !explicit.is_empty() {
        // entries at hypercube points b: eq(b_low, α) · f_M(b_high), with
        // f_M's table on the hypercube computed once
        let pre = m - fv;
        let fin = hypercube_table(&proof.final_poly);
        for &(b, v) in explicit {
            let e = (0..pre).fold(Fp3::ONE, |acc, l| {
                if (b >> l) & 1 == 1 { acc * alphas[l] } else { acc * (Fp3::ONE - alphas[l]) }
            });
            expected += v * e * fin[b >> pre];
        }
    }
    if expected != sigma {
        return Err(PcsError::Sumcheck);
    }
    Ok(())
}

/// Monomial coefficients → values on the hypercube (`e_s = Σ_{b ⊆ s} c_b`).
fn hypercube_table(coeffs: &[Fp3]) -> Vec<Fp3> {
    let mut a = coeffs.to_vec();
    let n = a.len();
    let mut step = 1;
    while step < n {
        for block in a.chunks_exact_mut(2 * step) {
            let (lo, hi) = block.split_at_mut(step);
            for (h, l) in hi.iter_mut().zip(lo.iter()) {
                *h += *l;
            }
        }
        step *= 2;
    }
    a
}
