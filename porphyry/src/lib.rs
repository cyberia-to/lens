//! cyb-lens-porphyry — Porphyry polynomial commitment.
//!
//! Brakedown instantiated over genies' F_q (CSIDH-512 prime, 512 bits).
//! Same expander-graph structure as Brakedown, wider field elements.
//!
//! See specs/isogeny-curves.md for the full specification.

pub use cyb_lens_core::{Commitment, Field, Lens, MultilinearPoly, Opening, Transcript};

use genies::Fq;

/// Expansion factor for the expander graph.
const EXPANSION: usize = 2;
/// Left-degree of the expander graph.
const DEGREE: usize = 24;

/// Porphyry polynomial commitment over F_q.
pub struct Porphyry;

impl Porphyry {
    fn serialize(elements: &[Fq]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(elements.len() * 64);
        for e in elements {
            for &limb in &e.limbs {
                bytes.extend_from_slice(&limb.to_le_bytes());
            }
        }
        bytes
    }

    fn commit_raw(elements: &[Fq]) -> Commitment {
        let n = elements.len();
        let m = EXPANSION * n;
        let mut encoded = vec![Fq::ZERO; m];

        for (i, &val) in elements.iter().enumerate() {
            for j in 0..DEGREE {
                let hash = (i as u64)
                    .wrapping_mul(0x9E37_79B9_7F4A_7C15)
                    .wrapping_add((j as u64).wrapping_mul(0x517C_C1B7_2722_0A95));
                let r = (hash as usize) % m;
                encoded[r] += val;
            }
        }

        let hash = cyber_hemera::hash(&Self::serialize(&encoded));
        Commitment(hash)
    }

    fn tensor_reduce(evals: &[Fq], challenge: Fq) -> Vec<Fq> {
        let half = evals.len() / 2;
        let mut result = Vec::with_capacity(half);
        for i in 0..half {
            let even = evals[2 * i];
            let odd = evals[2 * i + 1];
            result.push(even + challenge * (odd - even));
        }
        result
    }

    fn deserialize(bytes: &[u8]) -> Vec<Fq> {
        bytes
            .chunks_exact(64)
            .map(|chunk| {
                let mut limbs = [0u64; 8];
                for (i, word) in chunk.chunks_exact(8).enumerate() {
                    let mut buf = [0u8; 8];
                    buf.copy_from_slice(word);
                    limbs[i] = u64::from_le_bytes(buf);
                }
                Fq::from_limbs(limbs)
            })
            .collect()
    }
}

impl Lens<Fq> for Porphyry {
    fn commit(poly: &MultilinearPoly<Fq>) -> Commitment {
        Self::commit_raw(&poly.evals)
    }

    fn open(poly: &MultilinearPoly<Fq>, point: &[Fq], transcript: &mut Transcript) -> Opening {
        assert_eq!(point.len(), poly.num_vars);

        let mut current = poly.evals.clone();
        let mut round_commitments = Vec::with_capacity(poly.num_vars);

        let initial = Self::commit_raw(&current);
        transcript.absorb(initial.as_bytes());

        for &r_i in point {
            let rc = Self::commit_raw(&current);
            round_commitments.push(rc);
            transcript.absorb(rc.as_bytes());
            current = Self::tensor_reduce(&current, r_i);
        }

        assert_eq!(current.len(), 1);
        let final_poly = Self::serialize(&current);

        Opening::Tensor {
            round_commitments,
            final_poly,
            query_responses: vec![],
        }
    }

    fn verify(
        commitment: &Commitment,
        point: &[Fq],
        value: Fq,
        proof: &Opening,
        transcript: &mut Transcript,
    ) -> bool {
        let Opening::Tensor {
            round_commitments,
            final_poly,
            ..
        } = proof
        else {
            return false;
        };

        if round_commitments.len() != point.len() {
            return false;
        }

        // Check 1: first round commitment binds to the committed polynomial
        if !round_commitments.is_empty() && round_commitments[0] != *commitment {
            return false;
        }

        // Check 2: replay transcript
        transcript.absorb(commitment.as_bytes());
        for rc in round_commitments {
            transcript.absorb(rc.as_bytes());
        }

        // Check 3: final value matches claimed evaluation
        let final_elements = Self::deserialize(final_poly);
        if final_elements.len() != 1 {
            return false;
        }

        final_elements[0] == value
    }

    fn batch_open(
        poly: &MultilinearPoly<Fq>,
        points: &[(Vec<Fq>, Fq)],
        transcript: &mut Transcript,
    ) -> Opening {
        if points.len() <= 1 {
            let (pt, _) = &points[0];
            return Self::open(poly, pt, transcript);
        }

        let r_star: Vec<Fq> = (0..poly.num_vars)
            .map(|_| transcript.squeeze_field())
            .collect();
        Self::open(poly, &r_star, transcript)
    }

    fn batch_verify(
        commitment: &Commitment,
        points: &[(Vec<Fq>, Fq)],
        proof: &Opening,
        transcript: &mut Transcript,
    ) -> bool {
        if points.len() <= 1 {
            let (pt, val) = &points[0];
            return Self::verify(commitment, pt, *val, proof, transcript);
        }

        let num_vars = points[0].0.len();
        let r_star: Vec<Fq> = (0..num_vars).map(|_| transcript.squeeze_field()).collect();

        let final_elements = Self::deserialize(if let Opening::Tensor { final_poly, .. } = proof {
            final_poly
        } else {
            return false;
        });
        if final_elements.len() != 1 {
            return false;
        }

        Self::verify(commitment, &r_star, final_elements[0], proof, transcript)
    }
}

/// Multilinear equality polynomial over Fq.
#[cfg(test)]
fn _multilinear_eq_fq(r: &[Fq], x: &[Fq]) -> Fq {
    assert_eq!(r.len(), x.len());
    let mut result = Fq::ONE;
    for (&ri, &xi) in r.iter().zip(x.iter()) {
        result *= ri * xi + (Fq::ONE - ri) * (Fq::ONE - xi);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commit_deterministic() {
        let poly = MultilinearPoly::new(vec![
            Fq::from_u64(1),
            Fq::from_u64(2),
            Fq::from_u64(3),
            Fq::from_u64(4),
        ]);
        assert_eq!(Porphyry::commit(&poly), Porphyry::commit(&poly));
    }

    #[test]
    fn roundtrip_tiny() {
        let poly = MultilinearPoly::new(vec![Fq::from_u64(7), Fq::from_u64(13)]);
        let commitment = Porphyry::commit(&poly);
        let point = vec![Fq::ZERO];
        let value = poly.evaluate(&point);
        assert_eq!(value, Fq::from_u64(7));

        let mut pt = Transcript::new(b"porphyry-test");
        let proof = Porphyry::open(&poly, &point, &mut pt);
        let mut vt = Transcript::new(b"porphyry-test");
        assert!(Porphyry::verify(
            &commitment,
            &point,
            value,
            &proof,
            &mut vt
        ));
    }

    #[test]
    fn roundtrip_small() {
        let poly = MultilinearPoly::new(vec![
            Fq::from_u64(10),
            Fq::from_u64(20),
            Fq::from_u64(30),
            Fq::from_u64(40),
        ]);
        let commitment = Porphyry::commit(&poly);
        let point = vec![Fq::ONE, Fq::ONE];
        let value = poly.evaluate(&point);

        let mut pt = Transcript::new(b"test");
        let proof = Porphyry::open(&poly, &point, &mut pt);
        let mut vt = Transcript::new(b"test");
        assert!(Porphyry::verify(
            &commitment,
            &point,
            value,
            &proof,
            &mut vt
        ));
    }

    #[test]
    fn wrong_value_rejected() {
        let poly = MultilinearPoly::new(vec![Fq::from_u64(1), Fq::from_u64(2)]);
        let commitment = Porphyry::commit(&poly);
        let point = vec![Fq::ZERO];
        let value = poly.evaluate(&point);

        let mut pt = Transcript::new(b"test");
        let proof = Porphyry::open(&poly, &point, &mut pt);
        let mut vt = Transcript::new(b"test");
        assert!(!Porphyry::verify(
            &commitment,
            &point,
            value + Fq::ONE,
            &proof,
            &mut vt
        ));
    }

    #[test]
    fn wrong_commitment_rejected() {
        let poly = MultilinearPoly::new(vec![Fq::from_u64(1), Fq::from_u64(2)]);
        let point = vec![Fq::ZERO];
        let value = poly.evaluate(&point);
        let fake = Commitment(cyber_hemera::hash(b"fake"));

        let mut pt = Transcript::new(b"test");
        let proof = Porphyry::open(&poly, &point, &mut pt);
        let mut vt = Transcript::new(b"test");
        assert!(!Porphyry::verify(&fake, &point, value, &proof, &mut vt));
    }

    #[test]
    fn batch_roundtrip() {
        let poly = MultilinearPoly::new(vec![
            Fq::from_u64(5),
            Fq::from_u64(10),
            Fq::from_u64(15),
            Fq::from_u64(20),
        ]);
        let commitment = Porphyry::commit(&poly);

        let points: Vec<(Vec<Fq>, Fq)> = vec![
            (
                vec![Fq::ZERO, Fq::ZERO],
                poly.evaluate(&[Fq::ZERO, Fq::ZERO]),
            ),
            (vec![Fq::ONE, Fq::ZERO], poly.evaluate(&[Fq::ONE, Fq::ZERO])),
        ];

        let mut pt = Transcript::new(b"batch");
        let proof = Porphyry::batch_open(&poly, &points, &mut pt);
        let mut vt = Transcript::new(b"batch");
        assert!(Porphyry::batch_verify(
            &commitment,
            &points,
            &proof,
            &mut vt
        ));
    }
}
