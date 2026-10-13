//! TensorRs verifier.

use cyber_lens_core::{Commitment, PcsError, Transcript};
use nebu::Fp3;

use super::prove::powers;
use super::{TensorConfig, TensorRsParams, TensorRsProof, prologue};
use crate::field::{eq_table, mul_base, root_of_unity, univariate_ext_at_base};
use crate::merkle::{self, leaf_digest};
use crate::pow;
use crate::wire::base_bytes;

pub fn verify(
    params: &TensorRsParams,
    commitment: &Commitment,
    m: usize,
    point: &[Fp3],
    value: Fp3,
    proof: &TensorRsProof,
    transcript: &mut Transcript,
) -> Result<(), PcsError> {
    if proof.params != *params {
        return Err(PcsError::ParamsMismatch);
    }
    let c = TensorConfig::derive(params, m)?;
    let (k1, k2) = (c.k1(), c.k2());
    if point.len() != m
        || proof.row_test.len() != k2
        || proof.row_eval.len() != k2
        || proof.height != k1
        || proof.pow_nonce.is_some() != (c.pow_bits > 0)
    {
        return Err(PcsError::Shape);
    }

    prologue(transcript, params, m, commitment, point, value);
    let alpha = transcript.squeeze_fp3();
    transcript.absorb_fp3_slice(&proof.row_test);
    transcript.absorb_fp3_slice(&proof.row_eval);
    if !pow::check(transcript, c.pow_bits, proof.pow_nonce.unwrap_or(0)) {
        return Err(PcsError::ProofOfWork);
    }
    let idx = c.sample(transcript);
    if proof.columns.len() != idx.len() * k1 {
        return Err(PcsError::Shape);
    }

    // the evaluation claim: ⟨y, eq(z_lo, ·)⟩ = value
    let q2 = eq_table(&point[..c.col_vars]);
    let claimed = proof
        .row_eval
        .iter()
        .zip(&q2)
        .fold(Fp3::ZERO, |acc, (&y, &q)| acc + y * q);
    if claimed != value {
        return Err(PcsError::Evaluation);
    }

    // columns authenticate against the root
    let n = 1usize << c.log_n;
    let leaves = idx
        .iter()
        .zip(proof.columns.chunks_exact(k1))
        .map(|(&j, col)| leaf_digest(&base_bytes(col), j, n))
        .collect();
    merkle::verify(
        &commitment.0,
        c.log_n as usize,
        &idx,
        leaves,
        &proof.siblings,
    )?;

    // both linear checks on every opened column
    let r = powers(alpha, k1);
    let q1 = eq_table(&point[c.col_vars..]);
    let omega = root_of_unity(c.log_n);
    for (&j, col) in idx.iter().zip(proof.columns.chunks_exact(k1)) {
        let x = omega.exp(j as u64);
        let mut test = Fp3::ZERO;
        let mut eval = Fp3::ZERO;
        for ((&s, &ri), &qi) in col.iter().zip(&r).zip(&q1) {
            test += mul_base(ri, s);
            eval += mul_base(qi, s);
        }
        if univariate_ext_at_base(&proof.row_test, x) != test
            || univariate_ext_at_base(&proof.row_eval, x) != eval
        {
            return Err(PcsError::Consistency);
        }
    }
    Ok(())
}
