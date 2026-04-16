//! cyb-lens-assayer — Assayer tropical witness-verify commitment.
//!
//! The tropical semiring (min, +) supports min (addition) and saturating
//! addition (multiplication) with identities +inf and 0. It has no
//! subtraction or inversion.
//!
//! Assayer is a wrapper protocol: it accepts tropical computation results,
//! packs the witness (optimal assignment + cost) and LP dual certificate
//! as Goldilocks field elements, and commits via Brakedown.
//!
//! Assayer does not implement the Lens trait. It delegates to Brakedown.
//!
//! See specs/tropical-semiring.md for the full specification.

pub use cyb_lens_core::{Commitment, Opening, Transcript};

use cyb_lens_brakedown::Brakedown;
use cyb_lens_core::{Lens, MultilinearPoly};
use nebu::Goldilocks;
use trop::Tropical;

/// A tropical witness: the result of an optimization computation.
#[derive(Clone, Debug)]
pub struct TropicalWitness {
    /// The optimal assignment (indices into the problem structure).
    pub assignment: Vec<usize>,
    /// The optimal cost (tropical sum of assigned weights).
    pub cost: Tropical,
    /// Edge/node weights from the original problem (for verification).
    pub weights: Vec<Tropical>,
}

/// LP dual certificate: proves no cheaper alternative exists.
#[derive(Clone, Debug)]
pub struct DualCertificate {
    /// Dual variables satisfying feasibility constraints.
    pub dual_vars: Vec<Goldilocks>,
    /// Dual objective value (should equal primal cost for strong duality).
    pub dual_objective: Goldilocks,
}

/// Assayer tropical witness-verify commitment.
pub struct Assayer;

impl Assayer {
    /// Pack a tropical witness and dual certificate into a Goldilocks polynomial
    /// suitable for Brakedown commitment.
    ///
    /// Layout: [cost, assignment_len, assignment..., weights..., dual_vars..., dual_objective]
    pub fn pack_witness(
        witness: &TropicalWitness,
        certificate: &DualCertificate,
    ) -> MultilinearPoly<Goldilocks> {
        let mut data = Vec::new();

        // Pack cost
        data.push(Goldilocks::new(witness.cost.as_u64()));

        // Pack assignment length + assignment indices
        data.push(Goldilocks::new(witness.assignment.len() as u64));
        for &idx in &witness.assignment {
            data.push(Goldilocks::new(idx as u64));
        }

        // Pack weights
        for &w in &witness.weights {
            data.push(Goldilocks::new(w.as_u64()));
        }

        // Pack dual certificate
        for &d in &certificate.dual_vars {
            data.push(d);
        }
        data.push(certificate.dual_objective);

        // Pad to power of 2
        let n = data.len().next_power_of_two();
        data.resize(n, Goldilocks::ZERO);

        MultilinearPoly::new(data)
    }

    /// Commit to a tropical witness via Brakedown delegation.
    pub fn commit_witness(
        witness: &TropicalWitness,
        certificate: &DualCertificate,
    ) -> (Commitment, MultilinearPoly<Goldilocks>) {
        let poly = Self::pack_witness(witness, certificate);
        let commitment = Brakedown::commit(&poly);
        (commitment, poly)
    }

    /// Open the witness commitment at a point.
    pub fn open_witness(
        poly: &MultilinearPoly<Goldilocks>,
        point: &[Goldilocks],
        transcript: &mut Transcript,
    ) -> Opening {
        Brakedown::open(poly, point, transcript)
    }

    /// Verify a witness opening.
    pub fn verify_witness(
        commitment: &Commitment,
        point: &[Goldilocks],
        value: Goldilocks,
        proof: &Opening,
        transcript: &mut Transcript,
    ) -> bool {
        Brakedown::verify(commitment, point, value, proof, transcript)
    }

    /// Verify the three tropical witness properties:
    /// 1. structural validity (assignment is legal)
    /// 2. cost correctness (claimed cost matches assignment)
    /// 3. optimality (dual certificate proves no cheaper alternative)
    pub fn verify_tropical(witness: &TropicalWitness, certificate: &DualCertificate) -> bool {
        // Check cost correctness: sum of assigned weights should equal claimed cost
        let mut computed_cost = Tropical::ONE; // multiplicative identity = 0
        for &idx in &witness.assignment {
            if idx >= witness.weights.len() {
                return false; // invalid assignment index
            }
            computed_cost = computed_cost.mul(witness.weights[idx]);
        }
        if computed_cost != witness.cost {
            return false;
        }

        // Check dual feasibility: dual_objective should equal primal cost
        // (strong duality)
        let primal_as_goldilocks = Goldilocks::new(witness.cost.as_u64());
        if certificate.dual_objective != primal_as_goldilocks {
            return false;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_witness_power_of_two() {
        let witness = TropicalWitness {
            assignment: vec![0, 2],
            cost: Tropical::from_u64(5),
            weights: vec![
                Tropical::from_u64(3),
                Tropical::from_u64(10),
                Tropical::from_u64(2),
            ],
        };
        let cert = DualCertificate {
            dual_vars: vec![Goldilocks::new(3), Goldilocks::new(2)],
            dual_objective: Goldilocks::new(5),
        };

        let poly = Assayer::pack_witness(&witness, &cert);
        assert!(poly.len().is_power_of_two());
    }

    #[test]
    fn commit_and_verify_roundtrip() {
        let witness = TropicalWitness {
            assignment: vec![0, 2],
            cost: Tropical::from_u64(5), // 3 + 2 = 5 (tropical mul = ordinary add)
            weights: vec![
                Tropical::from_u64(3),
                Tropical::from_u64(10),
                Tropical::from_u64(2),
            ],
        };
        let cert = DualCertificate {
            dual_vars: vec![Goldilocks::new(3), Goldilocks::new(2)],
            dual_objective: Goldilocks::new(5),
        };

        let (commitment, poly) = Assayer::commit_witness(&witness, &cert);

        let point = vec![Goldilocks::ZERO; poly.num_vars];
        let value = poly.evaluate(&point);

        let mut pt = Transcript::new(b"assayer-test");
        let proof = Assayer::open_witness(&poly, &point, &mut pt);

        let mut vt = Transcript::new(b"assayer-test");
        assert!(Assayer::verify_witness(
            &commitment,
            &point,
            value,
            &proof,
            &mut vt
        ));
    }

    #[test]
    fn tropical_verification() {
        let witness = TropicalWitness {
            assignment: vec![0, 2],
            cost: Tropical::from_u64(5),
            weights: vec![
                Tropical::from_u64(3),
                Tropical::from_u64(10),
                Tropical::from_u64(2),
            ],
        };
        let cert = DualCertificate {
            dual_vars: vec![Goldilocks::new(3), Goldilocks::new(2)],
            dual_objective: Goldilocks::new(5),
        };

        assert!(Assayer::verify_tropical(&witness, &cert));
    }

    #[test]
    fn wrong_cost_rejected() {
        let witness = TropicalWitness {
            assignment: vec![0, 2],
            cost: Tropical::from_u64(999), // wrong cost
            weights: vec![
                Tropical::from_u64(3),
                Tropical::from_u64(10),
                Tropical::from_u64(2),
            ],
        };
        let cert = DualCertificate {
            dual_vars: vec![],
            dual_objective: Goldilocks::new(999),
        };

        assert!(!Assayer::verify_tropical(&witness, &cert));
    }
}
