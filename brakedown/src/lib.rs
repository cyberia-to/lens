//! cyber-lens-brakedown — Brakedown polynomial commitment.
//!
//! Expander-graph linear codes over Goldilocks (F_p) with Margulis
//! expander, committed as a tensor-matrix Merkle tree with a queried-column
//! consistency check (Ligero/Brakedown-style — Golovnev, Lee, Setty,
//! Thaler, Wahby, "Brakedown: Linear-time and post-quantum SNARKs for
//! R1CS", §5).
//!
//! See specs/scalar-field.md for the full specification and
//! github.com/cyberia-to/lens/issues/6 for the soundness defect this
//! construction replaces (the old per-round "commit fresh codeword, then
//! reduce" scheme never checked queried codeword values against anything).

mod expander;
mod matrix;
mod merkle;
mod tensor;

pub use cyber_lens_core::{Commitment, Field, Lens, MultilinearPoly, Opening, Transcript};
pub use expander::Expander;
pub use tensor::{evaluate_small, tensor_reduce};

use cyber_lens_core::ColumnQuery;
use merkle::ColumnTree;
use nebu::Goldilocks;

/// Target soundness, in bits, for a single Brakedown opening.
///
/// `num_queries` derives the query count from this target and the
/// expander code's empirically measured worst-case relative distance at
/// the matrix's actual column count — see that function's doc comment for
/// why the query count is NOT a fixed constant.
const TARGET_SOUNDNESS_BITS: f64 = 100.0;

/// A conservative floor on the number of nonzero output symbols an
/// adversary can force into an otherwise-plausible codeword, independent
/// of the row length k2.
///
/// `expander::tests::empirical_distance_probe` measures this directly: for
/// k2 in [64, 1024] a simple 2-input attack (one free ratio, cancelling
/// one shared output position between two rows) reliably forces the
/// output down to ~9-10 nonzero symbols out of m = 2·k2 — i.e. the
/// ABSOLUTE weight an adversary can achieve stays roughly constant while
/// m grows, so the RELATIVE distance (and therefore per-query soundness)
/// shrinks as k2 grows. This constant is set below the measured floor
/// (measured ~9, we assume 4) because the probe only searched 1- and
/// 2-input attacks — a real adversary is not restricted to those, and a
/// wider search could plausibly do better (find something with even lower
/// weight). This is an EMPIRICAL bound, not a proof; see the probe test
/// and the residual noted in this crate's PR / specs/scalar-field.md.
const ASSUMED_MIN_ABS_WEIGHT: usize = 4;

/// Number of columns to query for a row length of `k2`.
///
/// Soundness argument: an honest prover's `row_combination` v satisfies
/// `encode(v)[j] == Σ_row eq(r_row,row)·Û[row,j]` for every column j (by
/// linearity of `encode`). A dishonest prover who submits a v not
/// matching the committed matrix's actual rows creates a nonzero "error"
/// codeword `encode(v) - Σ_row eq(r_row,row)·Û[row,:]`; since the columns
/// are sampled independently and uniformly at random via Fiat-Shamir
/// AFTER the matrix is committed (the prover cannot tailor the matrix to
/// a not-yet-known r_row), each sampled column independently has
/// probability ≥ (assumed relative distance) of exposing a nonzero
/// mismatch, causing the consistency check in `verify` to reject. Given
/// `t` independent queries, the escape probability is
/// `(1 - relative_distance)^t`; solving for `t` at the target soundness
/// gives the formula below.
///
/// This scales query count UP as k2 grows, because the assumed distance
/// (ASSUMED_MIN_ABS_WEIGHT / m) shrinks as m = 2·k2 grows — a real
/// limitation of this single-layer Margulis expander code (it is not the
/// recursive, distance-amplifying code construction the Brakedown paper
/// uses to get a k2-INDEPENDENT constant relative distance). For the
/// small matrices zheng's decider actually commits today (k2 in the
/// single digits to low tens — decider.md's n=128 witness gives k2=8),
/// this is a modest number of queries; for very large future circuits
/// (k2 in the thousands) this formula honestly reports that the query
/// count — and therefore proof size — grows with the circuit, which is a
/// known residual, not hidden by a falsely-constant NUM_QUERIES.
fn num_queries(k2: usize) -> usize {
    let m = expander::EXPANSION * k2;
    let abs_weight = ASSUMED_MIN_ABS_WEIGHT.min(m.saturating_sub(1)).max(1);
    let relative_distance = abs_weight as f64 / m as f64;
    let per_query_bits = -(1.0 - relative_distance).log2();
    (TARGET_SOUNDNESS_BITS / per_query_bits).ceil() as usize
}

/// Brakedown polynomial commitment over Goldilocks.
pub struct Brakedown;

/// The reshaped-and-encoded matrix a commitment binds to, kept around so
/// `open` does not redundantly rebuild what `commit` already computed
/// within the same call (both are still recomputed by `open` when called
/// independently — the same trade-off the old round-by-round scheme made).
struct CommittedMatrix {
    /// U: k1 rows of length k2 (the original, unencoded evaluation table).
    rows: Vec<Vec<Goldilocks>>,
    /// Û: m columns of length k1 (the encoded matrix, transposed).
    columns: Vec<Vec<Goldilocks>>,
    tree: ColumnTree,
}

impl Brakedown {
    /// Serialize field elements to bytes for hashing.
    pub fn serialize(elements: &[Goldilocks]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(elements.len() * 8);
        for &e in elements {
            bytes.extend_from_slice(&e.as_u64().to_le_bytes());
        }
        bytes
    }

    /// Commit to raw field elements via Margulis expander encoding + a
    /// flat hemera hash of the whole codeword.
    ///
    /// This is DELIBERATELY not the tensor-Merkle scheme below: it is
    /// used only where a binding-but-never-opened commitment is needed
    /// (e.g. zheng's witness/accumulator commitments in
    /// `folding::fold`/`folding::decide`/`phi::spmv`, which are compared
    /// for equality but never passed to `Lens::open`/`verify`). A flat
    /// hash is a perfectly sound binding commitment for that use — the
    /// problem the old scheme had was using a flat hash for a commitment
    /// that ALSO needed to support authenticated partial openings, which
    /// `commit_raw` never claims to do. Do not feed a `commit_raw` output
    /// to `Brakedown::verify` — the digests are unrelated to `Lens::commit`'s
    /// Merkle-root commitments and would never match.
    pub fn commit_raw(elements: &[Goldilocks]) -> Commitment {
        Self::commit_raw_with_codeword(elements).0
    }

    fn commit_raw_with_codeword(elements: &[Goldilocks]) -> (Commitment, Vec<Goldilocks>) {
        let codeword = Self::encode(elements);
        let hash = cyber_hemera::hash(&Self::serialize(&codeword));
        (Commitment(hash), codeword)
    }

    /// Encode elements via expander graph. Expanders are memoized per
    /// length and thread: construction runs a prime search, and a proof
    /// asks for the same handful of sizes hundreds of times. Every row of
    /// the tensor matrix has the SAME length (k2), so a single opening
    /// builds exactly one `Expander` and reuses it for all k1 rows plus
    /// the verifier's `encode(row_combination)` — unlike the old
    /// round-by-round scheme, which built a fresh, different expander for
    /// every round (no two rounds' codewords were linearly related).
    fn encode(elements: &[Goldilocks]) -> Vec<Goldilocks> {
        use std::cell::RefCell;
        use std::collections::HashMap;
        use std::rc::Rc;
        thread_local! {
            static EXPANDERS: RefCell<HashMap<usize, Rc<Expander>>> =
                RefCell::new(HashMap::new());
        }
        let expander = EXPANDERS.with(|m| {
            m.borrow_mut()
                .entry(elements.len())
                .or_insert_with(|| Rc::new(Expander::new(elements.len())))
                .clone()
        });
        expander.encode(elements)
    }

    /// Split ν variables into (row_vars, col_vars); see `matrix` module.
    fn split_vars(num_vars: usize) -> (usize, usize) {
        matrix::split_vars(num_vars)
    }

    /// Reshape, encode every row, and build the column Merkle tree.
    fn build_matrix(evals: &[Goldilocks], row_vars: usize) -> CommittedMatrix {
        let rows = matrix::reshape(evals, row_vars);
        let encoded_rows: Vec<Vec<Goldilocks>> = rows.iter().map(|r| Self::encode(r)).collect();
        let m = encoded_rows.first().map_or(0, Vec::len);
        let columns: Vec<Vec<Goldilocks>> = (0..m)
            .map(|j| encoded_rows.iter().map(|row| row[j]).collect())
            .collect();
        let leaf_bytes: Vec<Vec<u8>> = columns.iter().map(|c| Self::serialize(c)).collect();
        let tree = ColumnTree::build(&leaf_bytes);
        CommittedMatrix { rows, columns, tree }
    }
}

impl Lens<Goldilocks> for Brakedown {
    fn commit(poly: &MultilinearPoly<Goldilocks>) -> Commitment {
        let (row_vars, _) = Self::split_vars(poly.num_vars);
        let matrix = Self::build_matrix(&poly.evals, row_vars);
        Commitment(matrix.tree.root())
    }

    fn open(
        poly: &MultilinearPoly<Goldilocks>,
        point: &[Goldilocks],
        transcript: &mut Transcript,
    ) -> Opening {
        assert_eq!(point.len(), poly.num_vars, "point dimension mismatch");

        let (row_vars, _col_vars) = Self::split_vars(poly.num_vars);
        let matrix = Self::build_matrix(&poly.evals, row_vars);
        let commitment = Commitment(matrix.tree.root());

        let (r_row, _r_col) = point.split_at(row_vars);
        let weights = matrix::eq_table(r_row);
        let v = matrix::row_combination(&matrix.rows, &weights);
        let row_combination_bytes = Self::serialize(&v);

        transcript.absorb(commitment.as_bytes());
        transcript.absorb(&row_combination_bytes);

        let m = matrix.columns.len();
        let k2 = v.len();
        let queries = num_queries(k2);
        let columns = (0..queries)
            .map(|_| {
                let challenge = transcript.squeeze();
                let idx = (u64::from_le_bytes(challenge.as_bytes()[..8].try_into().unwrap())
                    as usize)
                    % m;
                ColumnQuery {
                    index: idx,
                    column: Self::serialize(&matrix.columns[idx]),
                    path: matrix.tree.path(idx),
                }
            })
            .collect();

        Opening::TensorMerkle {
            row_combination: row_combination_bytes,
            columns,
        }
    }

    fn verify(
        commitment: &Commitment,
        point: &[Goldilocks],
        value: Goldilocks,
        proof: &Opening,
        transcript: &mut Transcript,
    ) -> bool {
        let Opening::TensorMerkle {
            row_combination,
            columns,
        } = proof
        else {
            return false;
        };

        let (row_vars, col_vars) = Self::split_vars(point.len());
        let k1 = 1usize << row_vars;
        let k2 = 1usize << col_vars;

        let v = deserialize_goldilocks(row_combination);
        if v.len() != k2 {
            return false;
        }

        transcript.absorb(commitment.as_bytes());
        transcript.absorb(row_combination);

        let m = expander::EXPANSION * k2;
        let expected_queries = num_queries(k2);
        if columns.len() != expected_queries {
            return false;
        }

        let encoded_v = Self::encode(&v);
        let (r_row, r_col) = point.split_at(row_vars);
        let weights = matrix::eq_table(r_row);

        for cq in columns {
            let challenge = transcript.squeeze();
            let expected_idx =
                (u64::from_le_bytes(challenge.as_bytes()[..8].try_into().unwrap()) as usize) % m;
            if cq.index != expected_idx {
                return false;
            }

            if !merkle::verify_column(&commitment.0, cq.index, m, &cq.column, &cq.path) {
                return false;
            }

            let col_vals = deserialize_goldilocks(&cq.column);
            if col_vals.len() != k1 {
                return false;
            }
            let combo: Goldilocks = weights
                .iter()
                .zip(&col_vals)
                .fold(Goldilocks::ZERO, |acc, (&w, &c)| acc + w * c);
            if encoded_v[cq.index] != combo {
                return false;
            }
        }

        let claimed = evaluate_small(&v, r_col);
        claimed == value
    }

    fn batch_open(
        poly: &MultilinearPoly<Goldilocks>,
        points: &[(Vec<Goldilocks>, Goldilocks)],
        transcript: &mut Transcript,
    ) -> Opening {
        if points.len() <= 1 {
            let (pt, _) = &points[0];
            return Self::open(poly, pt, transcript);
        }

        let num_vars = poly.num_vars;
        let r_star: Vec<Goldilocks> = (0..num_vars).map(|_| transcript.squeeze_field()).collect();
        Self::open(poly, &r_star, transcript)
    }

    fn batch_verify(
        commitment: &Commitment,
        points: &[(Vec<Goldilocks>, Goldilocks)],
        proof: &Opening,
        transcript: &mut Transcript,
    ) -> bool {
        if points.len() <= 1 {
            let (pt, val) = &points[0];
            return Self::verify(commitment, pt, *val, proof, transcript);
        }

        let num_vars = points[0].0.len();
        let r_star: Vec<Goldilocks> = (0..num_vars).map(|_| transcript.squeeze_field()).collect();

        let Opening::TensorMerkle { row_combination, .. } = proof else {
            return false;
        };
        let (_, col_vars) = Self::split_vars(num_vars);
        let k2 = 1usize << col_vars;
        let v = deserialize_goldilocks(row_combination);
        if v.len() != k2 {
            return false;
        }
        let (_, r_col) = r_star.split_at(num_vars - col_vars);
        let claimed = evaluate_small(&v, r_col);

        Self::verify(commitment, &r_star, claimed, proof, transcript)
    }
}

/// Multilinear equality polynomial.
pub fn multilinear_eq(r: &[Goldilocks], x: &[Goldilocks]) -> Goldilocks {
    assert_eq!(r.len(), x.len());
    let mut result = Goldilocks::ONE;
    for (&ri, &xi) in r.iter().zip(x.iter()) {
        result *= ri * xi + (Goldilocks::ONE - ri) * (Goldilocks::ONE - xi);
    }
    result
}

fn deserialize_goldilocks(bytes: &[u8]) -> Vec<Goldilocks> {
    bytes
        .chunks_exact(8)
        .map(|chunk| {
            let mut buf = [0u8; 8];
            buf.copy_from_slice(chunk);
            Goldilocks::new(u64::from_le_bytes(buf))
        })
        .collect()
}

#[cfg(test)]
mod tests {
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
        assert!(!Brakedown::verify(&commitment, &point, value, &proof, &mut vt));
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

        let Opening::TensorMerkle { row_combination, .. } = &mut proof else {
            panic!("expected TensorMerkle opening");
        };
        row_combination[0] ^= 0x01;

        let mut vt = Transcript::new(b"corrupt-v");
        assert!(!Brakedown::verify(&commitment, &point, value, &proof, &mut vt));
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
        assert!(!Brakedown::verify(&commitment_a, &point, value_a, &proof_b, &mut vt));
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
    fn batch_roundtrip() {
        let poly = random_poly(4, 200);
        let commitment = Brakedown::commit(&poly);
        let points: Vec<(Vec<Goldilocks>, Goldilocks)> = (0..3)
            .map(|seed| {
                let pt: Vec<Goldilocks> = (0..4)
                    .map(|i| Goldilocks::new(seed * 10 + i + 1).canonicalize())
                    .collect();
                let val = poly.evaluate(&pt);
                (pt, val)
            })
            .collect();

        let mut pt = Transcript::new(b"batch-test");
        let proof = Brakedown::batch_open(&poly, &points, &mut pt);
        let mut vt = Transcript::new(b"batch-test");
        assert!(Brakedown::batch_verify(
            &commitment,
            &points,
            &proof,
            &mut vt
        ));
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
}
