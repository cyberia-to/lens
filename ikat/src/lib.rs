//! cyb-lens-ikat — Ikat polynomial commitment.
//!
//! Ring-aware commitment over jali's R_q = F_p[x]/(x^n+1).
//! NTT decomposes ring operations into n independent Goldilocks operations.
//! Ikat commits the NTT evaluation slots via Brakedown, batching multiple
//! ring elements into a single commitment.
//!
//! The ring-awareness shows in how data is prepared: ring elements are
//! converted to NTT form before commitment, and multiple ring polynomials
//! can be batched into one multilinear polynomial over the NTT slots.
//!
//! Implements Lens<Goldilocks> — the NTT slots are Goldilocks scalars.
//!
//! See specs/polynomial-ring.md for the full specification.

pub use cyb_lens_core::{Commitment, Field, Lens, MultilinearPoly, Opening, Transcript};

use cyb_lens_brakedown::Brakedown;
use jali::ntt;
use jali::ring::RingElement;
use nebu::Goldilocks;

/// Ikat polynomial commitment — ring-aware Brakedown over NTT slots.
pub struct Ikat;

impl Ikat {
    /// Convert a ring element to its NTT slot representation.
    /// Returns n Goldilocks scalars (the NTT evaluation points).
    pub fn ring_to_ntt_slots(ring_elem: &RingElement) -> Vec<Goldilocks> {
        let mut elem = ring_elem.clone();
        if !elem.is_ntt {
            ntt::to_ntt(&mut elem);
        }
        elem.coeffs[..elem.n].to_vec()
    }

    /// Batch multiple ring elements into a single multilinear polynomial.
    ///
    /// Given m ring elements of dimension n, produces a multilinear polynomial
    /// of size m × n over Goldilocks. The NTT structure means all n slots
    /// of each ring element are committed together, enabling ring-aware
    /// verification.
    pub fn batch_rings(elements: &[RingElement]) -> MultilinearPoly<Goldilocks> {
        assert!(!elements.is_empty());
        let n = elements[0].n;

        let mut all_slots = Vec::with_capacity(elements.len() * n);
        for elem in elements {
            assert_eq!(elem.n, n, "all ring elements must have same dimension");
            all_slots.extend_from_slice(&Self::ring_to_ntt_slots(elem));
        }

        // Pad to power of 2
        let total = all_slots.len().next_power_of_two();
        all_slots.resize(total, Goldilocks::ZERO);

        MultilinearPoly::new(all_slots)
    }

    /// Commit to a batch of ring elements.
    pub fn commit_rings(elements: &[RingElement]) -> (Commitment, MultilinearPoly<Goldilocks>) {
        let poly = Self::batch_rings(elements);
        let commitment = Brakedown::commit(&poly);
        (commitment, poly)
    }
}

/// Ikat delegates to Brakedown for the base Lens operations.
/// The ring-awareness is in how data enters the system (via ring_to_ntt_slots
/// and batch_rings), not in the commit/open/verify mechanics.
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
    fn ntt_slots_roundtrip() {
        // Create a ring element, convert to NTT slots, verify dimensions
        let mut elem = RingElement::new(1024);
        elem.coeffs[0] = Goldilocks::new(1);
        elem.coeffs[1] = Goldilocks::new(2);

        let slots = Ikat::ring_to_ntt_slots(&elem);
        assert_eq!(slots.len(), 1024);
    }

    #[test]
    fn batch_rings_power_of_two() {
        let elem1 = RingElement::new(1024);
        let elem2 = RingElement::new(1024);
        let poly = Ikat::batch_rings(&[elem1, elem2]);
        assert!(poly.len().is_power_of_two());
        assert!(poly.len() >= 2048); // 2 elements × 1024 slots
    }

    #[test]
    fn commit_rings_works() {
        let mut elem = RingElement::new(1024);
        elem.coeffs[0] = Goldilocks::new(42);
        let (commitment, poly) = Ikat::commit_rings(&[elem]);

        // Commitment should be deterministic
        let mut elem2 = RingElement::new(1024);
        elem2.coeffs[0] = Goldilocks::new(42);
        let (commitment2, _) = Ikat::commit_rings(&[elem2]);
        assert_eq!(commitment, commitment2);

        // Should be verifiable via normal Lens interface
        let point = vec![Goldilocks::ZERO; poly.num_vars];
        let value = poly.evaluate(&point);

        let mut pt = Transcript::new(b"ikat-ring");
        let proof = Ikat::open(&poly, &point, &mut pt);
        let mut vt = Transcript::new(b"ikat-ring");
        assert!(Ikat::verify(&commitment, &point, value, &proof, &mut vt));
    }

    #[test]
    fn ikat_brakedown_equivalence_on_raw_poly() {
        // For raw multilinear polys, Ikat and Brakedown are identical
        let evals: Vec<Goldilocks> = (0..16).map(|i| Goldilocks::new(i + 1)).collect();
        let poly = MultilinearPoly::new(evals);
        assert_eq!(Ikat::commit(&poly), Brakedown::commit(&poly));
    }

    #[test]
    fn roundtrip_4var() {
        let evals: Vec<Goldilocks> = (0..16).map(|i| Goldilocks::new(i * 7 + 5)).collect();
        let poly = MultilinearPoly::new(evals);
        let point: Vec<Goldilocks> = (0..4).map(|i| Goldilocks::new(i + 2)).collect();
        let commitment = Ikat::commit(&poly);
        let value = poly.evaluate(&point);

        let mut pt = Transcript::new(b"ikat-4");
        let proof = Ikat::open(&poly, &point, &mut pt);
        let mut vt = Transcript::new(b"ikat-4");
        assert!(Ikat::verify(&commitment, &point, value, &proof, &mut vt));
    }
}
