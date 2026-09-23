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

fn random_point(num_vars: usize, seed: u64) -> Vec<Goldilocks> {
    (0..num_vars)
        .map(|i| Goldilocks::new(seed.wrapping_mul(31).wrapping_add(i as u64) + 1).canonicalize())
        .collect()
}

// ── positive: commit/open/verify round trips at several sizes ──────

#[test]
fn commit_deterministic() {
    let poly = random_poly(4, 42);
    assert_eq!(Brakedown::commit(&poly), Brakedown::commit(&poly));
}

#[test]
fn commit_different_polys_differ() {
    let p1 = random_poly(4, 1);
    let p2 = random_poly(4, 2);
    assert_ne!(Brakedown::commit(&p1), Brakedown::commit(&p2));
}

fn roundtrip_at(num_vars: usize, seed: u64) {
    let poly = random_poly(num_vars, seed);
    let commitment = Brakedown::commit(&poly);
    let point = random_point(num_vars, seed + 1000);
    let value = poly.evaluate(&point);

    let mut pt = Transcript::new(b"tensor-merkle-test");
    let proof = Brakedown::open(&poly, &point, &mut pt);
    let mut vt = Transcript::new(b"tensor-merkle-test");
    assert!(
        Brakedown::verify(&commitment, &point, value, &proof, &mut vt),
        "roundtrip failed at num_vars={num_vars}"
    );
}

#[test]
fn roundtrip_4_evals() {
    roundtrip_at(2, 10);
}

#[test]
fn roundtrip_16_evals() {
    roundtrip_at(4, 20);
}

#[test]
fn roundtrip_64_evals() {
    roundtrip_at(6, 30);
}

#[test]
fn roundtrip_256_evals() {
    roundtrip_at(8, 40);
}

#[test]
fn evaluation_matches_reference_formula() {
    // Independent check: MultilinearPoly::evaluate is the reference;
    // the value fed to open/verify above must equal it exactly (not
    // just "verify accepts" — accepting a WRONG reference would be a
    // silent bug in the test, not the code under test).
    let poly = random_poly(6, 99);
    let point = random_point(6, 55);
    let by_formula = poly.evaluate(&point);

    // Recompute via the row/col split independently, as a second
    // reference path distinct from MultilinearPoly::evaluate.
    let (row_vars, _) = Brakedown::split_vars(6);
    let rows = matrix::reshape(&poly.evals, row_vars);
    let (r_row, r_col) = point.split_at(row_vars);
    let weights = matrix::eq_table(r_row);
    let v = matrix::row_combination(&rows, &weights);
    let by_tensor = evaluate_small(&v, r_col);

    assert_eq!(by_formula, by_tensor);
}

// ── negative 1: corrupt a queried column's leaf value post-hoc ─────

#[test]
fn corrupted_queried_column_rejected() {
    let poly = random_poly(6, 7);
    let commitment = Brakedown::commit(&poly);
    let point = random_point(6, 8);
    let value = poly.evaluate(&point);

    let mut pt = Transcript::new(b"corrupt-column");
    let mut proof = Brakedown::open(&poly, &point, &mut pt);

    let Opening::TensorMerkle { columns, .. } = &mut proof else {
        panic!("expected TensorMerkle opening");
    };
    assert!(!columns.is_empty());
    // Flip one byte of the first query's revealed column — this is
    // exactly what zheng's bit-flip scan found the OLD scheme never
    // checked (query values were carried but never authenticated).
    columns[0].column[0] ^= 0x01;

    let mut vt = Transcript::new(b"corrupt-column");
    assert!(!Brakedown::verify(
        &commitment,
        &point,
        value,
        &proof,
        &mut vt
    ));
}

// ── negative 2: corrupt one entry of the row-combination v ─────────

#[test]
fn corrupted_row_combination_rejected() {
    let poly = random_poly(6, 11);
    let commitment = Brakedown::commit(&poly);
    let point = random_point(6, 12);
    let value = poly.evaluate(&point);

    let mut pt = Transcript::new(b"corrupt-v");
    let mut proof = Brakedown::open(&poly, &point, &mut pt);

    let Opening::TensorMerkle {
        row_combination, ..
    } = &mut proof
    else {
        panic!("expected TensorMerkle opening");
    };
    row_combination[0] ^= 0x01;

    let mut vt = Transcript::new(b"corrupt-v");
    assert!(!Brakedown::verify(
        &commitment,
        &point,
        value,
        &proof,
        &mut vt
    ));
}

// ── negative 3: splice an opening from a different polynomial ──────

#[test]
fn spliced_opening_from_different_poly_rejected() {
    let poly_a = random_poly(6, 21);
    let poly_b = random_poly(6, 22);
    let commitment_a = Brakedown::commit(&poly_a);
    let point = random_point(6, 23);
    let value_a = poly_a.evaluate(&point);

    let mut pt = Transcript::new(b"splice");
    // Opening PRODUCED for poly_b...
    let proof_b = Brakedown::open(&poly_b, &point, &mut pt);

    // ...presented against poly_a's commitment and claimed value.
    let mut vt = Transcript::new(b"splice");
    assert!(!Brakedown::verify(
        &commitment_a,
        &point,
        value_a,
        &proof_b,
        &mut vt
    ));
}

// ── negative 4: wrong claimed evaluation, everything else honest ───

#[test]
fn wrong_value_rejected() {
    let poly = random_poly(6, 31);
    let commitment = Brakedown::commit(&poly);
    let point = random_point(6, 32);
    let value = poly.evaluate(&point);

    let mut pt = Transcript::new(b"wrong-value");
    let proof = Brakedown::open(&poly, &point, &mut pt);
    let mut vt = Transcript::new(b"wrong-value");
    assert!(!Brakedown::verify(
        &commitment,
        &point,
        value + Goldilocks::ONE,
        &proof,
        &mut vt
    ));
}

#[test]
fn wrong_commitment_rejected() {
    let poly = random_poly(4, 50);
    let point = random_point(4, 51);
    let value = poly.evaluate(&point);
    let fake = Commitment(cyber_hemera::hash(b"fake"));

    let mut pt = Transcript::new(b"wrong-commitment");
    let proof = Brakedown::open(&poly, &point, &mut pt);
    let mut vt = Transcript::new(b"wrong-commitment");
    assert!(!Brakedown::verify(&fake, &point, value, &proof, &mut vt));
}

// ── negative 5: exhaustive bit-flip scan over the serialized opening ─
//
// Needs postcard + Opening's serde impl, both gated behind this crate's
// `serde` feature — `cargo test --features serde` (or the workspace's
// `cyber-lens` facade crate, which always enables it) to run this one.

#[test]
#[cfg(feature = "serde")]
fn every_bit_flip_of_the_wire_opening_is_rejected() {
    // The exact discipline zheng PR #14 established: serialize a real
    // opening, flip every single bit of every byte, and confirm
    // verify() rejects every one. This directly re-checks the finding
    // that motivated this rewrite (40% of a real proof's bytes could
    // be corrupted with zero effect on verification).
    //
    // num_vars=2 (not larger) deliberately: NUM_QUERIES scales with k2
    // (see `num_queries`'s doc comment), and each query costs several
    // Poseidon2 permutations to verify (Merkle path + leaf hash) — an
    // exhaustive per-bit scan over every byte of a bigger opening is
    // the same property at a cost unsuited to a unit test.
    let poly = random_poly(2, 200);
    let commitment = Brakedown::commit(&poly);
    let point = random_point(2, 201);
    let value = poly.evaluate(&point);

    let mut pt = Transcript::new(b"bitflip");
    let proof = Brakedown::open(&poly, &point, &mut pt);

    let bytes = postcard_bytes(&proof);
    let mut rejected = 0usize;
    let mut total = 0usize;
    for byte_idx in 0..bytes.len() {
        for bit in 0..8u8 {
            let mut flipped = bytes.clone();
            flipped[byte_idx] ^= 1 << bit;
            total += 1;
            match postcard_opening(&flipped) {
                Some(mutated) => {
                    let mut vt = Transcript::new(b"bitflip");
                    if !Brakedown::verify(&commitment, &point, value, &mutated, &mut vt) {
                        rejected += 1;
                    }
                    // else: this flip produced a proof that still
                    // verifies — a real finding, asserted below.
                }
                None => {
                    // Malformed postcard bytes: not a semantically
                    // valid opening, does not count against soundness.
                    rejected += 1;
                }
            }
        }
    }
    assert_eq!(
        rejected, total,
        "every bit flip of the wire opening must be rejected \
             ({rejected}/{total} were)"
    );
}

#[cfg(feature = "serde")]
fn postcard_bytes(opening: &Opening) -> Vec<u8> {
    postcard::to_allocvec(opening).expect("serialize opening")
}
#[cfg(feature = "serde")]
fn postcard_opening(bytes: &[u8]) -> Option<Opening> {
    postcard::from_bytes(bytes).ok()
}

#[test]
fn single_point_batches_work_and_other_batches_fail_closed() {
    let poly = random_poly(2, 42);
    let point = random_point(2, 100);
    let value = poly.evaluate(&point);
    let commitment = Brakedown::commit(&poly);
    let one = vec![(point.clone(), value)];
    let proof = Brakedown::batch_open(&poly, &one, &mut Transcript::new(b"batch"));
    assert!(Brakedown::batch_verify(
        &commitment,
        &one,
        &proof,
        &mut Transcript::new(b"batch")
    ));
    for points in [
        Vec::new(),
        vec![(point.clone(), value), (point, value + Goldilocks::ONE)],
    ] {
        assert!(!Brakedown::batch_verify(
            &commitment,
            &points,
            &proof,
            &mut Transcript::new(b"batch")
        ));
        assert!(std::panic::catch_unwind(|| Brakedown::batch_open(
            &poly,
            &points,
            &mut Transcript::new(b"batch")
        ))
        .is_err());
    }
}

#[test]
fn multilinear_eq_boolean_identity() {
    let b = vec![Goldilocks::ONE, Goldilocks::ZERO, Goldilocks::ONE];
    assert_eq!(multilinear_eq(&b, &b), Goldilocks::ONE);
}

#[test]
fn multilinear_eq_orthogonal() {
    let a = vec![Goldilocks::ZERO, Goldilocks::ZERO];
    let b = vec![Goldilocks::ONE, Goldilocks::ZERO];
    assert_eq!(multilinear_eq(&a, &b), Goldilocks::ZERO);
}

#[test]
fn num_queries_grows_with_k2() {
    assert!(num_queries(8) < num_queries(1024));
}
