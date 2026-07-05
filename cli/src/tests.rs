//! Unit tests: the commit → open → verify roundtrip through every
//! construction, the artifact codec, and field parsing.

use genies::Fq;
use kuro::F2_128;
use lens::binius::Binius;
use lens::brakedown::Brakedown;
use lens::ikat::Ikat;
use lens::porphyry::Porphyry;
use lens::{Lens, Transcript};
use nebu::Goldilocks;

use crate::algo::Algo;
use crate::artifact;
use crate::commands::{derive_point, poly_from_bytes};
use crate::field::CliField;

/// Commit, open at a derived point, roundtrip the artifact, and verify.
fn roundtrip<F: CliField + std::fmt::Debug, L: Lens<F>>(algo: Algo, data: &[u8]) {
    let poly = poly_from_bytes::<F>(data);
    let commitment = L::commit(&poly);
    let point = derive_point::<F>(&commitment, poly.num_vars);
    let value = poly.evaluate(&point);

    let mut ot = Transcript::new(algo.domain());
    let opening = L::open(&poly, &point, &mut ot);

    let points = vec![(point.clone(), value)];
    let bytes = artifact::encode::<F>(algo.tag(), poly.num_vars, false, &points, &opening).unwrap();
    let art = artifact::decode::<F>(&bytes).unwrap();
    assert_eq!(art.points.len(), 1);
    assert!(!art.batch);
    assert_eq!(art.points[0].0, point);
    assert_eq!(art.points[0].1, value);

    let (pt, val) = &art.points[0];
    let mut vt = Transcript::new(algo.domain());
    assert!(L::verify(&commitment, pt, *val, &art.opening, &mut vt));
}

#[test]
fn brakedown_roundtrip() {
    roundtrip::<Goldilocks, Brakedown>(Algo::Brakedown, b"the quick brown fox");
}

#[test]
fn ikat_roundtrip() {
    roundtrip::<Goldilocks, Ikat>(Algo::Ikat, b"hello lens");
}

#[test]
fn binius_roundtrip() {
    roundtrip::<F2_128, Binius>(Algo::Binius, b"binary tower field element commitment");
}

#[test]
fn porphyry_roundtrip() {
    roundtrip::<Fq, Porphyry>(Algo::Porphyry, b"isogeny field over a 512-bit prime");
}

#[test]
fn empty_file_roundtrips() {
    roundtrip::<Goldilocks, Brakedown>(Algo::Brakedown, b"");
}

#[test]
fn wrong_value_rejected() {
    let poly = poly_from_bytes::<Goldilocks>(b"data");
    let commitment = Brakedown::commit(&poly);
    let point = derive_point::<Goldilocks>(&commitment, poly.num_vars);
    let value = poly.evaluate(&point);
    let mut ot = Transcript::new(Algo::Brakedown.domain());
    let opening = Brakedown::open(&poly, &point, &mut ot);
    let mut vt = Transcript::new(Algo::Brakedown.domain());
    assert!(!Brakedown::verify(
        &commitment,
        &point,
        value + Goldilocks::ONE,
        &opening,
        &mut vt
    ));
}

#[test]
fn tampered_artifact_magic_rejected() {
    let poly = poly_from_bytes::<Goldilocks>(b"data");
    let commitment = Brakedown::commit(&poly);
    let point = derive_point::<Goldilocks>(&commitment, poly.num_vars);
    let value = poly.evaluate(&point);
    let mut ot = Transcript::new(Algo::Brakedown.domain());
    let opening = Brakedown::open(&poly, &point, &mut ot);
    let mut bytes = artifact::encode::<Goldilocks>(
        Algo::Brakedown.tag(),
        poly.num_vars,
        false,
        &[(point, value)],
        &opening,
    )
    .unwrap();
    bytes[0] ^= 0xFF;
    assert!(artifact::decode::<Goldilocks>(&bytes).is_err());
}

#[test]
fn field_parsing() {
    assert!(Goldilocks::parse("0xffffffffffffffff").is_err()); // ≥ p
    assert_eq!(Goldilocks::parse("42").unwrap(), Goldilocks::new(42));
    assert!(F2_128::parse("0x1").unwrap() == F2_128(1));
    assert!(Fq::parse("0x2a").unwrap() == Fq::from_u64(42));
}
