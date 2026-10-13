//! WHIR: completeness, rejection, bit-flip scan, cheating prover,
//! determinism, security selection.

use cyber_lens_core::{MultilinearPcs, Transcript};

use super::common::*;
use crate::field::mobius_base;
use crate::whir::{Decoding, WhirData};
use crate::{Whir, WhirConfig, WhirParams};

fn params(r: u8, k: u8) -> WhirParams {
    WhirParams {
        log_inv_rate: r,
        folding_factor: k,
        ..WhirParams::default()
    }
}

#[test]
fn completeness_1_to_14_rates_and_folding() {
    for r in [1, 2, 3, 4] {
        for m in 1..=14 {
            completeness::<Whir>(&params(r, 4), m, 200 + m as u64);
        }
    }
    for k in [1, 2, 3] {
        for m in [1, 3, 7, 12] {
            completeness::<Whir>(&params(2, k), m, 300 + m as u64);
        }
    }
}

#[test]
fn completeness_many_rounds_small_final() {
    for m in [5, 9, 13] {
        let p = WhirParams {
            max_final_vars: 0,
            ..params(2, 2)
        };
        completeness::<Whir>(&p, m, 400 + m as u64);
    }
}

#[test]
fn completeness_unique_decoding() {
    for m in [1, 6, 11, 16] {
        let p = WhirParams {
            decoding: Decoding::Unique,
            ..params(2, 4)
        };
        completeness::<Whir>(&p, m, 500 + m as u64);
    }
}

#[test]
fn completeness_16_18_20() {
    for m in [16, 18, 20] {
        completeness::<Whir>(&params(4, 4), m, m as u64);
    }
    completeness::<Whir>(&params(1, 4), 20, 21);
    completeness::<Whir>(&params(2, 4), 16, 22);
}

#[test]
fn rejects_wrong_value_point_commitment_size() {
    for m in [1, 5, 10] {
        rejects_wrong_claims::<Whir>(&params(2, 4), m, 3);
    }
    let p = WhirParams {
        max_final_vars: 1,
        ..params(2, 2)
    };
    rejects_wrong_claims::<Whir>(&p, 7, 4);
}

#[test]
fn every_bit_flip_rejected() {
    // three committed functions, extension-field openings, nonces present
    let p = WhirParams {
        log_inv_rate: 2,
        folding_factor: 2,
        pow_bits: 4,
        max_final_vars: 1,
        ..WhirParams::default()
    };
    assert_eq!(WhirConfig::derive(&p, 6).unwrap().rounds.len(), 3);
    let flips = bitflip_scan::<Whir>(&p, 6, 12);
    assert!(flips > 0);
}

/// Four committed functions, every query round sampled with a nonce.
#[test]
fn every_bit_flip_rejected_sampled_queries() {
    let p = WhirParams {
        log_inv_rate: 3,
        folding_factor: 2,
        pow_bits: 4,
        max_final_vars: 1,
        ..WhirParams::default()
    };
    let c = WhirConfig::derive(&p, 8).unwrap();
    assert_eq!(c.rounds.len(), 4);
    assert!(c.rounds.iter().all(|r| !r.opens_all() && r.query_pow > 0));
    bitflip_scan::<Whir>(&p, 8, 15);
}

#[test]
fn every_bit_flip_rejected_default_params() {
    bitflip_scan::<Whir>(&WhirParams::default(), 5, 13);
}

#[test]
fn deterministic_proof_bytes() {
    deterministic::<Whir>(&params(4, 4), 12, 5);
}

#[test]
fn security_meets_target_at_every_size() {
    for r in [1u8, 2, 3, 4] {
        for m in 1..=24 {
            if m + r as usize > 32 {
                continue;
            }
            let p = params(r, 4);
            let bits = Whir::security_bits(&p, m);
            assert!(bits >= 128.0, "r={r} m={m}: {bits}");
        }
    }
}

/// Commit to f, run the prover for g on f's codeword and tree.
#[test]
fn cheating_prover_commit_f_open_g_rejected() {
    for (m, p) in [
        (3, params(2, 4)),
        (9, params(4, 4)),
        (
            8,
            WhirParams {
                max_final_vars: 1,
                ..params(2, 2)
            },
        ),
    ] {
        let mut rng = Rng::new(77 + m as u64);
        let f = rng.evals(m);
        let g = rng.evals(m);
        let z = rng.point(m);
        let (cf, df) = Whir::commit(&p, &f);
        let mut gc = g.clone();
        mobius_base(&mut gc);
        let forged = WhirData {
            config: WhirConfig::derive(&p, m).unwrap(),
            message: crate::whir::prover_message_base(g.clone(), gc),
            codeword: df.codeword,
            tree: df.tree,
        };
        let (v, proof) = Whir::open(&p, &forged, &z, &mut Transcript::new(DOMAIN));
        assert_ne!(v, crate::field::ml_eval_base(&f, &z));
        assert!(check::<Whir>(&p, &cf, m, &z, v, &proof).is_err(), "m={m}");
    }
}

#[cfg(feature = "serde")]
#[test]
fn serde_roundtrip_is_wire_bytes() {
    let r = run::<Whir>(&params(2, 4), 8, 9);
    let enc = postcard::to_allocvec(&r.proof).unwrap();
    let dec: crate::WhirProof = postcard::from_bytes(&enc).unwrap();
    assert_eq!(dec, r.proof);
}
