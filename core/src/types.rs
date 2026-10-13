//! Core types for polynomial commitment.

// Re-export algebraic trait hierarchy from strata-core
pub use strata_core::{Field, Ring, Semiring};

use cyber_hemera::{Hash, Side};

/// A binding digest of a polynomial — a hemera hash.
///
/// Produced by `Lens::commit`, consumed by `Lens::verify`.
/// The format is identical across all constructions — always a hemera Hash.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Commitment(pub Hash);

impl Commitment {
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

/// A multilinear polynomial over ν variables, defined by its evaluation table.
///
/// The evaluation table has 2^ν entries. Entry at index i corresponds to
/// the boolean assignment (i₁, i₂, ..., i_ν) where i_k = (i >> k) & 1.
#[derive(Clone, Debug)]
pub struct MultilinearPoly<F: Field> {
    pub evals: Vec<F>,
    pub num_vars: usize,
}

impl<F: Field> MultilinearPoly<F> {
    /// Create from an evaluation table. Length must be a power of 2.
    pub fn new(evals: Vec<F>) -> Self {
        let n = evals.len();
        assert!(
            n.is_power_of_two(),
            "evaluation table length must be a power of 2"
        );
        let num_vars = n.trailing_zeros() as usize;
        Self { evals, num_vars }
    }

    /// Number of evaluations (2^num_vars).
    pub fn len(&self) -> usize {
        self.evals.len()
    }

    /// Whether the polynomial is empty (zero variables).
    pub fn is_empty(&self) -> bool {
        self.evals.is_empty()
    }

    /// Evaluate at a point r = (r₁, ..., r_ν) via multilinear extension.
    pub fn evaluate(&self, point: &[F]) -> F {
        assert_eq!(point.len(), self.num_vars);
        let mut result = F::ZERO;

        for (i, &val) in self.evals.iter().enumerate() {
            let mut basis = F::ONE;
            for (j, &r_j) in point.iter().enumerate() {
                let bit = if (i >> j) & 1 == 1 { r_j } else { F::ONE - r_j };
                basis *= bit;
            }
            result += val * basis;
        }
        result
    }
}

/// One column of the tensor-Merkle encoded matrix, opened against the root.
///
/// `index` is the column's position in the encoded matrix (0..m). `column`
/// is the k1 revealed field elements of that column, serialized (little-
/// endian, `Field::byte_len` bytes each — 8 for Goldilocks). `path` is the
/// Merkle authentication path from the column's leaf to the root, in the
/// leaf-to-root order `cyber_hemera::merkle_verify_path` expects.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ColumnQuery {
    pub index: usize,
    pub column: Vec<u8>,
    pub path: Vec<(Hash, Side)>,
}

/// A proof that a committed polynomial evaluates to a claimed value at a point.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Opening {
    /// Porphyry: recursive tensor decomposition with proximity testing via
    /// codeword queries. **Known-unsound residual**: the queried codeword
    /// values are carried but never checked against the round commitments
    /// (a flat hash of a whole codeword cannot authenticate one symbol),
    /// and consecutive round commitments are not tied to each other through
    /// the tensor reduction. See github.com/cyberia-to/lens/issues/6 — the
    /// same defect Brakedown had before it moved to `TensorMerkle`. Kept
    /// only because Porphyry has not been migrated yet.
    Tensor {
        round_commitments: Vec<Commitment>,
        final_poly: Vec<u8>,
        query_responses: Vec<(usize, Vec<u8>)>,
    },
    /// Brakedown (and Ikat, Assayer via delegation): tensor-matrix
    /// commitment via a linear code, Ligero/Brakedown-style (Golovnev,
    /// Lee, Setty, Thaler, Wahby, "Brakedown: Linear-time and post-quantum
    /// SNARKs for R1CS", §5). The evaluation table is reshaped into a
    /// k1×k2 matrix U; each row is encoded by the SAME linear code into an
    /// m-column matrix Û (m = EXPANSION·k2); the `Commitment` is the
    /// Merkle root over Û's columns.
    ///
    /// To open at point r = (r_row, r_col) (r_row = point[..log2 k1],
    /// r_col = point[log2 k1..]):
    /// - `row_combination` = Σ_i eq(r_row, i)·U[i,:], sent in the clear
    ///   (k2 field elements, serialized).
    /// - `columns` — the sampled columns of Û with Merkle paths. The
    ///   verifier checks each one authenticates against the root AND that
    ///   `encode(row_combination)[j] == Σ_i eq(r_row, i)·Û[i,j]` — the
    ///   consistency check that ties the opening to the committed matrix.
    ///   This is the check the old `Tensor` scheme never performed.
    ///
    /// See specs/scalar-field.md for the query-count derivation.
    TensorMerkle {
        row_combination: Vec<u8>,
        columns: Vec<ColumnQuery>,
    },
    /// Binius: folding with Merkle authentication paths.
    Folding {
        round_commitments: Vec<Commitment>,
        merkle_paths: Vec<Vec<Hash>>,
        final_value: Vec<u8>,
    },
    /// Assayer: tropical witness committed via Brakedown + dual certificate.
    Witness {
        witness_commitment: Commitment,
        witness_opening: Box<Opening>,
        certificate: Vec<u8>,
    },
}
