//! Multilinear polynomial commitment contract over Goldilocks with Fp3
//! challenges.
//!
//! This is the interface zheng consumes. A scheme commits to the
//! evaluation table of a multilinear polynomial over Goldilocks
//! (`evals.len() = 2^num_vars`, entry `i` is the value at the boolean
//! point whose `k`-th coordinate is bit `k` of `i`) and opens it at a point
//! of the cubic extension `Fp3 = F_p[t]/(t³ − t − 1)`.
//!
//! Security parameters are data (`Params`) and every proof carries a
//! header the verifier compares against its own parameters.
//! `security_bits` is the proven round-by-round soundness of the
//! non-interactive opening for the given parameters and size; it never
//! includes conjectured bounds.

use nebu::{Fp3, Goldilocks};

use crate::{Commitment, Transcript};

/// Why a verifier rejected an opening.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PcsError {
    /// The proof was produced under different parameters.
    ParamsMismatch,
    /// Size outside the supported range or inconsistent with the proof.
    Shape,
    /// The serialized proof is malformed or non-canonical.
    Malformed,
    /// A Merkle authentication path did not reproduce the root.
    Merkle,
    /// A proof-of-work nonce does not meet its difficulty.
    ProofOfWork,
    /// A sumcheck round polynomial is inconsistent with its claim.
    Sumcheck,
    /// A queried codeword symbol is inconsistent with the folded or
    /// combined value it must equal.
    Consistency,
    /// The opened value does not match the claimed evaluation.
    Evaluation,
}

impl core::fmt::Display for PcsError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let s = match self {
            PcsError::ParamsMismatch => "proof parameters differ from verifier parameters",
            PcsError::Shape => "size outside supported range or inconsistent with proof",
            PcsError::Malformed => "malformed or non-canonical proof encoding",
            PcsError::Merkle => "merkle authentication failed",
            PcsError::ProofOfWork => "proof-of-work nonce below difficulty",
            PcsError::Sumcheck => "sumcheck round inconsistent",
            PcsError::Consistency => "queried symbol inconsistent",
            PcsError::Evaluation => "claimed evaluation mismatch",
        };
        f.write_str(s)
    }
}

impl std::error::Error for PcsError {}

/// A multilinear polynomial commitment over Goldilocks, opened at Fp3
/// points.
pub trait MultilinearPcs {
    /// Scheme identifier, absorbed into every transcript.
    const NAME: &'static str;
    /// Security parameters: rate, folding factor, query policy, grinding
    /// bits, security target. Printed into proofs as a checked header.
    type Params: Clone + core::fmt::Debug;
    /// Prover-side state kept between `commit` and `open`.
    type ProverData;
    /// The opening proof.
    type Proof: Clone + core::fmt::Debug;

    /// Commit to `evals` (`evals.len() = 2^num_vars`).
    fn commit(params: &Self::Params, evals: &[Goldilocks]) -> (Commitment, Self::ProverData);

    /// Open the committed polynomial at `point`; returns the value and
    /// the proof.
    fn open(
        params: &Self::Params,
        data: &Self::ProverData,
        point: &[Fp3],
        transcript: &mut Transcript,
    ) -> (Fp3, Self::Proof);

    /// Verify that the polynomial committed in `commitment` has
    /// `num_vars` variables and evaluates to `value` at `point`.
    fn verify(
        params: &Self::Params,
        commitment: &Commitment,
        num_vars: usize,
        point: &[Fp3],
        value: Fp3,
        proof: &Self::Proof,
        transcript: &mut Transcript,
    ) -> Result<(), PcsError>;

    /// Proven soundness in bits for these params at this size (no
    /// conjectures).
    fn security_bits(params: &Self::Params, num_vars: usize) -> f64;
}
