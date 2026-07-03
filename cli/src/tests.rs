//! Unit tests for the cli's field-level helpers and the commit → open →
//! verify roundtrip through the real constructions.

use super::*;

fn roundtrip(algo: Algo, data: &[u8]) {
    let poly = poly_from_bytes(data);
    let commitment = commit_with(algo, &poly);
    let point = derive_point(&commitment, poly.num_vars);
    let value = poly.evaluate(&point);

    let mut ot = Transcript::new(algo.domain());
    let opening = open_with(algo, &poly, &point, &mut ot);

    // artifact codec roundtrip
    let bytes = artifact::encode(algo.tag(), &point, value, &opening).unwrap();
    let art = artifact::decode(&bytes).unwrap();
    assert_eq!(art.point, point);
    assert_eq!(art.value, value);

    let mut vt = Transcript::new(algo.domain());
    assert!(verify_with(
        algo,
        &commitment,
        &art.point,
        art.value,
        &art.opening,
        &mut vt
    ));
}

#[test]
fn brakedown_roundtrip_verifies() {
    roundtrip(
        Algo::Brakedown,
        b"the quick brown fox jumps over the lazy dog",
    );
}

#[test]
fn ikat_roundtrip_verifies() {
    roundtrip(Algo::Ikat, b"hello lens");
}

#[test]
fn empty_file_commits_and_verifies() {
    roundtrip(Algo::Brakedown, b"");
}

#[test]
fn wrong_value_rejected() {
    let poly = poly_from_bytes(b"data");
    let commitment = commit_with(Algo::Brakedown, &poly);
    let point = derive_point(&commitment, poly.num_vars);
    let value = poly.evaluate(&point);
    let mut ot = Transcript::new(Algo::Brakedown.domain());
    let opening = open_with(Algo::Brakedown, &poly, &point, &mut ot);
    let mut vt = Transcript::new(Algo::Brakedown.domain());
    assert!(!verify_with(
        Algo::Brakedown,
        &commitment,
        &point,
        value + Goldilocks::ONE,
        &opening,
        &mut vt
    ));
}

#[test]
fn tampered_artifact_magic_rejected() {
    let mut bytes = artifact::encode(
        Algo::Brakedown.tag(),
        &[],
        Goldilocks::new(7),
        &Opening::Tensor {
            round_commitments: vec![],
            final_poly: Goldilocks::new(7).as_u64().to_le_bytes().to_vec(),
            query_responses: vec![],
        },
    )
    .unwrap();
    bytes[0] ^= 0xFF;
    assert!(artifact::decode(&bytes).is_err());
}

#[test]
fn goldilocks_parsing_rejects_noncanonical() {
    assert!(parse_goldilocks("0xffffffffffffffff").is_err());
    assert!(parse_goldilocks("18446744069414584321").is_err()); // p
    assert_eq!(parse_goldilocks("42").unwrap(), Goldilocks::new(42));
}
