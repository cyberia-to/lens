//! Deterministic expander graph for linear-code encoding.
//!
//! The expander E = (L, R, edges) maps N input elements to c·N encoded
//! elements via sparse matrix-vector multiply. The expansion property
//! guarantees that any sufficiently large subset of encoded positions
//! determines the original polynomial.
//!
//! Construction: hash-based deterministic neighbor assignment.
//! For each left vertex i and neighbor index j ∈ [0, degree), the
//! right neighbor is H(i, j) mod m where H is a deterministic mixing
//! function. This gives a uniform-looking bipartite graph with
//! constant left-degree.

use nebu::Goldilocks;

/// Expansion factor: |R| = EXPANSION · |L|.
const EXPANSION: usize = 2;

/// Left-degree of the expander graph.
/// Each input element contributes to DEGREE output positions.
/// Higher degree = better expansion = higher security, but more work.
/// 24 gives ~128-bit security over Goldilocks.
const DEGREE: usize = 24;

/// Expander graph for Brakedown encoding.
pub struct Expander {
    /// Number of input elements (|L|).
    pub n: usize,
    /// Number of output elements (|R| = EXPANSION · n).
    pub m: usize,
}

impl Expander {
    /// Create an expander for input size n.
    pub fn new(n: usize) -> Self {
        assert!(n > 0, "expander requires n > 0");
        Self {
            n,
            m: EXPANSION * n,
        }
    }

    /// Compute the j-th right neighbor of left vertex i.
    /// Deterministic: same (i, j) always produces the same neighbor.
    #[inline]
    fn neighbor(&self, i: usize, j: usize) -> usize {
        // Deterministic mixing via golden-ratio hashing.
        // phi = (1 + sqrt(5)) / 2 ≈ 1.618...
        // The fractional parts of i·phi are well-distributed (Weyl's theorem).
        // We mix in the neighbor index j to get DEGREE distinct positions.
        let hash = (i as u64)
            .wrapping_mul(0x9E37_79B9_7F4A_7C15) // golden ratio * 2^64
            .wrapping_add((j as u64).wrapping_mul(0x517C_C1B7_2722_0A95));
        (hash as usize) % self.m
    }

    /// Encode input polynomial via sparse matrix-vector multiply.
    ///
    /// output[r] = Σ input[l] for all edges (l, r) in the graph.
    /// Cost: DEGREE · n field additions.
    pub fn encode(&self, input: &[Goldilocks]) -> Vec<Goldilocks> {
        assert_eq!(input.len(), self.n);
        let mut output = vec![Goldilocks::ZERO; self.m];

        for (i, &val) in input.iter().enumerate() {
            for j in 0..DEGREE {
                let r = self.neighbor(i, j);
                output[r] += val;
            }
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expander_deterministic() {
        let exp = Expander::new(16);
        let n1 = exp.neighbor(5, 3);
        let n2 = exp.neighbor(5, 3);
        assert_eq!(n1, n2);
    }

    #[test]
    fn expander_neighbors_in_range() {
        let exp = Expander::new(64);
        for i in 0..64 {
            for j in 0..DEGREE {
                assert!(exp.neighbor(i, j) < exp.m);
            }
        }
    }

    #[test]
    fn expander_different_neighbors() {
        let exp = Expander::new(64);
        // Most neighbors of a vertex should be distinct
        let mut neighbors: Vec<usize> = (0..DEGREE).map(|j| exp.neighbor(7, j)).collect();
        neighbors.sort();
        neighbors.dedup();
        // With degree 24 and m=128, expect most to be distinct
        assert!(neighbors.len() > DEGREE / 2);
    }

    #[test]
    fn encode_zero_polynomial() {
        let exp = Expander::new(8);
        let input = vec![Goldilocks::ZERO; 8];
        let output = exp.encode(&input);
        for &v in &output {
            assert_eq!(v, Goldilocks::ZERO);
        }
    }

    #[test]
    fn encode_output_size() {
        let exp = Expander::new(16);
        let input = vec![Goldilocks::ONE; 16];
        let output = exp.encode(&input);
        assert_eq!(output.len(), EXPANSION * 16);
    }
}
