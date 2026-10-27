//! TensorRs: completeness, rejection, bit-flip scan, cheating prover,
//! determinism.

use cyber_lens_core::{MultilinearPcs, Transcript};

use super::common::*;
use crate::tensor::TensorRsData;
use crate::{TensorConfig, TensorRs, TensorRsParams};

fn params(r: u8) -> TensorRsParams {
    TensorRsParams {
        log_inv_rate: r,
        ..TensorRsParams::default()
    }
}

#[test]
fn completeness_1_to_14_rates_half_quarter_sixteenth() {
    for r in [1, 2, 4] {
        for m in 1..=14 {
            completeness::<TensorRs>(&params(r), m, 100 + m as u64);
        }
    }
}

#[test]
fn completeness_fixed_shapes() {
    for m in [1, 5, 9] {
        for rv in 0..=m as u8 {
            let p = TensorRsParams {
                row_vars: Some(rv),
                ..params(2)
            };
            completeness::<TensorRs>(&p, m, 7 + u64::from(rv));
        }
    }
}

#[test]
#[ignore = "heavy: completeness at 2^16..2^20; run by CI job `heavy` (cargo test --release -- --ignored)"]
fn completeness_16_18_20() {
    for m in [16, 18, 20] {
        completeness::<TensorRs>(&params(2), m, m as u64);
    }
    completeness::<TensorRs>(&params(4), 20, 20);
    completeness::<TensorRs>(&params(1), 16, 16);
}

#[test]
fn rejects_wrong_value_point_commitment_size() {
    for m in [1, 4, 9] {
        rejects_wrong_claims::<TensorRs>(&params(2), m, 3);
    }
}

#[test]
fn every_bit_flip_rejected() {
    let p = TensorRsParams {
        pow_bits: 6,
        ..params(2)
    };
    let flips = bitflip_scan::<TensorRs>(&p, 6, 11);
    assert!(flips > 0);
}

/// Sampled columns with a grinding nonce (not every column opened).
#[test]
fn every_bit_flip_rejected_sampled_columns() {
    let p = TensorRsParams {
        pow_bits: 4,
        row_vars: Some(1),
        ..params(2)
    };
    let c = TensorConfig::derive(&p, 8).unwrap();
    assert!(!c.opens_all() && c.pow_bits > 0);
    bitflip_scan::<TensorRs>(&p, 8, 14);
}

#[test]
fn deterministic_proof_bytes() {
    deterministic::<TensorRs>(&params(2), 10, 5);
}

#[test]
fn security_meets_target_at_every_size() {
    for r in [1u8, 2, 4] {
        for m in 1..=24 {
            let p = params(r);
            let bits = TensorRs::security_bits(&p, m);
            assert!(bits >= 128.0, "r={r} m={m}: {bits}");
        }
    }
}

/// Commit to f, then run the prover on g with f's encoded matrix and
/// tree: the opened columns cannot match g's combinations.
#[test]
fn cheating_prover_commit_f_open_g_rejected() {
    for m in [2, 6, 10] {
        let p = params(2);
        let mut rng = Rng::new(42 + m as u64);
        let f = rng.evals(m);
        let g = rng.evals(m);
        let z = rng.point(m);
        let (cf, df) = TensorRs::commit(&p, &f);
        let forged = TensorRsData {
            config: TensorConfig::derive(&p, m).unwrap(),
            evals: g.clone(),
            encoded: df.encoded,
            tree: df.tree,
        };
        let (v, proof) = TensorRs::open(&p, &forged, &z, &mut Transcript::new(DOMAIN));
        assert_ne!(v, crate::field::ml_eval_base(&f, &z));
        assert!(check::<TensorRs>(&p, &cf, m, &z, v, &proof).is_err());
    }
}

#[cfg(feature = "serde")]
#[test]
fn serde_roundtrip_is_wire_bytes() {
    let r = run::<TensorRs>(&params(2), 8, 9);
    let enc = postcard::to_allocvec(&r.proof).unwrap();
    let dec: crate::TensorRsProof = postcard::from_bytes(&enc).unwrap();
    assert_eq!(dec, r.proof);
}
