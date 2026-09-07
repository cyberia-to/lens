//! Serde round-trip and wire-format stability for Commitment/Opening.
//!
//! The golden fixture pins the JSON wire format. If serialization ever
//! changes shape, `golden_fixture_is_stable` fails — the format must not
//! drift silently.
#![cfg(feature = "serde")]

use cyber_lens_core::{Commitment, Opening};

fn sample_commitment(byte: u8) -> Commitment {
    Commitment(cyber_hemera::Hash::from([byte; 32]))
}

/// One Opening exercising every variant (Witness wraps Tensor; Folding standalone).
fn sample_openings() -> (Opening, Opening, Opening) {
    let tensor = Opening::Tensor {
        round_commitments: vec![sample_commitment(1), sample_commitment(2)],
        final_poly: vec![3, 4, 5],
        query_responses: vec![(0, vec![6, 7]), (9, vec![8])],
    };
    let folding = Opening::Folding {
        round_commitments: vec![sample_commitment(10)],
        merkle_paths: vec![vec![cyber_hemera::Hash::from([11; 32])]],
        final_value: vec![12, 13],
    };
    let witness = Opening::Witness {
        witness_commitment: sample_commitment(20),
        witness_opening: Box::new(tensor.clone()),
        certificate: vec![21, 22, 23],
    };
    (tensor, folding, witness)
}

#[test]
fn commitment_roundtrip() {
    let c = sample_commitment(42);
    let json = serde_json::to_string(&c).unwrap();
    let back: Commitment = serde_json::from_str(&json).unwrap();
    assert_eq!(c, back);
}

#[test]
fn opening_roundtrip_all_variants() {
    let (tensor, folding, witness) = sample_openings();
    for op in [tensor, folding, witness] {
        let json = serde_json::to_string(&op).unwrap();
        let back: Opening = serde_json::from_str(&json).unwrap();
        assert_eq!(op, back);
    }
}

#[test]
fn golden_fixture_is_stable() {
    let (tensor, folding, witness) = sample_openings();
    let value = serde_json::json!({
        "commitment": sample_commitment(42),
        "tensor": tensor,
        "folding": folding,
        "witness": witness,
    });
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/opening_golden.json");
    if std::env::var_os("LENS_BLESS").is_some() {
        std::fs::write(path, serde_json::to_string_pretty(&value).unwrap()).unwrap();
    }
    let golden: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(
        value, golden,
        "Opening/Commitment wire format drifted from the committed fixture"
    );
    // and the fixture still deserializes into live types
    let _: Opening = serde_json::from_value(golden["tensor"].clone()).unwrap();
    let _: Opening = serde_json::from_value(golden["folding"].clone()).unwrap();
    let _: Opening = serde_json::from_value(golden["witness"].clone()).unwrap();
    let _: Commitment = serde_json::from_value(golden["commitment"].clone()).unwrap();
}
