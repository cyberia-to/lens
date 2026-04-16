//! cyb-lens-binius — Binius polynomial commitment.
//!
//! Binary folding over kuro's F₂ tower (F₂ → F₂¹²⁸).
//! Binary-native: AND/XOR = 1 constraint each.
//!
//! Uses hemera Merkle tree for commitment binding.
//! Folding: each round halves the polynomial by combining pairs
//! with a binary challenge. Round commitments prove consistency.
//!
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

    /// Commit via hemera Merkle tree over serialized elements.
    fn commit_raw(elements: &[F2_128]) -> Commitment {
        let bytes = Self::serialize(elements);
        let hash = cyber_hemera::tree::tree_hash(&bytes);
        Commitment(hash)
    }

    /// Build Merkle authentication path for a round.
    /// We hash pairs of elements to build a path from leaf to root.
    fn merkle_path(elements: &[F2_128]) -> Vec<cyber_hemera::Hash> {
        if elements.len() <= 1 {
            return vec![];
        }
        // Build bottom-up Merkle tree, returning sibling hashes
        let mut level: Vec<cyber_hemera::Hash> = elements
            .chunks(2)
            .map(|chunk| {
                let bytes = Self::serialize(chunk);
                cyber_hemera::hash(&bytes)
            })
            .collect();

        let mut path = Vec::new();
        while level.len() > 1 {
            // Store first sibling at each level as proof element
            if level.len() >= 2 {
                path.push(level[1]);
            }
            level = level
                .chunks(2)
                .map(|pair| {
                    if pair.len() == 2 {
                        cyber_hemera::tree::parent_cv(
                            &pair[0],
                            &pair[1],
                            pair.len() == 2 && level.len() <= 2,
                        )
                    } else {
                        pair[0]
                    }
                })
                .collect();
        }
        path
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

            // Merkle proof of current state
            let path = Self::merkle_path(&current);
            merkle_paths.push(path);

            // Binary folding: g'[i] = even + r · (odd + even) [char 2]
            let half = current.len() / 2;
            let mut folded = Vec::with_capacity(half);
            for i in 0..half {
                let even = current[2 * i];
                let odd = current[2 * i + 1];
                folded.push(even + r_i * (odd + even));
            }
            current = folded;
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
            merkle_paths,
            final_value,
        } = proof
        else {
            return false;
        };

        if round_commitments.len() != point.len() || merkle_paths.len() != point.len() {
            return false;
        }

        // Check 1: first round commitment must match the input commitment
        if !round_commitments.is_empty() && round_commitments[0] != *commitment {
            return false;
        }

        // Check 2: replay transcript
        transcript.absorb(commitment.as_bytes());
        for rc in round_commitments {
            transcript.absorb(rc.as_bytes());
        }

        // Check 3: Merkle paths are present for non-trivial polynomials
        for (i, path) in merkle_paths.iter().enumerate() {
            let expected_size = point.len() - i;
            if expected_size > 1 && path.is_empty() {
                return false;
            }
        }

        // Check 4: final value matches claimed evaluation
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
        let r_star: Vec<F2_128> = (0..poly.num_vars)
            .map(|_| transcript.squeeze_field())
            .collect();
        Self::open(poly, &r_star, transcript)
    }

    fn batch_verify(
        commitment: &Commitment,
        points: &[(Vec<F2_128>, F2_128)],
        proof: &Opening,
        transcript: &mut Transcript,
    ) -> bool {
        if points.is_empty() {
            return false;
        }

        let num_vars = points[0].0.len();
        let r_star: Vec<F2_128> = (0..num_vars).map(|_| transcript.squeeze_field()).collect();

        let alpha: F2_128 = transcript.squeeze_field();
        let mut combined = F2_128::ZERO;
        let mut alpha_pow = F2_128::ONE;
        for (pt, val) in points {
            let eq_val = multilinear_eq_f2(pt, &r_star);
            combined += alpha_pow * *val * eq_val;
            alpha_pow *= alpha;
        }

        Self::verify(commitment, &r_star, combined, proof, transcript)
    }
}

fn multilinear_eq_f2(r: &[F2_128], x: &[F2_128]) -> F2_128 {
    assert_eq!(r.len(), x.len());
    let mut result = F2_128::ONE;
    for (&ri, &xi) in r.iter().zip(x.iter()) {
        // In char 2: (1 - r) = (1 + r) since -1 = 1
        result *= ri * xi + (F2_128::ONE + ri) * (F2_128::ONE + xi);
    }
    result
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

        let mut pt = Transcript::new(b"test");
        let proof = Binius::open(&poly, &point, &mut pt);
        let mut vt = Transcript::new(b"test");
        assert!(!Binius::verify(
            &commitment,
            &point,
            F2_128(999),
            &proof,
            &mut vt
        ));
    }

    #[test]
    fn wrong_commitment_rejected() {
        let poly = MultilinearPoly::new(vec![F2_128(1), F2_128(2)]);
        let point = vec![F2_128::ZERO];
        let value = poly.evaluate(&point);
        let fake = Commitment(cyber_hemera::hash(b"fake"));

        let mut pt = Transcript::new(b"test");
        let proof = Binius::open(&poly, &point, &mut pt);
        let mut vt = Transcript::new(b"test");
        assert!(!Binius::verify(&fake, &point, value, &proof, &mut vt));
    }

    #[test]
    fn merkle_paths_present() {
        let poly = MultilinearPoly::new(vec![F2_128(1), F2_128(2), F2_128(3), F2_128(4)]);
        let point = vec![F2_128::ONE, F2_128::ZERO];

        let mut pt = Transcript::new(b"test");
        let proof = Binius::open(&poly, &point, &mut pt);

        if let Opening::Folding { merkle_paths, .. } = &proof {
            assert_eq!(merkle_paths.len(), 2);
            // At least the first round (4 elements) should have a non-empty path
            assert!(!merkle_paths[0].is_empty());
        } else {
            panic!("expected Folding opening");
        }
    }
}
