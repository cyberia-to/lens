//! cyb-lens-ikat — Ikat polynomial commitment.
//!
//! NTT-batched Brakedown over jali's R_q = F_p[x]/(x^n+1).
//! Commits NTT evaluation slots (Goldilocks scalars) via Brakedown.
//! Ring structure enables NTT batching and automorphism arguments.
//!
//! Ikat implements Lens<Goldilocks>: the NTT decomposition maps ring
//! operations to independent Goldilocks scalar operations, and the
//! commitment covers all n NTT slots simultaneously.
//!
//! See specs/polynomial-ring.md for the full specification.

pub use cyb_lens_core::{Commitment, Field, Lens, MultilinearPoly, Opening, Transcript};

use cyb_lens_brakedown::Brakedown;
use nebu::Goldilocks;

/// Ikat polynomial commitment — delegates to Brakedown over Goldilocks.
///
/// Ikat adds ring-awareness on top of Brakedown:
/// - NTT batching: multiple ring polynomial multiplies share one commitment
/// - Automorphism exploitation: Galois rotations as permutation arguments
/// - Noise tracking: running accumulator checked once at boundary
///
/// For the base commit/open/verify interface, Ikat delegates to Brakedown
/// since the NTT slots are Goldilocks scalars. Ring-specific optimizations
/// are applied at the constraint level (zheng), not at the commitment level.
pub struct Ikat;

impl Lens<Goldilocks> for Ikat {
    fn commit(poly: &MultilinearPoly<Goldilocks>) -> Commitment {
        Brakedown::commit(poly)
    }

    fn open(
        poly: &MultilinearPoly<Goldilocks>,
        point: &[Goldilocks],
        transcript: &mut Transcript,
    ) -> Opening {
        Brakedown::open(poly, point, transcript)
    }

    fn verify(
        commitment: &Commitment,
        point: &[Goldilocks],
        value: Goldilocks,
        proof: &Opening,
        transcript: &mut Transcript,
    ) -> bool {
        Brakedown::verify(commitment, point, value, proof, transcript)
    }

    fn batch_open(
        poly: &MultilinearPoly<Goldilocks>,
        points: &[(Vec<Goldilocks>, Goldilocks)],
        transcript: &mut Transcript,
    ) -> Opening {
        Brakedown::batch_open(poly, points, transcript)
    }

    fn batch_verify(
        commitment: &Commitment,
        points: &[(Vec<Goldilocks>, Goldilocks)],
        proof: &Opening,
        transcript: &mut Transcript,
    ) -> bool {
        Brakedown::batch_verify(commitment, points, proof, transcript)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ikat_delegates_to_brakedown() {
        let poly = MultilinearPoly::new(vec![
            Goldilocks::new(1),
            Goldilocks::new(2),
            Goldilocks::new(3),
            Goldilocks::new(4),
        ]);

        // Ikat and Brakedown should produce identical commitments
        let ikat_commit = Ikat::commit(&poly);
        let brakedown_commit = Brakedown::commit(&poly);
        assert_eq!(ikat_commit, brakedown_commit);
    }

    #[test]
    fn roundtrip() {
        let poly = MultilinearPoly::new(vec![
            Goldilocks::new(10),
            Goldilocks::new(20),
            Goldilocks::new(30),
            Goldilocks::new(40),
        ]);
        let commitment = Ikat::commit(&poly);

        let point = vec![Goldilocks::ZERO, Goldilocks::ONE];
        let value = poly.evaluate(&point);

        let mut pt = Transcript::new(b"ikat-test");
        let proof = Ikat::open(&poly, &point, &mut pt);

        let mut vt = Transcript::new(b"ikat-test");
        assert!(Ikat::verify(&commitment, &point, value, &proof, &mut vt));
    }
}
