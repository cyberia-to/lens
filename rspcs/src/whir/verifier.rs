//! WHIR verifier.

use cyber_lens_core::{Commitment, PcsError, Transcript};
use nebu::Fp3;

use super::fold::{check_opening, shift_point};
use super::proof::WhirProof;
use super::{WhirConfig, WhirParams, prologue, sumcheck};
use crate::field::{coeff_ml_eval, eq_eval, pow_point, univariate_ext_at_base};
use crate::pow;

/// A weight `coef · Z · eq(point, X)` added in round `round`.
struct Constraint {
    round: usize,
    point: Vec<Fp3>,
    coef: Fp3,
}

fn nonce(v: &[u64], bits: u32) -> Result<u64, PcsError> {
    match (bits > 0, v) {
        (true, [n]) => Ok(*n),
        (false, []) => Ok(0),
        _ => Err(PcsError::Shape),
    }
}

pub fn verify(
    params: &WhirParams,
    commitment: &Commitment,
    m: usize,
    point: &[Fp3],
    value: Fp3,
    proof: &WhirProof,
    t: &mut Transcript,
) -> Result<(), PcsError> {
    if proof.params != *params {
        return Err(PcsError::ParamsMismatch);
    }
    let cfg = WhirConfig::derive(params, m)?;
    let spec0 = cfg.rounds[0];
    if point.len() != m
        || proof.ood0.len() != spec0.ood
        || proof.rounds.len() + 1 != cfg.rounds.len()
        || proof.final_poly.len() != 1 << cfg.final_vars
    {
        return Err(PcsError::Shape);
    }
    prologue(t, params, m, commitment, point, value);

    let mut cons = vec![Constraint {
        round: 0,
        point: point.to_vec(),
        coef: Fp3::ONE,
    }];
    let mut zs = Vec::with_capacity(spec0.ood);
    for &y in &proof.ood0 {
        zs.push(t.squeeze_fp3());
        t.absorb_fp3(y);
    }
    let gamma = t.squeeze_fp3();
    let mut sigma = value;
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

    let mut prev_root = commitment.0;
    let mut prev_spec = spec0;
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
        let folded = check_opening(
            &prev_root,
            &prev_spec,
            i > 1,
            &idx,
            &rp.opening,
            prev_alphas,
        )?;

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
        prev_root = rp.root;
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
    let last_ext = !proof.rounds.is_empty();
    let folded = check_opening(
        &prev_root,
        &prev_spec,
        last_ext,
        &idx,
        &proof.final_opening,
        prev_alphas,
    )?;
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
    if expected != sigma {
        return Err(PcsError::Sumcheck);
    }
    Ok(())
}
