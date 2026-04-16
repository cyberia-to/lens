//! cyb-lens-core — Lens trait, types, and transcript for polynomial commitment.
//!
//! This crate defines the shared interface that all polynomial commitment
//! constructions implement. Consumers (nox, zheng, bbg) depend on this crate
//! for the trait; they depend on a specific construction crate only when
//! instantiating it.

mod transcript;
mod types;

pub use strata_proof::Reduce;
pub use transcript::Transcript;
pub use types::{Commitment, Field, MultilinearPoly, Opening, Ring, Semiring};

/// Polynomial commitment scheme — commit to a multilinear polynomial,
/// prove evaluations, verify without seeing the polynomial.
///
/// Four constructions implement this trait directly:
/// - Brakedown (cyb-lens-brakedown) over Goldilocks
/// - Binius (cyb-lens-binius) over F₂¹²⁸
/// - Ikat (cyb-lens-ikat) over Goldilocks (NTT slots)
/// - Porphyry (cyb-lens-porphyry) over F_q
///
/// Assayer (cyb-lens-assayer) is a wrapper protocol that delegates
/// commitment to Brakedown — it does not implement this trait.
pub trait Lens<F: Field> {
    /// Commit to a multilinear polynomial.
    /// Returns a 32-byte hemera digest.
    /// Cost: O(N) field operations where N = 2^num_vars.
    fn commit(poly: &MultilinearPoly<F>) -> Commitment;

    /// Produce a proof that poly(point) = value.
    fn open(poly: &MultilinearPoly<F>, point: &[F], transcript: &mut Transcript) -> Opening;

    /// Check that a committed polynomial evaluates to value at point.
    fn verify(
        commitment: &Commitment,
        point: &[F],
        value: F,
        proof: &Opening,
        transcript: &mut Transcript,
    ) -> bool;

    /// Amortize multiple openings into one proof.
    fn batch_open(
        poly: &MultilinearPoly<F>,
        points: &[(Vec<F>, F)],
        transcript: &mut Transcript,
    ) -> Opening;

    /// Verify a batch opening.
    fn batch_verify(
        commitment: &Commitment,
        points: &[(Vec<F>, F)],
        proof: &Opening,
        transcript: &mut Transcript,
    ) -> bool;
}
