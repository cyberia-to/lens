//! Core types for polynomial commitment.

// Re-export algebraic trait hierarchy from strata-core
pub use strata_core::{Field, Ring, Semiring};

use cyber_hemera::Hash;

/// A binding digest of a polynomial — a hemera hash.
///
/// Produced by `Lens::commit`, consumed by `Lens::verify`.
/// The format is identical across all constructions — always a hemera Hash.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
                basis = basis * bit;
            }
            result = result + val * basis;
        }
        result
    }
}

/// A proof that a committed polynomial evaluates to a claimed value at a point.
#[derive(Clone, Debug)]
pub enum Opening {
    /// Brakedown, Ikat, Porphyry: recursive tensor decomposition
    /// with proximity testing via codeword queries.
    Tensor {
        round_commitments: Vec<Commitment>,
        final_poly: Vec<u8>,
        query_responses: Vec<(usize, Vec<u8>)>,
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
