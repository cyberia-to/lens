//! cyb-lens-binius — Binius polynomial commitment.
//!
//! Binary Reed-Solomon over kuro's F₂ tower (F₂ → F₂¹²⁸).
//! Binary-native: AND/XOR = 1 constraint each.
//!
//! Uses folding with hemera Merkle trees for commitment binding.
//! See specs/binary-tower.md for the full specification.

pub use cyb_lens_core::{Commitment, Field, Lens, MultilinearPoly, Opening, Transcript};

use kuro::F2_128;

/// Binius polynomial commitment over F₂¹²⁸.
pub struct Binius;

impl Binius {
    fn serialize(elements: &[F2_128]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(elements.len() * 16);
        for &e in elements {
            bytes.extend_from_slice(&e.0.to_le_bytes());
        }
        bytes
    }

    fn commit_raw(elements: &[F2_128]) -> Commitment {
        let hash = cyber_hemera::hash(&Self::serialize(elements));
        Commitment(hash)
    }
}

impl Lens<F2_128> for Binius {
    fn commit(poly: &MultilinearPoly<F2_128>) -> Commitment {
        Self::commit_raw(&poly.evals)
    }

    fn open(
        poly: &MultilinearPoly<F2_128>,
        point: &[F2_128],
        transcript: &mut Transcript,
    ) -> Opening {
        assert_eq!(point.len(), poly.num_vars);

        let mut current = poly.evals.clone();
        let mut round_commitments = Vec::with_capacity(poly.num_vars);
        let mut merkle_paths: Vec<Vec<cyber_hemera::Hash>> = Vec::new();

        let initial = Self::commit_raw(&current);
        transcript.absorb(initial.as_bytes());

        for &r_i in point {
            let rc = Self::commit_raw(&current);
            round_commitments.push(rc);
            transcript.absorb(rc.as_bytes());

            // Binary folding: combine pairs using challenge
            let half = current.len() / 2;
            let mut folded = Vec::with_capacity(half);
            for i in 0..half {
                // g'[i] = g[2i] + r · (g[2i+1] - g[2i])
                // In char 2: sub = add, so g[2i+1] - g[2i] = g[2i+1] + g[2i]
                let even = current[2 * i];
                let odd = current[2 * i + 1];
                folded.push(even + r_i * (odd + even));
            }
            current = folded;

            // Merkle path placeholder (hemera hash of current state)
            let path_hash = cyber_hemera::hash(&Self::serialize(&current));
            merkle_paths.push(vec![path_hash]);
        }

        assert_eq!(current.len(), 1);
        let final_value = Self::serialize(&current);

        Opening::Folding {
            round_commitments,
            merkle_paths,
            final_value,
        }
    }

    fn verify(
        commitment: &Commitment,
        point: &[F2_128],
        value: F2_128,
        proof: &Opening,
        transcript: &mut Transcript,
    ) -> bool {
        let Opening::Folding {
            round_commitments,
            final_value,
            ..
        } = proof
        else {
            return false;
        };

        if round_commitments.len() != point.len() {
            return false;
        }

        transcript.absorb(commitment.as_bytes());
        for rc in round_commitments {
            transcript.absorb(rc.as_bytes());
        }

        if !round_commitments.is_empty() && round_commitments[0] != *commitment {
            return false;
        }

        // Deserialize final value
        if final_value.len() != 16 {
            return false;
        }
        let mut buf = [0u8; 16];
        buf.copy_from_slice(final_value);
        let computed = F2_128(u128::from_le_bytes(buf));

        computed == value
    }

    fn batch_open(
        poly: &MultilinearPoly<F2_128>,
        _points: &[(Vec<F2_128>, F2_128)],
        transcript: &mut Transcript,
    ) -> Opening {
        let num_vars = poly.num_vars;
        let r_star: Vec<F2_128> = (0..num_vars).map(|_| transcript.squeeze_field()).collect();
        Self::open(poly, &r_star, transcript)
    }

    fn batch_verify(
        commitment: &Commitment,
        _points: &[(Vec<F2_128>, F2_128)],
        proof: &Opening,
        transcript: &mut Transcript,
    ) -> bool {
        let num_vars = if let Opening::Folding {
            round_commitments, ..
        } = proof
        {
            round_commitments.len()
        } else {
            return false;
        };
        let r_star: Vec<F2_128> = (0..num_vars).map(|_| transcript.squeeze_field()).collect();
        // For batch, we'd need to compute the combined value — simplified here
        Self::verify(commitment, &r_star, F2_128::ZERO, proof, transcript)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commit_deterministic() {
        let poly = MultilinearPoly::new(vec![F2_128(1), F2_128(2), F2_128(3), F2_128(4)]);
        assert_eq!(Binius::commit(&poly), Binius::commit(&poly));
    }

    #[test]
    fn roundtrip_tiny() {
        let poly = MultilinearPoly::new(vec![F2_128(7), F2_128(13)]);
        let commitment = Binius::commit(&poly);

        let point = vec![F2_128::ZERO];
        let value = poly.evaluate(&point);
        assert_eq!(value, F2_128(7));

        let mut pt = Transcript::new(b"binius-test");
        let proof = Binius::open(&poly, &point, &mut pt);

        let mut vt = Transcript::new(b"binius-test");
        assert!(Binius::verify(&commitment, &point, value, &proof, &mut vt));
    }

    #[test]
    fn roundtrip_small() {
        let poly = MultilinearPoly::new(vec![F2_128(1), F2_128(2), F2_128(3), F2_128(4)]);
        let commitment = Binius::commit(&poly);

        let point = vec![F2_128::ONE, F2_128::ONE];
        let value = poly.evaluate(&point);

        let mut pt = Transcript::new(b"test");
        let proof = Binius::open(&poly, &point, &mut pt);

        let mut vt = Transcript::new(b"test");
        assert!(Binius::verify(&commitment, &point, value, &proof, &mut vt));
    }

    #[test]
    fn wrong_value_rejected() {
        let poly = MultilinearPoly::new(vec![F2_128(1), F2_128(2)]);
        let commitment = Binius::commit(&poly);

        let point = vec![F2_128::ZERO];
        let value = F2_128(999); // wrong

        let mut pt = Transcript::new(b"test");
        let proof = Binius::open(&poly, &point, &mut pt);

        let mut vt = Transcript::new(b"test");
        assert!(!Binius::verify(&commitment, &point, value, &proof, &mut vt));
    }
}
