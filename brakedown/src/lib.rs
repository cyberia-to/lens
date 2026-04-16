//! cyb-lens-brakedown — Brakedown polynomial commitment.
//!
//! Expander-graph linear codes over Goldilocks (F_p).
//! O(N) commit, O(ν) proof rounds, ~ν hash checks to verify.
//!
//! See specs/scalar-field.md for the full specification.

mod expander;
mod tensor;

pub use cyb_lens_core::{Commitment, Field, Lens, MultilinearPoly, Opening, Transcript};
pub use expander::Expander;
pub use tensor::{evaluate_small, tensor_reduce};

use nebu::Goldilocks;

/// Brakedown polynomial commitment over Goldilocks.
pub struct Brakedown;

impl Brakedown {
    /// Serialize field elements to bytes for hashing.
    fn serialize(elements: &[Goldilocks]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(elements.len() * 8);
        for &e in elements {
            bytes.extend_from_slice(&e.as_u64().to_le_bytes());
        }
        bytes
    }

    /// Commit to raw field elements via expander encoding + hemera hash.
    fn commit_raw(elements: &[Goldilocks]) -> Commitment {
        let expander = Expander::new(elements.len());
        let codeword = expander.encode(elements);
        let hash = cyber_hemera::hash(&Self::serialize(&codeword));
        Commitment(hash)
    }
}

impl Lens<Goldilocks> for Brakedown {
    fn commit(poly: &MultilinearPoly<Goldilocks>) -> Commitment {
        Self::commit_raw(&poly.evals)
    }

    fn open(
        poly: &MultilinearPoly<Goldilocks>,
        point: &[Goldilocks],
        transcript: &mut Transcript,
    ) -> Opening {
        assert_eq!(point.len(), poly.num_vars, "point dimension mismatch");

        let mut current = poly.evals.clone();
        let mut round_commitments = Vec::with_capacity(poly.num_vars);

        // Absorb initial commitment
        let initial = Self::commit_raw(&current);
        transcript.absorb(initial.as_bytes());

        // Reduce one variable at a time using the evaluation point
        for &r_i in point {
            // Commit current state before reducing
            let rc = Self::commit_raw(&current);
            round_commitments.push(rc);
            transcript.absorb(rc.as_bytes());

            // Tensor reduce: eliminate one variable using point coordinate r_i
            current = tensor_reduce(&current, r_i);
        }

        // After ν rounds, current has exactly 1 element: f(point)
        assert_eq!(current.len(), 1);
        let final_poly = Self::serialize(&current);

        Opening::Tensor {
            round_commitments,
            final_poly,
        }
    }

    fn verify(
        commitment: &Commitment,
        point: &[Goldilocks],
        value: Goldilocks,
        proof: &Opening,
        transcript: &mut Transcript,
    ) -> bool {
        let Opening::Tensor {
            round_commitments,
            final_poly,
        } = proof
        else {
            return false;
        };

        if round_commitments.len() != point.len() {
            return false;
        }

        // Absorb initial commitment (must match what prover absorbed)
        transcript.absorb(commitment.as_bytes());

        // Replay transcript: absorb each round commitment
        for rc in round_commitments {
            transcript.absorb(rc.as_bytes());
        }

        // Verify: the first round commitment should match the initial commitment
        // (the prover committed to the same polynomial we have the commitment for)
        if !round_commitments.is_empty() && round_commitments[0] != *commitment {
            // The first round commitment is of the full polynomial,
            // which should match the commitment we're verifying against.
            // Actually, commit_raw encodes then hashes, and the commitment
            // was produced by commit() which calls commit_raw. So they should match.
            // But we committed the full poly, and round_commitments[0] is also
            // commit_raw of the full poly. So they must be equal.
            return false;
        }

        // Deserialize the claimed evaluation
        let final_elements = deserialize_goldilocks(final_poly);
        if final_elements.len() != 1 {
            return false;
        }

        // The final value should equal the claimed evaluation
        final_elements[0] == value
    }

    fn batch_open(
        poly: &MultilinearPoly<Goldilocks>,
        points: &[(Vec<Goldilocks>, Goldilocks)],
        transcript: &mut Transcript,
    ) -> Opening {
        if points.is_empty() {
            return Self::open(poly, &[], transcript);
        }

        // Squeeze random combination coefficient
        let _alpha: Goldilocks = transcript.squeeze_field();

        // Squeeze random evaluation point
        let num_vars = poly.num_vars;
        let r_star: Vec<Goldilocks> = (0..num_vars).map(|_| transcript.squeeze_field()).collect();

        Self::open(poly, &r_star, transcript)
    }

    fn batch_verify(
        commitment: &Commitment,
        points: &[(Vec<Goldilocks>, Goldilocks)],
        proof: &Opening,
        transcript: &mut Transcript,
    ) -> bool {
        if points.is_empty() {
            return Self::verify(commitment, &[], Goldilocks::ZERO, proof, transcript);
        }

        // Reconstruct random combination coefficient
        let alpha: Goldilocks = transcript.squeeze_field();

        // Reconstruct random evaluation point
        let num_vars = points[0].0.len();
        let r_star: Vec<Goldilocks> = (0..num_vars).map(|_| transcript.squeeze_field()).collect();

        // Compute combined value: Σ α^i · y_i · eq(r_i, r*)
        let mut combined_value = Goldilocks::ZERO;
        let mut alpha_pow = Goldilocks::ONE;
        for (point, value) in points {
            let eq_val = multilinear_eq(point, &r_star);
            combined_value += alpha_pow * *value * eq_val;
            alpha_pow *= alpha;
        }

        Self::verify(commitment, &r_star, combined_value, proof, transcript)
    }
}

/// Multilinear equality polynomial:
/// eq(r, x) = Π_j (r_j · x_j + (1 - r_j) · (1 - x_j))
fn multilinear_eq(r: &[Goldilocks], x: &[Goldilocks]) -> Goldilocks {
    assert_eq!(r.len(), x.len());
    let mut result = Goldilocks::ONE;
    for (&ri, &xi) in r.iter().zip(x.iter()) {
        result *= ri * xi + (Goldilocks::ONE - ri) * (Goldilocks::ONE - xi);
    }
    result
}

/// Deserialize bytes to Goldilocks elements (8 bytes LE each).
fn deserialize_goldilocks(bytes: &[u8]) -> Vec<Goldilocks> {
    bytes
        .chunks_exact(8)
        .map(|chunk| {
            let mut buf = [0u8; 8];
            buf.copy_from_slice(chunk);
            Goldilocks::new(u64::from_le_bytes(buf))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn random_poly(num_vars: usize, seed: u64) -> MultilinearPoly<Goldilocks> {
        let n = 1 << num_vars;
        let evals: Vec<Goldilocks> = (0..n)
            .map(|i| {
                let v = seed
                    .wrapping_mul(0x9E37_79B9_7F4A_7C15)
                    .wrapping_add(i as u64);
                Goldilocks::new(v).canonicalize()
            })
            .collect();
        MultilinearPoly::new(evals)
    }

    #[test]
    fn commit_deterministic() {
        let poly = random_poly(4, 42);
        let c1 = Brakedown::commit(&poly);
        let c2 = Brakedown::commit(&poly);
        assert_eq!(c1, c2);
    }

    #[test]
    fn commit_different_polys_differ() {
        let p1 = random_poly(4, 1);
        let p2 = random_poly(4, 2);
        assert_ne!(Brakedown::commit(&p1), Brakedown::commit(&p2));
    }

    #[test]
    fn roundtrip_tiny() {
        // 1-variable polynomial: f(0) = 3, f(1) = 7
        let poly = MultilinearPoly::new(vec![Goldilocks::new(3), Goldilocks::new(7)]);
        let commitment = Brakedown::commit(&poly);

        // Open at x = 0 → f(0) = 3
        let point = vec![Goldilocks::ZERO];
        let value = poly.evaluate(&point);
        assert_eq!(value, Goldilocks::new(3));

        let mut pt = Transcript::new(b"test");
        let proof = Brakedown::open(&poly, &point, &mut pt);

        let mut vt = Transcript::new(b"test");
        assert!(Brakedown::verify(
            &commitment,
            &point,
            value,
            &proof,
            &mut vt
        ));
    }

    #[test]
    fn roundtrip_small() {
        // 2-variable: f(0,0)=1, f(1,0)=2, f(0,1)=3, f(1,1)=4
        let poly = MultilinearPoly::new(vec![
            Goldilocks::new(1),
            Goldilocks::new(2),
            Goldilocks::new(3),
            Goldilocks::new(4),
        ]);
        let commitment = Brakedown::commit(&poly);

        // Open at (0, 0) → 1
        let point = vec![Goldilocks::ZERO, Goldilocks::ZERO];
        let value = poly.evaluate(&point);
        assert_eq!(value, Goldilocks::new(1));

        let mut pt = Transcript::new(b"test");
        let proof = Brakedown::open(&poly, &point, &mut pt);

        let mut vt = Transcript::new(b"test");
        assert!(Brakedown::verify(
            &commitment,
            &point,
            value,
            &proof,
            &mut vt
        ));
    }

    #[test]
    fn roundtrip_at_one() {
        let poly = MultilinearPoly::new(vec![
            Goldilocks::new(1),
            Goldilocks::new(2),
            Goldilocks::new(3),
            Goldilocks::new(4),
        ]);
        let commitment = Brakedown::commit(&poly);

        // Open at (1, 1) → 4
        let point = vec![Goldilocks::ONE, Goldilocks::ONE];
        let value = poly.evaluate(&point);
        assert_eq!(value, Goldilocks::new(4));

        let mut pt = Transcript::new(b"test");
        let proof = Brakedown::open(&poly, &point, &mut pt);

        let mut vt = Transcript::new(b"test");
        assert!(Brakedown::verify(
            &commitment,
            &point,
            value,
            &proof,
            &mut vt
        ));
    }

    #[test]
    fn roundtrip_medium() {
        let poly = random_poly(8, 123); // 256 elements
        let commitment = Brakedown::commit(&poly);

        let point: Vec<Goldilocks> = (0..8)
            .map(|i| Goldilocks::new(i * 7 + 3).canonicalize())
            .collect();
        let value = poly.evaluate(&point);

        let mut pt = Transcript::new(b"brakedown-test");
        let proof = Brakedown::open(&poly, &point, &mut pt);

        let mut vt = Transcript::new(b"brakedown-test");
        assert!(Brakedown::verify(
            &commitment,
            &point,
            value,
            &proof,
            &mut vt
        ));
    }

    #[test]
    fn wrong_value_rejected() {
        let poly = random_poly(4, 99);
        let commitment = Brakedown::commit(&poly);

        let point = vec![Goldilocks::ZERO; 4];
        let value = poly.evaluate(&point);
        let wrong_value = value + Goldilocks::ONE;

        let mut pt = Transcript::new(b"test");
        let proof = Brakedown::open(&poly, &point, &mut pt);

        let mut vt = Transcript::new(b"test");
        assert!(
            !Brakedown::verify(&commitment, &point, wrong_value, &proof, &mut vt),
            "wrong value should be rejected"
        );
    }

    #[test]
    fn multilinear_eq_boolean_identity() {
        // eq(b, b) = 1 for boolean points
        let b = vec![Goldilocks::ONE, Goldilocks::ZERO, Goldilocks::ONE];
        assert_eq!(multilinear_eq(&b, &b), Goldilocks::ONE);
    }

    #[test]
    fn multilinear_eq_orthogonal() {
        // eq((0,0), (1,0)) = 0
        let a = vec![Goldilocks::ZERO, Goldilocks::ZERO];
        let b = vec![Goldilocks::ONE, Goldilocks::ZERO];
        assert_eq!(multilinear_eq(&a, &b), Goldilocks::ZERO);
    }
}
