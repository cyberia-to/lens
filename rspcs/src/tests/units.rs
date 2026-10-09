//! Unit tests: field helpers, RS encoding, Merkle multi-openings,
//! folding, transcript challenges, wire canonicity.

use cyber_lens_core::Transcript;
use nebu::field::P;
use nebu::{Fp3, Goldilocks};

use super::common::Rng;
use crate::field::*;
use crate::merkle::{self, Tree, leaf_digest};
use crate::rs::{encode_base, encode_ext};
use crate::whir::fold_leaf_for_tests as fold_leaf;
use crate::wire::Reader;

#[test]
fn eq_table_matches_eq_eval() {
    let mut rng = Rng::new(1);
    let z = rng.point(5);
    let t = eq_table(&z);
    for (b, &v) in t.iter().enumerate() {
        let bits: Vec<Fp3> = (0..5)
            .map(|k| {
                if (b >> k) & 1 == 1 {
                    Fp3::ONE
                } else {
                    Fp3::ZERO
                }
            })
            .collect();
        assert_eq!(v, eq_eval(&z, &bits));
    }
}

#[test]
fn mobius_coefficients_evaluate_like_the_table() {
    let mut rng = Rng::new(2);
    for m in 0..8 {
        let evals = rng.evals(m);
        let z = rng.point(m);
        let mut c = evals.clone();
        mobius_base(&mut c);
        let ce: Vec<Fp3> = c.iter().map(|&x| Fp3::from_base(x)).collect();
        assert_eq!(coeff_ml_eval(&ce, &z), ml_eval_base(&evals, &z));
        // univariate view: f̂(x) = f(x, x², x⁴, …)
        let x = rng.e();
        assert_eq!(univariate_base(&c, x), coeff_ml_eval(&ce, &pow_point(x, m)));
    }
}

#[test]
fn rs_encoding_is_evaluation_on_the_subgroup() {
    let mut rng = Rng::new(3);
    for (logk, logn) in [(0, 0), (0, 3), (2, 2), (3, 5), (10, 12)] {
        let c: Vec<Goldilocks> = (0..1usize << logk).map(|_| rng.g()).collect();
        let cw = encode_base(&c, logn);
        let w = root_of_unity(logn);
        for j in [0usize, 1, (1 << logn) - 1, (1 << logn) / 3]
            .into_iter()
            .filter(|&j| j < 1 << logn)
        {
            let x = w.exp(j as u64);
            let direct = c.iter().rev().fold(Goldilocks::ZERO, |a, &ci| a * x + ci);
            assert_eq!(cw[j], direct, "logk={logk} logn={logn} j={j}");
        }
        let ce: Vec<Fp3> = (0..1usize << logk).map(|_| rng.e()).collect();
        let cwe = encode_ext(&ce, logn);
        assert_eq!(
            cwe[1 % cwe.len()],
            univariate_ext_at_base(&ce, w.exp(1 % cwe.len() as u64))
        );
    }
}

#[test]
fn merkle_multi_openings_verify_and_tamper_fails() {
    let mut rng = Rng::new(4);
    for depth in 0..7 {
        let n = 1usize << depth;
        let leaves: Vec<Vec<u8>> = (0..n).map(|i| vec![i as u8; 1 + i % 5]).collect();
        let tree = Tree::build(n, |i| leaves[i].clone());
        for _ in 0..8 {
            let mut idx: Vec<usize> = (0..1 + rng.u64() as usize % n.max(1))
                .map(|_| rng.u64() as usize % n)
                .collect();
            idx.sort_unstable();
            idx.dedup();
            let sib = tree.open(&idx);
            let digests = |l: &[Vec<u8>]| idx.iter().map(|&i| leaf_digest(&l[i], i, n)).collect();
            assert!(merkle::verify(&tree.root(), depth, &idx, digests(&leaves), &sib).is_ok());
            let mut bad = leaves.clone();
            bad[idx[0]].push(0);
            assert!(merkle::verify(&tree.root(), depth, &idx, digests(&bad), &sib).is_err());
            if !sib.is_empty() {
                assert!(
                    merkle::verify(&tree.root(), depth, &idx, digests(&leaves), &sib[1..]).is_err()
                );
                let mut extra = sib.clone();
                extra.push(sib[0]);
                assert!(
                    merkle::verify(&tree.root(), depth, &idx, digests(&leaves), &extra).is_err()
                );
            }
        }
    }
}

#[test]
fn fold_of_a_coset_is_the_folded_polynomial() {
    let mut rng = Rng::new(5);
    for (m, k, logn) in [(4usize, 1usize, 6u32), (6, 3, 8), (5, 5, 9), (8, 4, 12)] {
        let c: Vec<Fp3> = (0..1usize << m).map(|_| rng.e()).collect();
        let cw = encode_ext(&c, logn);
        let alphas = rng.point(k);
        let mut folded = c.clone();
        for &a in &alphas {
            fold_coeffs_ext(&mut folded, a);
        }
        let width = 1usize << k;
        let leaves = (1usize << logn) / width;
        for j in [0, 1, leaves - 1] {
            let vals: Vec<Fp3> = (0..width).map(|t| cw[j + t * leaves]).collect();
            let r = root_of_unity(logn - k as u32).exp(j as u64);
            assert_eq!(
                fold_leaf(&vals, &alphas, logn, j),
                univariate_ext_at_base(&folded, r)
            );
        }
    }
}

#[test]
fn transcript_challenges_are_canonical_and_deterministic() {
    let mut a = Transcript::new(b"x");
    let mut b = Transcript::new(b"x");
    for _ in 0..50 {
        let (x, y) = (a.squeeze_fp3(), b.squeeze_fp3());
        assert_eq!(x, y);
        for l in [x.c0, x.c1, x.c2] {
            assert!(l.as_u64() < P);
        }
    }
    let idx = a.squeeze_indices(1000, 7);
    assert!(idx.iter().all(|&i| i < 128));
    assert_eq!(idx, b.squeeze_indices(1000, 7));
    assert!(idx.iter().any(|&i| i >= 64) && idx.iter().any(|&i| i < 64));
}

#[test]
fn wire_rejects_non_canonical_field_and_digest_limbs() {
    let bad = P.to_le_bytes();
    assert!(Reader::new(&bad).base().is_err());
    let mut d = [0u8; 32];
    d[8..16].copy_from_slice(&bad);
    assert!(Reader::new(&d).digest().is_err());
    let ok = (P - 1).to_le_bytes();
    assert!(Reader::new(&ok).base().is_ok());
}

#[test]
fn quadratic_interpolation() {
    let mut rng = Rng::new(6);
    let (a, b, c, x) = (rng.e(), rng.e(), rng.e(), rng.e());
    let h = |t: Fp3| a + b * t + c * t * t;
    let two = Fp3::from_base(Goldilocks::new(2));
    assert_eq!(quadratic_at(h(Fp3::ZERO), h(Fp3::ONE), h(two), x), h(x));
}
