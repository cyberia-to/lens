//! WHIR multilinear PCS (Arnon, Chiesa, Fenzi, Yogev — "WHIR: Reed–Solomon
//! Proximity Testing with Super-Fast Verification", eprint 2024/1586).
//!
//! Commit: monomial coefficients of the multilinear `f` (Möbius transform
//! of the table) RS-encoded on the order-`2^{m+r}` subgroup; leaves are the
//! `2^{k_0}` symbols of one folding coset; hemera Merkle root.
//!
//! Open at `z` with value `v`: OOD samples bind the commitment (Theorem
//! 7.5), the evaluation and OOD weights are combined (Construction 5.5),
//! then Construction 5.1: per round, `k` sumcheck rounds with folding
//! challenges, a fresh RS commitment of the folded polynomial, OOD
//! samples, shift queries on the previous function through `Fold`, and a
//! γ-combination of the new weights; finally the folded polynomial in the
//! clear, final queries and the closing weighted-sum check.
//! See `specs/whir.md`.

mod access;
pub mod batch;
mod config;
mod fold;
mod proof;
mod prover;
mod sumcheck;
mod verifier;

pub use config::{RoundSpec, WhirConfig};
pub use proof::{Opening, WhirProof, WhirRoundProof};
pub use access::{LeafLayout, verify_leaves};
pub use prover::WhirData;

#[cfg(test)]
pub(crate) use fold::fold_leaf as fold_leaf_for_tests;
#[cfg(test)]
pub(crate) use prover::open_with as open_unchecked_for_tests;
#[cfg(test)]
pub(crate) fn prover_message_base(
    evals: Vec<nebu::Goldilocks>,
    coeffs: Vec<nebu::Goldilocks>,
) -> prover::Message {
    prover::Message::Base { evals, coeffs }
}

use cyber_lens_core::{Commitment, MultilinearPcs, PcsError, Transcript};
use nebu::{Fp3, Goldilocks};

/// Decoding regime policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decoding {
    /// Unique-decoding radius, list size 1, no OOD samples.
    Unique,
    /// Up to the Johnson radius with a per-round integer trade-off `m ≥ 3`
    /// chosen to minimise queries within the grinding budget.
    Johnson,
}

/// Security parameters for WHIR.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WhirParams {
    /// Starting rate `ρ_0 = 2^-log_inv_rate`.
    pub log_inv_rate: u8,
    /// Variables folded per round (`k`).
    pub folding_factor: u8,
    /// Target round-by-round soundness in bits.
    pub security_target: u16,
    /// Grinding budget per challenge, in bits.
    pub pow_bits: u8,
    pub decoding: Decoding,
    /// Stop committing once at most this many variables remain; the rest
    /// of the polynomial is sent in the clear.
    pub max_final_vars: u8,
}

impl Default for WhirParams {
    fn default() -> Self {
        Self {
            log_inv_rate: 4,
            folding_factor: 4,
            security_target: 128,
            pow_bits: 16,
            decoding: Decoding::Johnson,
            max_final_vars: 8,
        }
    }
}

const VERSION: u8 = 1;

impl WhirParams {
    /// 8-byte header: version, log_inv_rate, folding_factor, pow_bits,
    /// target (u16 LE), decoding (0 unique, 1 Johnson), max_final_vars.
    pub fn header(&self) -> [u8; 8] {
        let t = self.security_target.to_le_bytes();
        let d = match self.decoding {
            Decoding::Unique => 0,
            Decoding::Johnson => 1,
        };
        [
            VERSION,
            self.log_inv_rate,
            self.folding_factor,
            self.pow_bits,
            t[0],
            t[1],
            d,
            self.max_final_vars,
        ]
    }

    pub fn from_header(h: [u8; 8]) -> Result<Self, PcsError> {
        let decoding = match h[6] {
            0 => Decoding::Unique,
            1 => Decoding::Johnson,
            _ => return Err(PcsError::Malformed),
        };
        if h[0] != VERSION {
            return Err(PcsError::Malformed);
        }
        Ok(Self {
            log_inv_rate: h[1],
            folding_factor: h[2],
            pow_bits: h[3],
            security_target: u16::from_le_bytes([h[4], h[5]]),
            decoding,
            max_final_vars: h[7],
        })
    }
}

/// The WHIR polynomial commitment.
#[derive(Clone, Copy, Debug)]
pub struct Whir;

impl Whir {
    /// Commit an Fp3-valued table (`evals.len() = 2^num_vars`). Same
    /// domain, rate and leaf layout as a base commitment; the code is
    /// `RS[K, L, 2^m]` and every proven term of `security_bits` holds
    /// unchanged (all of them are already over `K`).
    pub fn commit_ext(params: &WhirParams, evals: &[Fp3]) -> (Commitment, WhirData) {
        prover::commit_ext(params, evals)
    }

    /// Verify an opening of an Fp3 commitment (`commit_ext`).
    #[allow(clippy::too_many_arguments)]
    pub fn verify_ext(
        params: &WhirParams,
        commitment: &Commitment,
        num_vars: usize,
        point: &[Fp3],
        value: Fp3,
        proof: &WhirProof,
        transcript: &mut Transcript,
    ) -> Result<(), PcsError> {
        verifier::verify(
            params, commitment, num_vars, point, value, proof, transcript, true,
        )
    }
}

impl MultilinearPcs for Whir {
    const NAME: &'static str = "lens-whir-rs-v1";
    type Params = WhirParams;
    type ProverData = WhirData;
    type Proof = WhirProof;

    fn commit(params: &Self::Params, evals: &[Goldilocks]) -> (Commitment, Self::ProverData) {
        prover::commit(params, evals)
    }

    fn open(
        params: &Self::Params,
        data: &Self::ProverData,
        point: &[Fp3],
        transcript: &mut Transcript,
    ) -> (Fp3, Self::Proof) {
        prover::open(params, data, point, transcript)
    }

    fn verify(
        params: &Self::Params,
        commitment: &Commitment,
        num_vars: usize,
        point: &[Fp3],
        value: Fp3,
        proof: &Self::Proof,
        transcript: &mut Transcript,
    ) -> Result<(), PcsError> {
        verifier::verify(
            params, commitment, num_vars, point, value, proof, transcript, false,
        )
    }

    fn security_bits(params: &Self::Params, num_vars: usize) -> f64 {
        WhirConfig::derive(params, num_vars).map_or(0.0, |c| c.security_bits())
    }
}

pub(crate) fn prologue(
    t: &mut Transcript,
    params: &WhirParams,
    m: usize,
    root: &Commitment,
    point: &[Fp3],
    value: Fp3,
    ext0: bool,
) {
    t.absorb(Whir::NAME.as_bytes());
    if ext0 {
        // an Fp3 commitment opens on a transcript no base opening shares
        t.absorb(b"ext");
    }
    t.absorb(&params.header());
    t.absorb_u64(m as u64);
    t.absorb(root.as_bytes());
    t.absorb_fp3_slice(point);
    t.absorb_fp3(value);
}
