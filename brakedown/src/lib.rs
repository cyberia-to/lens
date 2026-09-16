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
mod public;
mod tensor;

pub use public::PublicTensor;

pub use cyber_lens_core::{Commitment, Field, Lens, MultilinearPoly, Opening, Transcript};
pub use expander::Expander;
pub use tensor::{evaluate_small, tensor_reduce};

use cyber_lens_core::ColumnQuery;
use merkle::ColumnTree;
use nebu::Goldilocks;

/// Current raw and tensor commitment format. Old roots must be recomputed.
pub const COMMITMENT_VERSION: u32 = 2;
const RAW_DOMAIN: &[u8] = b"lens-brakedown-raw-v2";
const TENSOR_DOMAIN: &[u8] = b"lens-brakedown-tensor-v2";
pub const OPENING_VERSION: u32 = 3;
const OPENING_DOMAIN: &[u8] = b"lens-brakedown-opening-v3";
const MAX_VARIABLES: usize = 20;
/// Conditional sampling budget; this is not a global PCS security claim.
const TARGET_SOUNDNESS_BITS: usize = 100;
/// A nonzero systematic codeword contains a nonzero raw symbol.
const MIN_ABS_WEIGHT: usize = 1;

/// Query budget using only the proven absolute weight of one.
/// Malicious-matrix validity and enclosing field bounds remain separate.
fn num_queries(k2: usize) -> usize {
    // (1 - 1/m)^m <= e^-1 < 1/2, so 100*m queries suffice
    // for the conditional 100-bit sampling budget, without floating point.
    TARGET_SOUNDNESS_BITS * expander::EXPANSION * k2 / MIN_ABS_WEIGHT
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
        let mut bytes = RAW_DOMAIN.to_vec();
        bytes.extend_from_slice(&(elements.len() as u64).to_le_bytes());
        bytes.extend_from_slice(&Self::serialize(&codeword));
        let hash = cyber_hemera::hash(&bytes);
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
        if elements.is_empty() {
            return Vec::new();
        }
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
        CommittedMatrix {
            rows,
            columns,
            tree,
        }
    }
}

impl Lens<Goldilocks> for Brakedown {
    fn commit(poly: &MultilinearPoly<Goldilocks>) -> Commitment {
        validate_poly(poly);
        let (row_vars, _) = Self::split_vars(poly.num_vars);
        let matrix = Self::build_matrix(&poly.evals, row_vars);
        tensor_commitment(matrix.tree.root(), poly.num_vars)
    }

    fn open(
        poly: &MultilinearPoly<Goldilocks>,
        point: &[Goldilocks],
        transcript: &mut Transcript,
    ) -> Opening {
        validate_poly(poly);
        assert_eq!(point.len(), poly.num_vars, "point dimension mismatch");

        let (row_vars, _col_vars) = Self::split_vars(poly.num_vars);
        let matrix = Self::build_matrix(&poly.evals, row_vars);
        let commitment = tensor_commitment(matrix.tree.root(), poly.num_vars);

        let (r_row, _r_col) = point.split_at(row_vars);
        let weights = matrix::eq_table(r_row);
        let v = matrix::row_combination(&matrix.rows, &weights);
        let row_combination_bytes = Self::serialize(&v);

        absorb_claim(transcript, &commitment, point, poly.evaluate(point));
        transcript.absorb(&row_combination_bytes);

        let k2 = v.len();
        let columns = query_indices(transcript, k2)
            .into_iter()
            .map(|idx| ColumnQuery {
                index: idx,
                column: Self::serialize(&matrix.columns[idx]),
                path: matrix.tree.path(idx),
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

        if point.len() > MAX_VARIABLES {
            return false;
        }
        let (row_vars, col_vars) = Self::split_vars(point.len());
        let k1 = 1usize << row_vars;
        let k2 = 1usize << col_vars;

        let Some(v) = deserialize_goldilocks(row_combination, k2) else {
            return false;
        };

        absorb_claim(transcript, commitment, point, value);
        transcript.absorb(row_combination);

        let m = expander::EXPANSION * k2;
        if columns.len() > m || columns.windows(2).any(|w| w[0].index >= w[1].index) {
            return false;
        }
        let expected = query_indices(transcript, k2);
        if !columns.iter().map(|c| c.index).eq(expected) {
            return false;
        }

        let encoded_v = Self::encode(&v);
        let (r_row, r_col) = point.split_at(row_vars);
        let weights = matrix::eq_table(r_row);

        for cq in columns {
            // Validate the bounded canonical payload before hashing attacker
            // supplied bytes. Dimensions alone must bound verification work.
            let Some(col_vals) = deserialize_goldilocks(&cq.column, k1) else {
                return false;
            };
            let Some(root) = merkle::column_root(cq.index, m, &cq.column, &cq.path) else {
                return false;
            };
            if tensor_commitment(root, point.len()) != *commitment {
                return false;
            };
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
        assert_eq!(
            points.len(),
            1,
            "Brakedown supports only single-point batches; use PublicTensor for exact public batches"
        );
        Self::open(poly, &points[0].0, transcript)
    }

    fn batch_verify(
        commitment: &Commitment,
        points: &[(Vec<Goldilocks>, Goldilocks)],
        proof: &Opening,
        transcript: &mut Transcript,
    ) -> bool {
        points.len() == 1 && Self::verify(commitment, &points[0].0, points[0].1, proof, transcript)
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

fn deserialize_goldilocks(bytes: &[u8], elements: usize) -> Option<Vec<Goldilocks>> {
    if bytes.len() != elements.checked_mul(8)? {
        return None;
    }
    bytes
        .chunks_exact(8)
        .map(|chunk| {
            let value = u64::from_le_bytes(chunk.try_into().ok()?);
            (value < nebu::field::P).then(|| Goldilocks::new(value))
        })
        .collect()
}

fn validate_poly(poly: &MultilinearPoly<Goldilocks>) {
    assert!(poly.num_vars <= MAX_VARIABLES, "Brakedown variable limit");
    assert_eq!(
        poly.evals.len(),
        1 << poly.num_vars,
        "invalid polynomial dimensions"
    );
}

fn tensor_commitment(root: cyber_hemera::Hash, variables: usize) -> Commitment {
    let mut bytes = TENSOR_DOMAIN.to_vec();
    bytes.extend_from_slice(&(variables as u64).to_le_bytes());
    bytes.extend_from_slice(root.as_bytes());
    Commitment(cyber_hemera::hash(&bytes))
}

fn absorb_claim(t: &mut Transcript, c: &Commitment, point: &[Goldilocks], value: Goldilocks) {
    t.absorb(OPENING_DOMAIN);
    t.absorb(c.as_bytes());
    t.absorb(&(point.len() as u64).to_le_bytes());
    t.absorb(&Brakedown::serialize(point));
    t.absorb(&value.as_u64().to_le_bytes());
}

/// Repeated samples retain their transcript position but share one authenticated
/// payload. Sorted unique indices make the wire canonical and bounded by m.
fn query_indices(t: &mut Transcript, k2: usize) -> std::collections::BTreeSet<usize> {
    let m = expander::EXPANSION * k2;
    (0..num_queries(k2))
        .map(|_| {
            let challenge = t.squeeze();
            (u64::from_le_bytes(
                challenge.as_bytes()[..8]
                    .try_into()
                    .expect("eight-byte digest prefix"),
            ) % m as u64) as usize
        })
        .collect()
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod version_tests;
