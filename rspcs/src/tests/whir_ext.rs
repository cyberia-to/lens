//! WHIR over an Fp3 message (`Whir::commit_ext`) and the leaf-access API:
//! completeness, rejection, the base path unchanged, leaf authentication.

use cyber_lens_core::{MultilinearPcs, Transcript};
use nebu::Fp3;

use super::common::*;
use crate::field::{ml_eval_ext, pow_point};
use crate::whir::{LeafLayout, verify_leaves};
use crate::{Whir, WhirParams, WhirProof};

fn ext_table(rng: &mut Rng, m: usize) -> Vec<Fp3> {
    (0..1usize << m).map(|_| rng.e()).collect()
}

fn roundtrip(params: &WhirParams, m: usize, seed: u64) {
    let mut rng = Rng::new(seed);
    let evals = ext_table(&mut rng, m);
    let point = rng.point(m);
    let (root, data) = Whir::commit_ext(params, &evals);
    assert!(data.is_ext());
    let mut tp = Transcript::new(DOMAIN);
    let (value, proof) = Whir::open(params, &data, &point, &mut tp);
    assert_eq!(value, ml_eval_ext(&evals, &point));
    let bytes = proof.to_bytes();
    let parsed = WhirProof::from_bytes_with(&bytes, true).expect("ext proof parses");
    assert_eq!(parsed, proof);
    let mut tv = Transcript::new(DOMAIN);
    Whir::verify_ext(params, &root, m, &point, value, &parsed, &mut tv).expect("verifies");
    // the same proof does not verify as a base commitment, nor a wrong value
    let mut tb = Transcript::new(DOMAIN);
    assert!(Whir::verify(params, &root, m, &point, value, &parsed, &mut tb).is_err());
    let mut tw = Transcript::new(DOMAIN);
    let bad = value + Fp3::ONE;
    assert!(Whir::verify_ext(params, &root, m, &point, bad, &parsed, &mut tw).is_err());
}

#[test]
fn ext_commitments_open_and_verify() {
    let small = WhirParams {
        log_inv_rate: 2,
        ..WhirParams::default()
    };
    for m in [1, 4, 9, 12] {
        roundtrip(&small, m, 900 + m as u64);
    }
    roundtrip(&WhirParams::default(), 14, 914);
}

#[test]
fn ext_proof_bit_flips_are_rejected() {
    let params = WhirParams {
        log_inv_rate: 2,
        ..WhirParams::default()
    };
    let mut rng = Rng::new(77);
    let m = 6;
    let evals = ext_table(&mut rng, m);
    let point = rng.point(m);
    let (root, data) = Whir::commit_ext(&params, &evals);
    let (value, proof) = Whir::open(&params, &data, &point, &mut Transcript::new(DOMAIN));
    let bytes = proof.to_bytes();
    let mut accepted = 0;
    for i in 0..bytes.len() {
        for bit in [0, 3, 7] {
            let mut b = bytes.clone();
            b[i] ^= 1 << bit;
            if let Ok(p) = WhirProof::from_bytes_with(&b, true) {
                let mut t = Transcript::new(DOMAIN);
                if Whir::verify_ext(&params, &root, m, &point, value, &p, &mut t).is_ok() {
                    accepted += 1;
                }
            }
        }
    }
    assert_eq!(accepted, 0);
}

#[test]
fn leaves_authenticate_symbols_and_univariate_matches_codeword() {
    let params = WhirParams::default();
    let mut rng = Rng::new(5);
    for (m, ext) in [(8, false), (8, true), (11, true)] {
        let (root, data) = if ext {
            Whir::commit_ext(&params, &ext_table(&mut rng, m))
        } else {
            Whir::commit(&params, &rng.evals(m))
        };
        let layout = LeafLayout::derive(&params, m).unwrap();
        let n = 1usize << layout.log_domain;
        // symbol s is f̂(ω^s), and f̂(z) = f(pow(z)) for the table's multilinear
        for s in [0, 1, 7, n / 2 + 3, n - 1] {
            let x = Fp3::from_base(layout.point(s));
            assert_eq!(data.symbol(s), data.univariate(x));
            let table = data.table();
            assert_eq!(ml_eval_ext(&table, &pow_point(x, m)), data.symbol(s));
            let (leaf, pos) = layout.locate(s);
            assert_eq!(data.leaf(leaf)[pos], data.symbol(s));
        }
        let leaves = vec![0, 3, 5, (1 << layout.log_leaves()) - 1];
        let opening = data.open_leaves(&leaves);
        let got = verify_leaves(&root.0, &layout, ext, &leaves, &opening).unwrap();
        for (q, &j) in leaves.iter().enumerate() {
            assert_eq!(got[q], data.leaf(j));
        }
        let mut forged = opening.clone();
        let mut b = *forged.siblings[0].as_bytes();
        b[0] ^= 1;
        forged.siblings[0] = cyber_hemera::Hash::from_bytes(b);
        assert!(verify_leaves(&root.0, &layout, ext, &leaves, &forged).is_err());
        assert!(verify_leaves(&root.0, &layout, !ext, &leaves, &opening).is_err());
        let wire = opening.to_bytes();
        assert_eq!(crate::whir::Opening::from_bytes(&wire, ext).unwrap(), opening);
    }
}

#[cfg(feature = "serde")]
#[test]
fn serde_round_trips_fp3_and_base_proofs() {
    let params = WhirParams {
        log_inv_rate: 2,
        ..WhirParams::default()
    };
    let mut rng = Rng::new(31);
    for (m, ext) in [(5, true), (5, false), (11, true)] {
        let point = rng.point(m);
        let data = if ext {
            Whir::commit_ext(&params, &ext_table(&mut rng, m)).1
        } else {
            Whir::commit(&params, &rng.evals(m)).1
        };
        let (_, proof) = Whir::open(&params, &data, &point, &mut Transcript::new(DOMAIN));
        assert_eq!(proof.opens_ext0(), ext);
        let bytes = postcard::to_allocvec(&proof).unwrap();
        let back: WhirProof = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(back, proof, "m={m} ext={ext}");
    }
}
