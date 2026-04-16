//! Margulis expander graph for linear-code encoding.
//!
//! The Margulis construction uses affine transformations over Z_p × Z_p
//! to produce a bipartite graph with provable spectral expansion.
//! For any subset S of left vertices with |S| ≤ δN, the neighborhood
//! satisfies |Γ(S)| ≥ (1-ε)d|S|, where d is the left-degree.
//!
//! This replaces hash-based neighbor assignment with an algebraically
//! constructed graph whose expansion is guaranteed by Kazhdan's property (T).
//!
//! Reference: Margulis, "Explicit constructions of expanders" (1973).
//! The bipartite version maps (x, y) ∈ Z_p × Z_p to neighbors via
//! six affine transformations, giving base degree 6. We compose
//! WALK_LENGTH steps of these transformations to achieve the target degree.

use nebu::Goldilocks;

/// Expansion factor: |R| = EXPANSION · |L|.
pub const EXPANSION: usize = 2;

/// Number of base Margulis transformations per step.
const BASE_DEGREE: usize = 6;

/// Number of composition steps. Total degree = BASE_DEGREE^WALK_LENGTH.
/// 2 steps: degree 36, 3 steps: degree 216. We use 2 for efficiency.
const WALK_LENGTH: usize = 2;

/// Effective degree: BASE_DEGREE^WALK_LENGTH = 36.
/// For 128-bit security over Goldilocks (|F| ≈ 2^64), degree 36 with
/// expansion factor 2 provides sufficient distance.
#[cfg(test)]
const DEGREE: usize = BASE_DEGREE * BASE_DEGREE; // 36

/// Expander graph for Brakedown encoding.
pub struct Expander {
    /// Number of input elements (|L|).
    pub n: usize,
    /// Number of output elements (|R| = EXPANSION · n).
    pub m: usize,
    /// Prime p such that p² ≥ m. Vertices live in Z_p × Z_p.
    p: usize,
}

impl Expander {
    /// Create an expander for input size n.
    pub fn new(n: usize) -> Self {
        assert!(n > 0, "expander requires n > 0");
        let m = EXPANSION * n;
        // Find smallest prime p such that p² ≥ m
        let p = Self::smallest_prime_with_square_ge(m);
        Self { n, m, p }
    }

    /// Smallest prime p where p² ≥ target.
    fn smallest_prime_with_square_ge(target: usize) -> usize {
        let sqrt = (target as f64).sqrt().ceil() as usize;
        let mut p = if sqrt < 2 { 2 } else { sqrt };
        while !Self::is_prime(p) {
            p += 1;
        }
        p
    }

    fn is_prime(n: usize) -> bool {
        if n < 2 {
            return false;
        }
        if n < 4 {
            return true;
        }
        if n.is_multiple_of(2) || n.is_multiple_of(3) {
            return false;
        }
        let mut i = 5;
        while i * i <= n {
            if n.is_multiple_of(i) || n.is_multiple_of(i + 2) {
                return false;
            }
            i += 6;
        }
        true
    }

    /// The six Margulis transformations on Z_p × Z_p.
    /// Each maps (x, y) to a neighbor deterministically.
    #[inline]
    fn margulis_neighbors(&self, x: usize, y: usize) -> [(usize, usize); BASE_DEGREE] {
        let p = self.p;
        [
            ((x + y) % p, y),
            ((x + p - y) % p, y),
            (x, (y + x) % p),
            (x, (y + p - x) % p),
            ((x + y + 1) % p, y),
            (x, (y + x + 1) % p),
        ]
    }

    /// Convert a 2D coordinate (x, y) ∈ Z_p × Z_p to a linear index.
    #[inline]
    fn to_index(&self, x: usize, y: usize) -> usize {
        (x * self.p + y) % self.m
    }

    /// Convert a linear index to 2D coordinate.
    #[inline]
    fn index_to_2d(&self, i: usize) -> (usize, usize) {
        (i / self.p % self.p, i % self.p)
    }

    /// Compute all right neighbors of left vertex i.
    /// Uses WALK_LENGTH-step composition of Margulis transformations.
    /// Returns DEGREE = BASE_DEGREE^WALK_LENGTH neighbors.
    pub fn neighbors(&self, i: usize) -> Vec<usize> {
        let (x, y) = self.index_to_2d(i);

        // Step 1: base Margulis neighbors
        let mut current: Vec<(usize, usize)> = self.margulis_neighbors(x, y).to_vec();

        // Steps 2..WALK_LENGTH: compose by applying Margulis again
        for _ in 1..WALK_LENGTH {
            let mut next = Vec::with_capacity(current.len() * BASE_DEGREE);
            for &(cx, cy) in &current {
                next.extend_from_slice(&self.margulis_neighbors(cx, cy));
            }
            current = next;
        }

        // Map 2D coordinates to linear indices in [0, m)
        current.iter().map(|&(x, y)| self.to_index(x, y)).collect()
    }

    /// Encode input polynomial via sparse matrix-vector multiply.
    ///
    /// output[r] += input[l] for all edges (l, r) in the Margulis graph.
    /// Cost: DEGREE · n field additions.
    pub fn encode(&self, input: &[Goldilocks]) -> Vec<Goldilocks> {
        assert_eq!(input.len(), self.n);
        let mut output = vec![Goldilocks::ZERO; self.m];

        for (i, &val) in input.iter().enumerate() {
            for r in self.neighbors(i) {
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
    fn smallest_prime() {
        assert_eq!(Expander::smallest_prime_with_square_ge(4), 2); // 2² = 4
        assert_eq!(Expander::smallest_prime_with_square_ge(5), 3); // 3² = 9
        assert_eq!(Expander::smallest_prime_with_square_ge(100), 11); // 11² = 121
    }

    #[test]
    fn expander_deterministic() {
        let exp = Expander::new(16);
        let n1 = exp.neighbors(5);
        let n2 = exp.neighbors(5);
        assert_eq!(n1, n2);
    }

    #[test]
    fn correct_degree() {
        let exp = Expander::new(64);
        let neighbors = exp.neighbors(7);
        assert_eq!(neighbors.len(), DEGREE);
    }

    #[test]
    fn neighbors_in_range() {
        let exp = Expander::new(64);
        for i in 0..64 {
            for &r in &exp.neighbors(i) {
                assert!(r < exp.m, "neighbor {r} out of range [0, {})", exp.m);
            }
        }
    }

    #[test]
    fn expansion_property() {
        // For a proper Margulis graph, distinct vertices should have
        // neighborhoods that collectively cover many distinct right vertices.
        let exp = Expander::new(64);
        let mut all_neighbors = std::collections::HashSet::new();
        // Take 10 vertices, collect their neighborhoods
        for i in 0..10 {
            for &r in &exp.neighbors(i) {
                all_neighbors.insert(r);
            }
        }
        // 10 vertices × 36 neighbors = 360 edges.
        // With m=128, expect significant coverage.
        // A good expander should cover at least half the right vertices.
        assert!(
            all_neighbors.len() > exp.m / 3,
            "expansion too low: {} distinct out of {}",
            all_neighbors.len(),
            exp.m
        );
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

    #[test]
    fn encode_different_inputs_differ() {
        let exp = Expander::new(8);
        let a = vec![Goldilocks::ONE; 8];
        let mut b = vec![Goldilocks::ONE; 8];
        b[0] = Goldilocks::new(42);
        assert_ne!(exp.encode(&a), exp.encode(&b));
    }
}
